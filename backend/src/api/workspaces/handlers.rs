use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    api::workspaces::models::*,
    error::Result,
    services::WorkspaceService,
    state::AppState,
};

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
    let service = WorkspaceService::new(state.db.clone());
    
    let workspace = service.create_workspace(req.repository_id).await?;
    
    Ok((StatusCode::CREATED, Json(workspace.into())))
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
    let service = WorkspaceService::new(state.db.clone());
    
    let workspace = service.get_workspace_by_id(id).await?;
    
    Ok(Json(workspace.into()))
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
    let service = WorkspaceService::new(state.db.clone());
    
    let workspaces = service.list_workspaces().await?;
    
    let responses: Vec<WorkspaceResponse> = workspaces
        .into_iter()
        .map(|w| w.into())
        .collect();
    
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
    let service = WorkspaceService::new(state.db.clone());
    
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
    let service = WorkspaceService::new(state.db.clone());
    
    let workspace = service.soft_delete_workspace(id).await?;
    
    Ok(Json(workspace.into()))
}
