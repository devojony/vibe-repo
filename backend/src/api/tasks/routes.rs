use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers;

pub fn task_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tasks", post(handlers::create_task))
        .route("/api/tasks", get(handlers::list_tasks_by_workspace))
        .route("/api/tasks/:id", get(handlers::get_task))
        .route("/api/tasks/:id/status", patch(handlers::update_task_status))
}
