use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers::*;

pub fn agent_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/agents", post(create_agent))
        .route("/api/agents/:id", get(get_agent))
        .route(
            "/api/workspaces/:workspace_id/agents",
            get(list_agents_by_workspace),
        )
        .route("/api/agents/:id/enabled", patch(update_agent_enabled))
        .route("/api/agents/:id", delete(delete_agent))
}
