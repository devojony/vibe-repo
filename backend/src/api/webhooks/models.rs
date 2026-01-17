//! Webhook request/response models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::VibeRepoError;

/// Generic webhook payload
/// This will be expanded to handle specific webhook types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookPayload {
    /// Raw JSON payload from the webhook
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Webhook response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookResponse {
    /// Success status
    pub success: bool,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Gitea Webhook Payload Models
// ============================================================================

/// Gitea issue comment webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GiteaIssueCommentPayload {
    /// Action performed: "created", "edited", "deleted"
    pub action: String,
    /// Issue information
    pub issue: GiteaIssuePayload,
    /// Comment information
    pub comment: GiteaCommentPayload,
    /// Repository information
    pub repository: GiteaRepositoryPayload,
    /// User who triggered the event
    pub sender: GiteaUserPayload,
}

/// Gitea pull request comment webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GiteaPullRequestCommentPayload {
    /// Action performed: "created", "edited", "deleted"
    pub action: String,
    /// Pull request information
    pub pull_request: GiteaPullRequestPayload,
    /// Comment information
    pub comment: GiteaCommentPayload,
    /// Repository information
    pub repository: GiteaRepositoryPayload,
    /// User who triggered the event
    pub sender: GiteaUserPayload,
}

/// Gitea issue in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GiteaIssuePayload {
    /// Issue ID
    pub id: i64,
    /// Issue number
    pub number: i64,
    /// Issue title
    pub title: String,
    /// Issue body/description
    pub body: Option<String>,
    /// Issue state: "open" or "closed"
    pub state: String,
}

/// Gitea pull request in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GiteaPullRequestPayload {
    /// Pull request ID
    pub id: i64,
    /// Pull request number
    pub number: i64,
    /// Pull request title
    pub title: String,
    /// Pull request body/description
    pub body: Option<String>,
    /// Pull request state: "open" or "closed"
    pub state: String,
}

/// Gitea comment in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GiteaCommentPayload {
    /// Comment ID
    pub id: i64,
    /// Comment body/content
    pub body: String,
    /// User who created the comment
    pub user: GiteaUserPayload,
    /// When comment was created (ISO 8601 format)
    pub created_at: String,
    /// When comment was last updated (ISO 8601 format)
    pub updated_at: String,
}

/// Gitea repository in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GiteaRepositoryPayload {
    /// Repository ID
    pub id: i64,
    /// Repository name
    pub name: String,
    /// Full repository name (owner/repo)
    pub full_name: String,
    /// Repository owner
    pub owner: GiteaUserPayload,
}

/// Gitea user in webhook payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GiteaUserPayload {
    /// User ID
    pub id: i64,
    /// Username/login
    pub login: String,
    /// User email (optional)
    pub email: Option<String>,
    /// User avatar URL (optional)
    pub avatar_url: Option<String>,
}

// ============================================================================
// Unified Comment Information
// ============================================================================

/// Type of comment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommentType {
    /// Comment on an issue
    Issue,
    /// Comment on a pull request
    PullRequest,
}

/// Unified comment information extracted from webhook payloads
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommentInfo {
    /// Comment ID (as string for cross-platform compatibility)
    pub comment_id: String,
    /// Comment body/content
    pub comment_body: String,
    /// Username of comment author
    pub comment_author: String,
    /// Issue or PR number
    pub issue_or_pr_number: i64,
    /// Full repository name (owner/repo)
    pub repository_full_name: String,
    /// Action performed: "created", "edited", "deleted"
    pub action: String,
    /// Type of comment (issue or pull request)
    pub comment_type: CommentType,
    /// When comment was created (ISO 8601 format)
    pub created_at: String,
    /// When comment was last updated (ISO 8601 format)
    pub updated_at: String,
}

// ============================================================================
// Payload Extraction Methods
// ============================================================================

impl GiteaIssueCommentPayload {
    /// Extract unified comment information from issue comment payload
    ///
    /// Validates the action field and returns an error for invalid actions.
    pub fn extract_comment_info(&self) -> Result<CommentInfo, VibeRepoError> {
        // Validate action
        if !matches!(self.action.as_str(), "created" | "edited" | "deleted") {
            return Err(VibeRepoError::Validation(format!(
                "Invalid action '{}'. Expected 'created', 'edited', or 'deleted'",
                self.action
            )));
        }

        Ok(CommentInfo {
            comment_id: self.comment.id.to_string(),
            comment_body: self.comment.body.clone(),
            comment_author: self.comment.user.login.clone(),
            issue_or_pr_number: self.issue.number,
            repository_full_name: self.repository.full_name.clone(),
            action: self.action.clone(),
            comment_type: CommentType::Issue,
            created_at: self.comment.created_at.clone(),
            updated_at: self.comment.updated_at.clone(),
        })
    }
}

impl GiteaPullRequestCommentPayload {
    /// Extract unified comment information from PR comment payload
    ///
    /// Validates the action field and returns an error for invalid actions.
    pub fn extract_comment_info(&self) -> Result<CommentInfo, VibeRepoError> {
        // Validate action
        if !matches!(self.action.as_str(), "created" | "edited" | "deleted") {
            return Err(VibeRepoError::Validation(format!(
                "Invalid action '{}'. Expected 'created', 'edited', or 'deleted'",
                self.action
            )));
        }

        Ok(CommentInfo {
            comment_id: self.comment.id.to_string(),
            comment_body: self.comment.body.clone(),
            comment_author: self.comment.user.login.clone(),
            issue_or_pr_number: self.pull_request.number,
            repository_full_name: self.repository.full_name.clone(),
            action: self.action.clone(),
            comment_type: CommentType::PullRequest,
            created_at: self.comment.created_at.clone(),
            updated_at: self.comment.updated_at.clone(),
        })
    }
}
