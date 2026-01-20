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
    services::ContainerService,
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
