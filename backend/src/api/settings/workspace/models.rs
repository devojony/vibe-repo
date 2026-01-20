//! Workspace image management API models
//!
//! Request and response DTOs for workspace image management API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for image information query
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImageInfoResponse {
    /// Whether the image exists
    pub exists: bool,
    /// Image name (e.g., "vibe-repo-workspace:latest")
    pub image_name: String,
    /// Docker image ID (SHA256 hash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    /// Image size in megabytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_mb: Option<f64>,
    /// Image creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Number of workspaces using this image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_use_by_workspaces: Option<usize>,
    /// List of workspace IDs using this image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_ids: Option<Vec<i32>>,
    /// Optional message (e.g., "Image does not exist")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Response for image deletion
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteImageResponse {
    /// Status message
    pub message: String,
    /// Image name that was deleted or attempted to delete
    pub image_name: String,
    /// List of workspace IDs still using the image (if deletion failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_workspace_ids: Option<Vec<i32>>,
    /// Suggestion for resolving the conflict
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Request to rebuild workspace image
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RebuildImageRequest {
    /// Force rebuild even if workspaces are using the image
    #[serde(default)]
    pub force: bool,
}

/// Response for image rebuild operation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RebuildImageResponse {
    /// Status message
    pub message: String,
    /// Image name that was rebuilt
    pub image_name: String,
    /// New Docker image ID
    pub image_id: String,
    /// Time taken to build the image in seconds
    pub build_time_seconds: f64,
    /// New image size in megabytes
    pub size_mb: f64,
    /// List of workspace IDs using the old image (if force=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_workspace_ids: Option<Vec<i32>>,
    /// Suggestion for next steps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Warning message (e.g., when force rebuilding with active workspaces)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}
