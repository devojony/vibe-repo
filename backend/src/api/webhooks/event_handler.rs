//! Event handlers for webhook events

use crate::api::webhooks::models::CommentInfo;
use crate::error::VibeRepoError;

/// Handle issue or PR comment event
///
/// This function is called asynchronously to process comment events.
/// Future implementation will trigger AI agent workflows.
pub async fn handle_comment_event(
    comment_info: CommentInfo,
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

    // Check for @mention (placeholder bot username)
    let bot_username = "gitautodev-bot"; // TODO: Load from config
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
    use chrono::Utc;

    #[tokio::test]
    async fn test_handle_comment_event_with_mention() {
        let comment_info = CommentInfo {
            comment_id: "123".to_string(),
            comment_body: "@gitautodev-bot please help".to_string(),
            comment_author: "user1".to_string(),
            issue_or_pr_number: 42,
            repository_full_name: "owner/repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::Issue,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let result = handle_comment_event(comment_info).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_comment_event_without_mention() {
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

        let result = handle_comment_event(comment_info).await;
        assert!(result.is_ok());
    }
}
