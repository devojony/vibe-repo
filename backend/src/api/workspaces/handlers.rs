use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    api::workspaces::models::*,
    entities::prelude::*,
    error::Result,
    services::{GitService, InitScriptService, WorkspaceService},
    state::AppState,
};
use sea_orm::EntityTrait;

/// Create a new workspace
#[utoipa::path(
    post,
    path = "/api/workspaces",
    request_body = CreateWorkspaceRequest,
    responses(
        (status = 201, description = "Workspace created successfully", body = WorkspaceResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>)> {
    let workspace_service = WorkspaceService::new(state.db.clone(), state.docker.clone());
    let init_script_service = InitScriptService::new(state.db.clone(), state.docker.clone());
    let git_service = GitService::new(state.db.clone(), state.config.workspace.base_dir.clone());

    // Create workspace
    let workspace = workspace_service
        .create_workspace(req.repository_id)
        .await?;

    // Get repository for cloning
    let repository = Repository::find_by_id(req.repository_id)
        .one(&state.db)
        .await
        .map_err(crate::error::VibeRepoError::Database)?
        .ok_or_else(|| {
            crate::error::VibeRepoError::NotFound(format!(
                "Repository {} not found",
                req.repository_id
            ))
        })?;

    // Clone repository to workspace
    tracing::info!(
        workspace_id = workspace.id,
        repository_id = repository.id,
        "Cloning repository to workspace"
    );

    match git_service.clone_repository(&workspace, &repository).await {
        Ok(()) => {
            tracing::info!(
                workspace_id = workspace.id,
                "Repository cloned successfully"
            );
        }
        Err(e) => {
            tracing::error!(
                workspace_id = workspace.id,
                error = %e,
                "Failed to clone repository"
            );
            // Continue anyway - user can retry later
        }
    }

    // Create init script if provided
    let init_script = if let Some(script_content) = req.init_script {
        let script = init_script_service
            .create_init_script(workspace.id, script_content, req.script_timeout_seconds)
            .await?;

        tracing::info!(
            workspace_id = workspace.id,
            script_id = script.id,
            "Created init script for new workspace"
        );

        Some(script)
    } else {
        None
    };

    let response = WorkspaceResponse::from((workspace, init_script));

    Ok((StatusCode::CREATED, Json(response)))
}

/// Get workspace by ID
#[utoipa::path(
    get,
    path = "/api/workspaces/{id}",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace found", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn get_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<WorkspaceResponse>> {
    let workspace_service = WorkspaceService::new(state.db.clone(), state.docker.clone());
    let init_script_service = InitScriptService::new(state.db.clone(), state.docker.clone());

    // Get workspace
    let workspace = workspace_service.get_workspace_by_id(id).await?;

    // Get init script if exists
    let init_script = init_script_service
        .get_init_script_by_workspace_id(id)
        .await?;

    let response = WorkspaceResponse::from((workspace, init_script));

    Ok(Json(response))
}

/// List all workspaces
#[utoipa::path(
    get,
    path = "/api/workspaces",
    responses(
        (status = 200, description = "List of workspaces", body = Vec<WorkspaceResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn list_workspaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorkspaceResponse>>> {
    let workspace_service = WorkspaceService::new(state.db.clone(), state.docker.clone());
    let init_script_service = InitScriptService::new(state.db.clone(), state.docker.clone());

    // Get all workspaces
    let workspaces = workspace_service.list_workspaces().await?;

    // Build responses with init scripts
    let mut responses = Vec::new();
    for workspace in workspaces {
        let init_script = init_script_service
            .get_init_script_by_workspace_id(workspace.id)
            .await?;

        responses.push(WorkspaceResponse::from((workspace, init_script)));
    }

    Ok(Json(responses))
}

/// Update workspace status
#[utoipa::path(
    patch,
    path = "/api/workspaces/{id}/status",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    request_body = UpdateWorkspaceStatusRequest,
    responses(
        (status = 200, description = "Workspace status updated", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn update_workspace_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWorkspaceStatusRequest>,
) -> Result<Json<WorkspaceResponse>> {
    let service = WorkspaceService::new(state.db.clone(), state.docker.clone());

    let workspace = service.update_workspace_status(id, &req.status).await?;

    Ok(Json(workspace.into()))
}

/// Soft delete workspace
#[utoipa::path(
    delete,
    path = "/api/workspaces/{id}",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace deleted", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<WorkspaceResponse>> {
    let service = WorkspaceService::new(state.db.clone(), state.docker.clone());

    let workspace = service.soft_delete_workspace(id).await?;

    Ok(Json(workspace.into()))
}
