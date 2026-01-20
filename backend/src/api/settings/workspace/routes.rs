//! Workspace API routes
//!
//! Route definitions for workspace image management endpoints.

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers;

/// Create workspace routes
pub fn workspace_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/image", get(handlers::get_image_info))
        .route("/image", delete(handlers::delete_image))
        .route("/image/rebuild", post(handlers::rebuild_image))
}
