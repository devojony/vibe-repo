//! Health check handlers
//!
//! Contains the business logic for health check endpoints.

use axum::{extract::State, http::StatusCode, Json};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::state::AppState;

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Overall health status
    pub status: String,
    /// Database connection status
    pub database: String,
}

/// Health check handler - processes GET /health requests
///
/// Checks database connectivity and returns health status.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is unhealthy", body = HealthResponse)
    ),
    tag = "Health"
)]
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<HealthResponse>)> {
    // Check database connectivity by executing a simple query
    let db_connected = state.db.execute_unprepared("SELECT 1").await.is_ok();

    if db_connected {
        Ok(Json(HealthResponse {
            status: "healthy".to_string(),
            database: "connected".to_string(),
        }))
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "unhealthy".to_string(),
                database: "disconnected".to_string(),
            }),
        ))
    }
}
