//! Workspace image management API handlers
//!
//! HTTP request handlers for workspace Docker image operations.

use crate::{
    error::{Result, VibeRepoError},
    services::ImageManagementService,
    state::AppState,
};
use axum::{extract::State, Json};
use std::sync::Arc;

use super::models::{
    DeleteImageResponse, ImageInfoResponse, RebuildImageRequest, RebuildImageResponse,
};

/// Default workspace image name
const DEFAULT_IMAGE_NAME: &str = "vibe-repo-workspace:latest";

/// Get workspace image information
///
/// Returns information about the workspace Docker image including size,
/// creation time, and which workspaces are using it.
#[utoipa::path(
    get,
    path = "/api/settings/workspace/image",
    responses(
        (status = 200, description = "Image information", body = ImageInfoResponse)
    ),
    tag = "workspace"
)]
pub async fn get_image_info(State(state): State<Arc<AppState>>) -> Result<Json<ImageInfoResponse>> {
    let image_service = ImageManagementService::new(state.db.clone(), state.docker.clone());

    // Get image info
    let image_info = image_service.get_image_info(DEFAULT_IMAGE_NAME).await?;

    match image_info {
        Some(info) => {
            // Image exists, get workspaces using it
            let workspace_ids = image_service
                .get_workspaces_using_image(DEFAULT_IMAGE_NAME)
                .await?;

            let size_mb = info.size_bytes as f64 / (1024.0 * 1024.0);

            Ok(Json(ImageInfoResponse {
                exists: true,
                image_name: DEFAULT_IMAGE_NAME.to_string(),
                image_id: Some(info.id),
                size_mb: Some(size_mb),
                created_at: Some(info.created_at),
                in_use_by_workspaces: Some(workspace_ids.len()),
                workspace_ids: Some(workspace_ids),
                message: None,
            }))
        }
        None => {
            // Image doesn't exist
            Ok(Json(ImageInfoResponse {
                exists: false,
                image_name: DEFAULT_IMAGE_NAME.to_string(),
                image_id: None,
                size_mb: None,
                created_at: None,
                in_use_by_workspaces: None,
                workspace_ids: None,
                message: Some(format!(
                    "Image '{}' does not exist. Use POST /api/settings/workspace/image/rebuild to build it.",
                    DEFAULT_IMAGE_NAME
                )),
            }))
        }
    }
}

/// Delete workspace image
///
/// Deletes the workspace Docker image. Returns error if any workspaces
/// are currently using the image.
#[utoipa::path(
    delete,
    path = "/api/settings/workspace/image",
    responses(
        (status = 200, description = "Image deleted successfully", body = DeleteImageResponse),
        (status = 409, description = "Image is in use by workspaces", body = DeleteImageResponse)
    ),
    tag = "workspace"
)]
pub async fn delete_image(State(state): State<Arc<AppState>>) -> Result<Json<DeleteImageResponse>> {
    let image_service = ImageManagementService::new(state.db.clone(), state.docker.clone());

    // Attempt to delete the image
    match image_service.delete_image(DEFAULT_IMAGE_NAME).await {
        Ok(()) => Ok(Json(DeleteImageResponse {
            message: format!("Image '{}' deleted successfully", DEFAULT_IMAGE_NAME),
            image_name: DEFAULT_IMAGE_NAME.to_string(),
            active_workspace_ids: None,
            suggestion: None,
        })),
        Err(VibeRepoError::Conflict(msg)) => {
            // Extract workspace IDs from error message
            // Error format: "Cannot delete image: N workspace(s) are using it"
            let workspace_ids = image_service
                .get_workspaces_using_image(DEFAULT_IMAGE_NAME)
                .await
                .unwrap_or_default();

            Err(VibeRepoError::Conflict(format!(
                "{}. Active workspace IDs: {:?}. Stop or delete these workspaces first.",
                msg, workspace_ids
            )))
        }
        Err(VibeRepoError::NotFound(_)) => {
            // Image doesn't exist - treat as success
            Ok(Json(DeleteImageResponse {
                message: format!("Image '{}' does not exist", DEFAULT_IMAGE_NAME),
                image_name: DEFAULT_IMAGE_NAME.to_string(),
                active_workspace_ids: None,
                suggestion: None,
            }))
        }
        Err(e) => Err(e),
    }
}

/// Rebuild workspace image
///
/// Rebuilds the workspace Docker image from the Dockerfile. If `force` is true,
/// rebuilds even if workspaces are using the current image (they will need to
/// be restarted to use the new image).
#[utoipa::path(
    post,
    path = "/api/settings/workspace/image/rebuild",
    request_body = RebuildImageRequest,
    responses(
        (status = 200, description = "Image rebuilt successfully", body = RebuildImageResponse),
        (status = 409, description = "Image is in use and force=false", body = RebuildImageResponse)
    ),
    tag = "workspace"
)]
pub async fn rebuild_image(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RebuildImageRequest>,
) -> Result<Json<RebuildImageResponse>> {
    let image_service = ImageManagementService::new(state.db.clone(), state.docker.clone());

    // Attempt to rebuild the image
    match image_service
        .rebuild_image(DEFAULT_IMAGE_NAME, request.force)
        .await
    {
        Ok(build_result) => {
            let size_mb = build_result.size_bytes as f64 / (1024.0 * 1024.0);

            // Check if any workspaces were using the old image
            let workspace_ids = image_service
                .get_workspaces_using_image(DEFAULT_IMAGE_NAME)
                .await
                .unwrap_or_default();

            let (warning, suggestion) = if !workspace_ids.is_empty() && request.force {
                (
                    Some(format!(
                        "{} workspace(s) are using the old image and may need to be restarted",
                        workspace_ids.len()
                    )),
                    Some(format!(
                        "Restart workspaces {:?} to use the new image",
                        workspace_ids
                    )),
                )
            } else {
                (None, None)
            };

            Ok(Json(RebuildImageResponse {
                message: format!("Image '{}' rebuilt successfully", DEFAULT_IMAGE_NAME),
                image_name: DEFAULT_IMAGE_NAME.to_string(),
                image_id: build_result.image_id,
                build_time_seconds: build_result.build_time_seconds,
                size_mb,
                active_workspace_ids: if !workspace_ids.is_empty() {
                    Some(workspace_ids)
                } else {
                    None
                },
                suggestion,
                warning,
            }))
        }
        Err(VibeRepoError::Conflict(msg)) => {
            // Get workspace IDs for helpful error message
            let workspace_ids = image_service
                .get_workspaces_using_image(DEFAULT_IMAGE_NAME)
                .await
                .unwrap_or_default();

            Err(VibeRepoError::Conflict(format!(
                "{}. Active workspace IDs: {:?}. Use force=true to rebuild anyway, or stop these workspaces first.",
                msg, workspace_ids
            )))
        }
        Err(e) => Err(e),
    }
}
