//! Provider API routes
//!
//! Route definitions for the RepoProvider API.

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers;

/// Create the provider router
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            get(handlers::list_providers).post(handlers::create_provider),
        )
        .route(
            "/:id",
            get(handlers::get_provider)
                .put(handlers::update_provider)
                .delete(handlers::delete_provider),
        )
        .route("/:id/validate", post(handlers::validate_provider))
        .route("/:id/sync", post(handlers::sync_provider))
}
