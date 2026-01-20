use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers::*;

pub fn init_script_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/workspaces/:id/init-script", put(update_init_script))
        .route("/api/workspaces/:id/init-script/logs", get(get_logs))
        .route(
            "/api/workspaces/:id/init-script/logs/full",
            get(download_full_log),
        )
        .route(
            "/api/workspaces/:id/init-script/execute",
            post(execute_script),
        )
}
