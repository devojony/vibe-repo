//! Event handlers for webhook events

use crate::api::webhooks::models::CommentInfo;
use crate::error::VibeRepoError;
use crate::state::AppState;

/// Handle issue or PR comment event
///
/// This function is called asynchronously to process comment events.
/// Future implementation will trigger AI agent workflows.
pub async fn handle_comment_event(
    comment_info: CommentInfo,
    state: &AppState,
) -> Result<(), VibeRepoError> {
    tracing::info!(
        comment_id = %comment_info.comment_id,
        author = %comment_info.comment_author,
        issue_or_pr = comment_info.issue_or_pr_number,
        repository = %comment_info.repository_full_name,
        action = %comment_info.action,
        comment_type = ?comment_info.comment_type,
        "Processing comment event"
    );

    // Get bot username from config
    let bot_username = &state.config.webhook.bot_username;
    let has_mention = super::mention::detect_mention(&comment_info.comment_body, bot_username);

    if has_mention {
        tracing::info!(
            comment_id = %comment_info.comment_id,
            "Comment mentions bot, triggering workflow"
        );
        // TODO: Trigger AI agent workflow (future task)
    } else {
        tracing::debug!(
            comment_id = %comment_info.comment_id,
            "Comment does not mention bot, skipping"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::webhooks::models::CommentType;
    use crate::config::AppConfig;
    use crate::services::RepositoryService;
    use crate::test_utils::db::create_test_database;
    use chrono::Utc;
    use std::sync::Arc;

    async fn create_test_state() -> AppState {
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::default();
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));
        AppState::new(db, config, repository_service)
    }

    #[tokio::test]
    async fn test_handle_comment_event_with_mention() {
        let state = create_test_state().await;
        let bot_username = &state.config.webhook.bot_username;

        let comment_info = CommentInfo {
            comment_id: "123".to_string(),
            comment_body: format!("@{} please help", bot_username),
            comment_author: "user1".to_string(),
            issue_or_pr_number: 42,
            repository_full_name: "owner/repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::Issue,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let result = handle_comment_event(comment_info, &state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_comment_event_without_mention() {
        let state = create_test_state().await;

        let comment_info = CommentInfo {
            comment_id: "124".to_string(),
            comment_body: "Just a regular comment".to_string(),
            comment_author: "user2".to_string(),
            issue_or_pr_number: 43,
            repository_full_name: "owner/repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::PullRequest,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let result = handle_comment_event(comment_info, &state).await;
        assert!(result.is_ok());
    }
}
