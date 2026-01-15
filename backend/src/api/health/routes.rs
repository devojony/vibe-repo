//! Health check routes
//!
//! Defines URL path mappings for the health feature.

use axum::{routing::get, Router};
use std::sync::Arc;

use super::handlers;
use crate::state::AppState;

/// Create health check router
/// All routes in this module are relative to the mount point (/health)
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(handlers::health_check))
}
