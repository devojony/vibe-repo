//! Webhook handlers

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

use crate::{error::VibeRepoError, state::AppState};

use super::models::WebhookResponse;

/// Verify webhook request signature
///
/// Retrieves provider from database and verifies webhook signature
async fn verify_webhook_request(
    provider_id: i32,
    headers: &HeaderMap,
    body: &[u8],
    state: &AppState,
) -> Result<(), VibeRepoError> {
    // Get provider from database
    use crate::entities::prelude::*;
    use sea_orm::EntityTrait;

    let provider = RepoProvider::find_by_id(provider_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| {
            tracing::error!(
                provider_id = provider_id,
                "Provider not found for webhook request"
            );
            VibeRepoError::NotFound(format!("Provider {} not found", provider_id))
        })?;

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
        .ok_or_else(|| {
            tracing::error!(
                provider_id = provider_id,
                "Missing webhook signature header"
            );
            VibeRepoError::Validation("Missing webhook signature".to_string())
        })?;

    // Verify signature
    let body_str = std::str::from_utf8(body).map_err(|e| {
        tracing::error!(
            provider_id = provider_id,
            error = %e,
            "Invalid UTF-8 in webhook body"
        );
        VibeRepoError::Validation(format!("Invalid UTF-8 in body: {}", e))
    })?;

    let is_valid = crate::api::webhooks::verification::verify_webhook_signature(
        &provider.provider_type,
        signature,
        body_str,
        secret,
    )?;

    if !is_valid {
        tracing::error!(
            provider_id = provider_id,
            "Invalid webhook signature"
        );
        return Err(VibeRepoError::Validation(
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
) -> Result<Json<WebhookResponse>, VibeRepoError> {
    tracing::info!(
        provider_id = provider_id,
        "Received webhook request"
    );

    // Verify webhook signature
    verify_webhook_request(provider_id, &headers, &body, &state).await?;

    tracing::info!(
        provider_id = provider_id,
        "Webhook signature verified"
    );

    // Parse webhook payload based on event type
    let payload_str = std::str::from_utf8(&body).map_err(|e| {
        tracing::error!(
            provider_id = provider_id,
            error = %e,
            "Invalid UTF-8 in webhook payload"
        );
        VibeRepoError::Validation(format!("Invalid UTF-8 in payload: {}", e))
    })?;

    // Detect event type from headers
    let event_type = headers
        .get("X-Gitea-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    tracing::info!(
        provider_id = provider_id,
        event_type = event_type,
        "Processing webhook event"
    );

    // Parse based on event type
    match event_type {
        "issue_comment" => {
            let payload: super::models::GiteaIssueCommentPayload =
                serde_json::from_str(payload_str).map_err(|e| {
                    tracing::error!(
                        provider_id = provider_id,
                        event_type = "issue_comment",
                        error = %e,
                        "Failed to parse issue comment payload"
                    );
                    VibeRepoError::Validation(format!(
                        "Failed to parse issue comment payload: {}",
                        e
                    ))
                })?;
            
            let comment_info = payload.extract_comment_info()?;
            tracing::info!(
                comment_id = %comment_info.comment_id,
                author = %comment_info.comment_author,
                issue = comment_info.issue_or_pr_number,
                repository = %comment_info.repository_full_name,
                "Extracted issue comment info"
            );
            
            // Spawn async task to handle event
            let comment_info_clone = comment_info.clone();
            tokio::spawn(async move {
                if let Err(e) = super::event_handler::handle_comment_event(comment_info_clone).await {
                    tracing::error!(error = %e, "Failed to handle comment event");
                }
            });
        }
        "pull_request_comment" => {
            let payload: super::models::GiteaPullRequestCommentPayload =
                serde_json::from_str(payload_str).map_err(|e| {
                    tracing::error!(
                        provider_id = provider_id,
                        event_type = "pull_request_comment",
                        error = %e,
                        "Failed to parse PR comment payload"
                    );
                    VibeRepoError::Validation(format!("Failed to parse PR comment payload: {}", e))
                })?;
            
            let comment_info = payload.extract_comment_info()?;
            tracing::info!(
                comment_id = %comment_info.comment_id,
                author = %comment_info.comment_author,
                pr = comment_info.issue_or_pr_number,
                repository = %comment_info.repository_full_name,
                "Extracted PR comment info"
            );
            
            // Spawn async task to handle event
            let comment_info_clone = comment_info.clone();
            tokio::spawn(async move {
                if let Err(e) = super::event_handler::handle_comment_event(comment_info_clone).await {
                    tracing::error!(error = %e, "Failed to handle comment event");
                }
            });
        }
        _ => {
            tracing::warn!(
                provider_id = provider_id,
                event_type = event_type,
                "Unsupported webhook event type"
            );
        }
    }

    Ok(Json(WebhookResponse {
        success: true,
        message: Some("Webhook received and verified".to_string()),
    }))
}
