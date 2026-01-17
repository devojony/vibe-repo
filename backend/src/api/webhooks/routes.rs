//! Webhook routes

use axum::{routing::post, Router};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers;

/// Create webhook router
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/:repository_id", post(handlers::handle_webhook))
}
