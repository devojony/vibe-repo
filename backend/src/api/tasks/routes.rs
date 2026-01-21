use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers;

pub fn task_routes() -> Router<Arc<AppState>> {
    Router::new()
        // CRUD operations
        .route("/api/tasks", post(handlers::create_task))
        .route("/api/tasks", get(handlers::list_tasks_by_workspace))
        .route("/api/tasks/:id", get(handlers::get_task))
        .route("/api/tasks/:id", patch(handlers::update_task))
        .route("/api/tasks/:id", delete(handlers::delete_task))
        // Status management operations
        .route("/api/tasks/:id/status", patch(handlers::update_task_status))
        .route("/api/tasks/:id/assign", post(handlers::assign_agent))
        .route("/api/tasks/:id/start", post(handlers::start_task))
        .route("/api/tasks/:id/complete", post(handlers::complete_task))
        .route("/api/tasks/:id/fail", post(handlers::fail_task))
        .route("/api/tasks/:id/retry", post(handlers::retry_task))
        .route("/api/tasks/:id/cancel", post(handlers::cancel_task))
        // Task execution
        .route("/api/tasks/:id/execute", post(handlers::execute_task))
}
