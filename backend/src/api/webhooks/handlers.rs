//! Webhook handlers

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

use crate::{error::GitAutoDevError, state::AppState};

use super::models::WebhookResponse;

/// Verify webhook request signature
///
/// Retrieves provider from database and verifies webhook signature
async fn verify_webhook_request(
    provider_id: i32,
    headers: &HeaderMap,
    body: &[u8],
    state: &AppState,
) -> Result<(), GitAutoDevError> {
    // Get provider from database
    use crate::entities::prelude::*;
    use sea_orm::EntityTrait;

    let provider = RepoProvider::find_by_id(provider_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| GitAutoDevError::NotFound(format!("Provider {} not found", provider_id)))?;

    // Get webhook config for this provider
    // For now, we'll use a placeholder secret
    // Real implementation will query webhook_configs table (Task 3.3)
    let secret = "placeholder-secret";

    // Get signature from headers based on provider type
    let signature = headers
        .get("X-Gitea-Signature")
        .or_else(|| headers.get("X-Hub-Signature-256"))
        .or_else(|| headers.get("X-Gitlab-Token"))
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| GitAutoDevError::Validation("Missing webhook signature".to_string()))?;

    // Verify signature
    let body_str = std::str::from_utf8(body)
        .map_err(|e| GitAutoDevError::Validation(format!("Invalid UTF-8 in body: {}", e)))?;

    let is_valid = crate::api::webhooks::verification::verify_webhook_signature(
        &provider.provider_type,
        signature,
        body_str,
        secret,
    )?;

    if !is_valid {
        return Err(GitAutoDevError::Validation(
            "Invalid webhook signature".to_string(),
        ));
    }

    Ok(())
}

/// Handle incoming webhook from Git provider
///
/// Verifies webhook signature and processes the payload.
/// Future enhancements will parse payload and trigger workflows.
#[utoipa::path(
    post,
    path = "/api/webhooks/{provider_id}",
    params(
        ("provider_id" = i32, Path, description = "Git provider ID")
    ),
    request_body = String,
    responses(
        (status = 200, description = "Webhook received successfully", body = WebhookResponse),
        (status = 400, description = "Invalid signature or payload"),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "webhooks"
)]
pub async fn handle_webhook(
    Path(provider_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, GitAutoDevError> {
    tracing::info!("Received webhook for provider {}", provider_id);

    // Verify webhook signature
    verify_webhook_request(provider_id, &headers, &body, &state).await?;

    tracing::info!("Webhook signature verified for provider {}", provider_id);

    // TODO: Process webhook payload (Task 3.3)

    Ok(Json(WebhookResponse {
        success: true,
        message: Some("Webhook received and verified".to_string()),
    }))
}
