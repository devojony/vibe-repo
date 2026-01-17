use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers::*;

pub fn workspace_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/workspaces", post(create_workspace))
        .route("/api/workspaces", get(list_workspaces))
        .route("/api/workspaces/:id", get(get_workspace))
        .route("/api/workspaces/:id/status", patch(update_workspace_status))
        .route("/api/workspaces/:id", delete(delete_workspace))
}
