//! Repository API routes
//!
//! Route definitions for repository endpoints.

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers;

/// Create repository router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(handlers::add_repository))
        .route("/", get(handlers::list_repositories))
        .route("/:id", get(handlers::get_repository))
        .route("/:id", patch(handlers::update_repository))
        .route("/:id", delete(handlers::delete_repository))
        .route("/:id/refresh", post(handlers::refresh_repository))
        .route("/:id/initialize", post(handlers::initialize_repository))
        .route("/:id/reinitialize", post(handlers::reinitialize_repository))
        .route("/:id/archive", post(handlers::archive_repository))
        .route("/:id/unarchive", post(handlers::unarchive_repository))
}
