//! Webhook handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::state::AppState;

use super::models::{WebhookPayload, WebhookResponse};

/// Handle incoming webhook from Git provider
///
/// This is a basic implementation that accepts webhooks and returns 200 OK.
/// Future enhancements will process the webhook payload and trigger workflows.
#[utoipa::path(
    post,
    path = "/api/webhooks/{provider_id}",
    params(
        ("provider_id" = i32, Path, description = "Git provider ID")
    ),
    request_body = WebhookPayload,
    responses(
        (status = 200, description = "Webhook received successfully", body = WebhookResponse),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "webhooks"
)]
pub async fn handle_webhook(
    State(_state): State<Arc<AppState>>,
    Path(_provider_id): Path<i32>,
    Json(_payload): Json<WebhookPayload>,
) -> Result<Json<WebhookResponse>, StatusCode> {
    // Basic implementation: accept webhook and return success
    // Future: validate provider, parse payload, trigger workflows
    Ok(Json(WebhookResponse {
        success: true,
        message: Some("Webhook received".to_string()),
    }))
}
