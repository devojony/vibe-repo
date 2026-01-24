//! Workspace lifecycle handlers
//!
//! Provides HTTP handlers for workspace container lifecycle operations.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use sea_orm::EntityTrait;
use std::sync::Arc;

use crate::{
    api::workspaces::models::{
        ContainerInfo, ContainerStatsInfo, RestartWorkspaceResponse, WorkspaceStatsResponse,
    },
    entities::prelude::*,
    error::{Result, VibeRepoError},
    services::{ContainerService, GitService, WorkspaceService},
    state::AppState,
};

/// Restart a workspace container
#[utoipa::path(
    post,
    path = "/api/workspaces/{id}/restart",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Container restarted successfully", body = RestartWorkspaceResponse),
        (status = 404, description = "Workspace or container not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn restart_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<RestartWorkspaceResponse>> {
    tracing::info!(workspace_id = id, "Restarting workspace container");

    // Get workspace by id from database
    let workspace = Workspace::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(VibeRepoError::Database)?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Workspace with id {} not found", id)))?;

    // Get container by workspace_id using ContainerService
    let container_service = ContainerService::new(state.db.clone(), state.docker.clone());
    let container = container_service
        .get_container_by_workspace_id(workspace.id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Container for workspace {} not found",
                workspace.id
            ))
        })?;

    tracing::info!(
        workspace_id = workspace.id,
        container_id = container.id,
        docker_container_id = %container.container_id,
        "Found container, initiating restart"
    );

    // Call container_service.manual_restart_container(container.id)
    let updated_container = container_service
        .manual_restart_container(container.id)
        .await?;

    tracing::info!(
        workspace_id = workspace.id,
        container_id = updated_container.id,
        restart_count = updated_container.restart_count,
        "Container restarted successfully"
    );

    // Return success response with container info
    let response = RestartWorkspaceResponse {
        message: "Container restarted successfully".to_string(),
        workspace_id: workspace.id,
        container: ContainerInfo {
            id: updated_container.id,
            container_id: updated_container.container_id,
            status: updated_container.status,
            restart_count: updated_container.restart_count,
            last_restart_at: updated_container.last_restart_at.map(|dt| dt.to_string()),
        },
    };

    Ok(Json(response))
}

/// Create and start a container for a workspace
#[utoipa::path(
    post,
    path = "/api/workspaces/{id}/container",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Container created successfully", body = RestartWorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 409, description = "Container already exists"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn create_container(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<RestartWorkspaceResponse>> {
    tracing::info!(workspace_id = id, "Creating container for workspace");

    // Get workspace by id from database
    let workspace = Workspace::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(VibeRepoError::Database)?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Workspace with id {} not found", id)))?;

    // Check if container already exists
    let container_service = ContainerService::new(state.db.clone(), state.docker.clone());
    if let Some(existing_container) = container_service
        .get_container_by_workspace_id(workspace.id)
        .await?
    {
        tracing::warn!(
            workspace_id = workspace.id,
            container_id = existing_container.id,
            "Container already exists for workspace"
        );
        return Err(VibeRepoError::Conflict(format!(
            "Container already exists for workspace {}",
            workspace.id
        )));
    }

    // Check if Docker is available
    if state.docker.is_none() {
        return Err(VibeRepoError::ServiceUnavailable(
            "Docker service is not available".to_string(),
        ));
    }

    // Create workspace service and ensure image exists
    let workspace_service = WorkspaceService::new(state.db.clone(), state.docker.clone());
    let image_name = &workspace.image_source;

    tracing::info!(
        workspace_id = workspace.id,
        image_name = %image_name,
        "Ensuring image exists before creating container"
    );

    // This will build the image if it doesn't exist
    workspace_service
        .ensure_image_exists(image_name)
        .await
        .map_err(|e| {
            tracing::error!(
                workspace_id = workspace.id,
                image_name = %image_name,
                error = %e,
                "Failed to ensure image exists"
            );
            e
        })?;

    tracing::info!(
        workspace_id = workspace.id,
        image_name = %image_name,
        "Image ready, creating container"
    );

    // Get workspace directory path for volume mounting
    let git_service = GitService::new(state.db.clone(), state.config.workspace.base_dir.clone());
    let workspace_dir = git_service.get_workspace_dir(workspace.id);

    // Convert to absolute path if it's relative
    let absolute_workspace_dir = if workspace_dir.is_absolute() {
        workspace_dir
    } else {
        std::env::current_dir()
            .map_err(|e| {
                VibeRepoError::Internal(format!("Failed to get current directory: {}", e))
            })?
            .join(&workspace_dir)
    };

    let host_workspace_dir = absolute_workspace_dir.to_str().map(|s| s.to_string());

    tracing::info!(
        workspace_id = workspace.id,
        host_workspace_dir = ?host_workspace_dir,
        "Using absolute workspace directory for container mount"
    );

    // Create and start container
    let container = container_service
        .create_and_start_container(
            workspace.id,
            image_name,
            workspace.cpu_limit,
            &workspace.memory_limit,
            host_workspace_dir,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                workspace_id = workspace.id,
                error = %e,
                "Failed to create and start container"
            );
            e
        })?;

    tracing::info!(
        workspace_id = workspace.id,
        container_id = container.id,
        docker_container_id = %container.container_id,
        "Container created and started successfully"
    );

    // Return success response with container info
    let response = RestartWorkspaceResponse {
        message: "Container created and started successfully".to_string(),
        workspace_id: workspace.id,
        container: ContainerInfo {
            id: container.id,
            container_id: container.container_id,
            status: container.status,
            restart_count: container.restart_count,
            last_restart_at: container.last_restart_at.map(|dt| dt.to_string()),
        },
    };

    Ok(Json(response))
}

/// Get workspace container resource usage statistics
#[utoipa::path(
    get,
    path = "/api/workspaces/{id}/stats",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Container stats retrieved successfully", body = WorkspaceStatsResponse),
        (status = 404, description = "Workspace or container not found"),
        (status = 409, description = "Container is not running"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn get_workspace_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<WorkspaceStatsResponse>> {
    tracing::info!(workspace_id = id, "Getting workspace container stats");

    // Get workspace by id from database
    let workspace = Workspace::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(VibeRepoError::Database)?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Workspace with id {} not found", id)))?;

    // Get container by workspace_id using ContainerService
    let container_service = ContainerService::new(state.db.clone(), state.docker.clone());
    let container = container_service
        .get_container_by_workspace_id(workspace.id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Container for workspace {} not found",
                workspace.id
            ))
        })?;

    // Check if container status is "running"
    if container.status != "running" {
        tracing::warn!(
            workspace_id = workspace.id,
            container_id = container.id,
            status = %container.status,
            "Container is not running, cannot get stats"
        );
        return Err(VibeRepoError::Conflict(format!(
            "Container is not running (status: {})",
            container.status
        )));
    }

    // Get Docker service
    let docker_service = state
        .docker
        .as_ref()
        .ok_or_else(|| VibeRepoError::ServiceUnavailable("Docker not available".to_string()))?;

    tracing::info!(
        workspace_id = workspace.id,
        container_id = container.id,
        docker_container_id = %container.container_id,
        "Fetching container stats from Docker"
    );

    // Call docker_service.get_container_stats(container.container_id)
    let stats = docker_service
        .get_container_stats(&container.container_id)
        .await?;

    let collected_at = Utc::now();

    tracing::info!(
        workspace_id = workspace.id,
        container_id = container.id,
        cpu_percent = stats.cpu_percent,
        memory_usage_mb = stats.memory_usage_mb,
        "Container stats retrieved successfully"
    );

    // Return success response with stats
    let response = WorkspaceStatsResponse {
        workspace_id: workspace.id,
        container_id: container.container_id,
        stats: ContainerStatsInfo {
            cpu_percent: stats.cpu_percent,
            memory_usage_mb: stats.memory_usage_mb,
            memory_limit_mb: stats.memory_limit_mb,
            memory_percent: stats.memory_percent,
            network_rx_bytes: stats.network_rx_bytes,
            network_tx_bytes: stats.network_tx_bytes,
        },
        collected_at: collected_at.to_string(),
    };

    Ok(Json(response))
}
