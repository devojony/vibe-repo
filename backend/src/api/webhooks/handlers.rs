//! Webhook handlers

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::{
    entities::{prelude::*, task, workspace},
    error::VibeRepoError,
    services::IssueClosureService,
    state::AppState,
};

use super::models::WebhookResponse;

/// Verify webhook request signature
///
/// Retrieves repository from database and verifies webhook signature
async fn verify_webhook_request(
    repository_id: i32,
    headers: &HeaderMap,
    body: &[u8],
    state: &AppState,
) -> Result<(), VibeRepoError> {
    // Get repository with provider configuration
    let repo = Repository::find_by_id(repository_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| {
            tracing::error!(
                repository_id = repository_id,
                "Repository not found for webhook request"
            );
            VibeRepoError::NotFound(format!("Repository {} not found", repository_id))
        })?;

    // Get webhook secret from repository
    let secret = repo.webhook_secret.as_ref().ok_or_else(|| {
        tracing::error!(
            repository_id = repository_id,
            "Webhook secret not configured for repository"
        );
        VibeRepoError::Validation(format!(
            "Webhook secret not configured for repository {}",
            repository_id
        ))
    })?;

    // Get signature from headers based on provider type
    let signature = headers
        .get("X-Gitea-Signature")
        .or_else(|| headers.get("X-Hub-Signature-256"))
        .or_else(|| headers.get("X-Gitlab-Token"))
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::error!(
                repository_id = repository_id,
                "Missing webhook signature header"
            );
            VibeRepoError::Validation("Missing webhook signature".to_string())
        })?;

    // Verify signature
    let body_str = std::str::from_utf8(body).map_err(|e| {
        tracing::error!(
            repository_id = repository_id,
            error = %e,
            "Invalid UTF-8 in webhook body"
        );
        VibeRepoError::Validation(format!("Invalid UTF-8 in body: {}", e))
    })?;

    let is_valid = crate::api::webhooks::verification::verify_webhook_signature(
        &repo.provider_type,
        signature,
        body_str,
        secret,
    )?;

    if !is_valid {
        tracing::error!(repository_id = repository_id, "Invalid webhook signature");
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
    path = "/api/webhooks/{repository_id}",
    params(
        ("repository_id" = i32, Path, description = "Repository ID")
    ),
    request_body = String,
    responses(
        (status = 200, description = "Webhook received successfully", body = WebhookResponse),
        (status = 400, description = "Invalid signature or payload"),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "webhooks"
)]
pub async fn handle_webhook(
    Path(repository_id): Path<i32>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, VibeRepoError> {
    tracing::info!(repository_id = repository_id, "Received webhook request");

    // Log webhook details for debugging
    let event_type = headers
        .get("X-Gitea-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    let signature = headers
        .get("X-Gitea-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none");
    tracing::info!(
        repository_id = repository_id,
        event_type = event_type,
        signature = signature,
        body_len = body.len(),
        "Webhook details"
    );

    // Verify webhook signature
    verify_webhook_request(repository_id, &headers, &body, &state).await?;

    tracing::info!(repository_id = repository_id, "Webhook signature verified");

    // Parse webhook payload based on event type
    let payload_str = std::str::from_utf8(&body).map_err(|e| {
        tracing::error!(
            repository_id = repository_id,
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
        repository_id = repository_id,
        event_type = event_type,
        "Processing webhook event"
    );

    // Parse based on event type
    match event_type {
        "issue_comment" => {
            let payload: super::models::GiteaIssueCommentPayload =
                serde_json::from_str(payload_str).map_err(|e| {
                    tracing::error!(
                        repository_id = repository_id,
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
            let state_clone = Arc::clone(&state);
            tokio::spawn(async move {
                if let Err(e) =
                    super::event_handler::handle_comment_event(comment_info_clone, &state_clone)
                        .await
                {
                    tracing::error!(error = %e, "Failed to handle comment event");
                }
            });
        }
        "pull_request_comment" => {
            let payload: super::models::GiteaPullRequestCommentPayload =
                serde_json::from_str(payload_str).map_err(|e| {
                    tracing::error!(
                        repository_id = repository_id,
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
            let state_clone = Arc::clone(&state);
            tokio::spawn(async move {
                if let Err(e) =
                    super::event_handler::handle_comment_event(comment_info_clone, &state_clone)
                        .await
                {
                    tracing::error!(error = %e, "Failed to handle comment event");
                }
            });
        }
        "pull_request" => {
            // Parse payload as generic JSON to check for PR merge
            let payload: serde_json::Value = serde_json::from_str(payload_str).map_err(|e| {
                tracing::error!(
                    repository_id = repository_id,
                    event_type = "pull_request",
                    error = %e,
                    "Failed to parse pull request payload"
                );
                VibeRepoError::Validation(format!("Failed to parse pull request payload: {}", e))
            })?;

            // Check if this is a PR merge event
            let action = payload.get("action").and_then(|v| v.as_str());
            let pr_data = payload.get("pull_request");
            let merged = pr_data
                .and_then(|pr| pr.get("merged"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if action == Some("closed") && merged {
                let pr_number = pr_data
                    .and_then(|pr| pr.get("number"))
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32);

                if let Some(pr_num) = pr_number {
                    info!(
                        repository_id = repository_id,
                        pr_number = pr_num,
                        "PR merged, attempting to close linked issue"
                    );

                    // Get workspace for this repository
                    let workspace = Workspace::find()
                        .filter(workspace::Column::RepositoryId.eq(repository_id))
                        .one(&state.db)
                        .await?;

                    let workspace = match workspace {
                        Some(w) => w,
                        None => {
                            warn!(
                                repository_id = repository_id,
                                "No workspace found for repository"
                            );
                            // Continue processing - webhook was received successfully
                            return Ok(Json(WebhookResponse {
                                success: true,
                                message: Some("Webhook received and verified".to_string()),
                            }));
                        }
                    };

                    // Find task by PR number
                    let task = Task::find()
                        .filter(task::Column::PrNumber.eq(pr_num))
                        .filter(task::Column::WorkspaceId.eq(workspace.id))
                        .one(&state.db)
                        .await?;

                    if let Some(task) = task {
                        info!(
                            task_id = task.id,
                            pr_number = pr_num,
                            "Found task for merged PR, closing issue"
                        );

                        let closure_service = IssueClosureService::new(state.db.clone());
                        if let Err(e) = closure_service.close_issue_for_task(task.id).await {
                            error!(
                                task_id = task.id,
                                error = %e,
                                "Failed to close issue"
                            );
                            // Don't return error - webhook was received successfully
                        }
                    } else {
                        info!(
                            pr_number = pr_num,
                            workspace_id = workspace.id,
                            "No task found for merged PR"
                        );
                    }
                }
            }
        }
        _ => {
            tracing::warn!(
                repository_id = repository_id,
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
