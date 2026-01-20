use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    api::workspaces::models::*,
    error::{Result, VibeRepoError},
    services::{InitScriptService, WorkspaceService},
    state::AppState,
};

/// Update init script for a workspace
#[utoipa::path(
    put,
    path = "/api/workspaces/{id}/init-script",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    request_body = UpdateInitScriptRequest,
    responses(
        (status = 200, description = "Init script updated successfully", body = InitScriptResponse),
        (status = 201, description = "Init script created successfully", body = InitScriptResponse),
        (status = 404, description = "Workspace not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "init-scripts"
)]
pub async fn update_init_script(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<i32>,
    Json(req): Json<UpdateInitScriptRequest>,
) -> Result<impl IntoResponse> {
    let init_script_service = InitScriptService::new(state.db.clone(), state.docker.clone());
    let workspace_service = WorkspaceService::new(state.db.clone(), state.docker.clone());

    // Verify workspace exists
    workspace_service.get_workspace_by_id(workspace_id).await?;

    // Check if init script already exists
    let existing_script = init_script_service
        .get_init_script_by_workspace_id(workspace_id)
        .await?;

    let script = match existing_script {
        Some(_) => {
            // Update existing script
            let script = init_script_service
                .update_init_script(
                    workspace_id,
                    req.script_content.clone(),
                    req.timeout_seconds,
                )
                .await?;

            tracing::info!(
                workspace_id = workspace_id,
                script_id = script.id,
                "Updated init script"
            );

            (StatusCode::OK, Json(InitScriptResponse::from(script)))
        }
        None => {
            // Create new script
            let script = init_script_service
                .create_init_script(
                    workspace_id,
                    req.script_content.clone(),
                    req.timeout_seconds,
                )
                .await?;

            tracing::info!(
                workspace_id = workspace_id,
                script_id = script.id,
                "Created init script"
            );

            (StatusCode::CREATED, Json(InitScriptResponse::from(script)))
        }
    };

    // Execute immediately if requested
    if req.execute_immediately {
        let workspace = workspace_service.get_workspace_by_id(workspace_id).await?;

        if let Some(container_id) = workspace.container_id {
            tracing::info!(
                workspace_id = workspace_id,
                container_id = %container_id,
                "Executing init script immediately"
            );

            // Execute in background - don't wait for completion
            let init_script_service_clone = init_script_service.clone();
            let container_id_clone = container_id.clone();
            tokio::spawn(async move {
                if let Err(e) = init_script_service_clone
                    .execute_script(workspace_id, &container_id_clone)
                    .await
                {
                    tracing::error!(
                        workspace_id = workspace_id,
                        error = %e,
                        "Failed to execute init script"
                    );
                }
            });
        } else {
            tracing::warn!(
                workspace_id = workspace_id,
                "Cannot execute init script: workspace has no container"
            );
        }
    }

    Ok(script)
}

/// Get init script logs for a workspace
#[utoipa::path(
    get,
    path = "/api/workspaces/{id}/init-script/logs",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Init script logs retrieved", body = InitScriptLogsResponse),
        (status = 404, description = "Workspace or init script not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "init-scripts"
)]
pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<i32>,
) -> Result<Json<InitScriptLogsResponse>> {
    let init_script_service = InitScriptService::new(state.db.clone(), state.docker.clone());

    let script = init_script_service
        .get_init_script_by_workspace_id(workspace_id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Init script for workspace {} not found",
                workspace_id
            ))
        })?;

    let response = InitScriptLogsResponse {
        status: script.status,
        output_summary: script.output_summary,
        has_full_log: script.output_file_path.is_some(),
        executed_at: script.executed_at.map(|dt| dt.to_string()),
    };

    Ok(Json(response))
}

/// Download full log file for init script
#[utoipa::path(
    get,
    path = "/api/workspaces/{id}/init-script/logs/full",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Full log file", content_type = "text/plain"),
        (status = 404, description = "Workspace, init script, or log file not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "init-scripts"
)]
pub async fn download_full_log(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<i32>,
) -> Result<impl IntoResponse> {
    let init_script_service = InitScriptService::new(state.db.clone(), state.docker.clone());

    let script = init_script_service
        .get_init_script_by_workspace_id(workspace_id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Init script for workspace {} not found",
                workspace_id
            ))
        })?;

    let file_path = script.output_file_path.ok_or_else(|| {
        VibeRepoError::NotFound(format!(
            "Full log file for workspace {} not found",
            workspace_id
        ))
    })?;

    // Read file contents
    let contents = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
        tracing::error!(
            workspace_id = workspace_id,
            file_path = %file_path,
            error = %e,
            "Failed to read log file"
        );
        VibeRepoError::Internal(format!("Failed to read log file: {}", e))
    })?;

    Ok((StatusCode::OK, [("Content-Type", "text/plain")], contents))
}

/// Execute init script for a workspace
#[utoipa::path(
    post,
    path = "/api/workspaces/{id}/init-script/execute",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    request_body = ExecuteScriptRequest,
    responses(
        (status = 202, description = "Init script execution started", body = InitScriptResponse),
        (status = 404, description = "Workspace or init script not found"),
        (status = 409, description = "Script is already running"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "init-scripts"
)]
pub async fn execute_script(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<i32>,
    Json(req): Json<ExecuteScriptRequest>,
) -> Result<(StatusCode, Json<InitScriptResponse>)> {
    let init_script_service = InitScriptService::new(state.db.clone(), state.docker.clone());
    let workspace_service = WorkspaceService::new(state.db.clone(), state.docker.clone());

    // Get workspace and verify it has a container
    let workspace = workspace_service.get_workspace_by_id(workspace_id).await?;

    let container_id = workspace.container_id.ok_or_else(|| {
        VibeRepoError::Validation(format!(
            "Workspace {} has no container to execute script in",
            workspace_id
        ))
    })?;

    // Verify init script exists
    let script = init_script_service
        .get_init_script_by_workspace_id(workspace_id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Init script for workspace {} not found",
                workspace_id
            ))
        })?;

    // Check if already running (unless force is true)
    if !req.force && script.status == "Running" {
        tracing::warn!(
            workspace_id = workspace_id,
            script_id = script.id,
            "Rejected execution: script is already running"
        );
        return Err(VibeRepoError::Conflict(
            "Script is already running. Use force=true to override.".to_string(),
        ));
    }

    tracing::info!(
        workspace_id = workspace_id,
        script_id = script.id,
        container_id = %container_id,
        force = req.force,
        "Starting init script execution"
    );

    // Execute script (this will handle concurrency control internally)
    let script = init_script_service
        .execute_script(workspace_id, &container_id)
        .await?;

    Ok((StatusCode::ACCEPTED, Json(InitScriptResponse::from(script))))
}
