use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::{handlers::*, lifecycle_handlers};

pub fn workspace_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/workspaces", post(create_workspace))
        .route("/api/workspaces", get(list_workspaces))
        .route("/api/workspaces/:id", get(get_workspace))
        .route("/api/workspaces/:id/status", patch(update_workspace_status))
        .route("/api/workspaces/:id", delete(delete_workspace))
        .route(
            "/api/workspaces/:id/container",
            post(lifecycle_handlers::create_container),
        )
        .route(
            "/api/workspaces/:id/restart",
            post(lifecycle_handlers::restart_workspace),
        )
        .route(
            "/api/workspaces/:id/stats",
            get(lifecycle_handlers::get_workspace_stats),
        )
}
