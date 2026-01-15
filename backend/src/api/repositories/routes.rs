//! Repository API routes
//!
//! Route definitions for repository endpoints.

use axum::{
    routing::{get, post},
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
        .route("/:id/refresh", post(handlers::refresh_repository))
        .route("/:id/initialize", post(handlers::initialize_repository))
        .route(
            "/batch-initialize",
            post(handlers::batch_initialize_repositories),
        )
}
