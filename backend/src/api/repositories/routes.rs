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
        .route("/", get(handlers::list_repositories))
        .route("/:id", get(handlers::get_repository))
        .route("/:id", patch(handlers::update_repository))
        .route("/:id", delete(handlers::delete_repository))
        .route("/:id/refresh", post(handlers::refresh_repository))
        .route("/:id/initialize", post(handlers::initialize_repository))
        .route("/:id/reinitialize", post(handlers::reinitialize_repository))
        .route("/:id/archive", post(handlers::archive_repository))
        .route("/:id/unarchive", post(handlers::unarchive_repository))
        .route("/:id/polling", patch(handlers::update_repository_polling))
        .route("/:id/poll-issues", post(handlers::trigger_issue_polling))
        .route(
            "/batch-initialize",
            post(handlers::batch_initialize_repositories),
        )
        .route("/batch-archive", post(handlers::batch_archive_repositories))
        .route("/batch-delete", post(handlers::batch_delete_repositories))
        .route("/batch-refresh", post(handlers::batch_refresh_repositories))
        .route(
            "/batch-reinitialize",
            post(handlers::batch_reinitialize_repositories),
        )
}
