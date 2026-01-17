use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unified user model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

/// Unified repository model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitRepository {
    pub id: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub clone_url: String,
    pub ssh_url: Option<String>,
    pub default_branch: String,
    pub private: bool,
    pub permissions: RepositoryPermissions,
}

/// Repository permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepositoryPermissions {
    pub admin: bool,
    pub push: bool,
    pub pull: bool,
}

/// Unified branch model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitBranch {
    pub name: String,
    pub commit_sha: String,
    pub protected: bool,
}

/// Unified issue model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitIssue {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: IssueState,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Issue state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

/// Unified pull request model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitPullRequest {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: PullRequestState,
    pub source_branch: String,
    pub target_branch: String,
    pub mergeable: Option<bool>,
    pub merged: bool,
    pub labels: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Pull request state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

/// Unified label model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitLabel {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

/// Request to create a branch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateBranchRequest {
    pub name: String,
    pub source: String, // Source branch or commit SHA
}

/// Request to create an issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateIssueRequest {
    pub title: String,
    pub body: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

/// Request to update an issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<IssueState>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

/// Request to create a pull request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreatePullRequestRequest {
    pub title: String,
    pub body: Option<String>,
    pub head: String, // Source branch
    pub base: String, // Target branch
}

/// Request to update a pull request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdatePullRequestRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<PullRequestState>,
}

/// Options for merging a pull request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeOptions {
    pub strategy: MergeStrategy,
    pub delete_branch: bool,
}

/// Merge strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    Merge,
    Rebase,
    Squash,
}

/// Request to create a label
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateLabelRequest {
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

/// Filter for listing issues
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IssueFilter {
    pub state: Option<IssueState>,
    pub labels: Option<Vec<String>>,
    pub assignee: Option<String>,
}

/// Filter for listing pull requests
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PullRequestFilter {
    pub state: Option<PullRequestState>,
    pub labels: Option<Vec<String>>,
}

/// Unified webhook event type across all Git platforms
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    /// Issue comment created/edited/deleted
    IssueComment,
    /// Pull request comment created/edited/deleted
    PullRequestComment,
    /// Commit comment created/edited/deleted
    CommitComment,
    /// Code pushed to repository
    Push,
}

/// Unified request to create a webhook (internal use)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateWebhookRequest {
    /// Webhook endpoint URL
    pub url: String,
    /// Secret for signing webhook payloads
    pub secret: String,
    /// Events that trigger this webhook
    pub events: Vec<WebhookEvent>,
    /// Whether webhook is active
    pub active: bool,
}

/// Unified webhook response from Git platforms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitWebhook {
    /// Webhook ID (string to support different platforms)
    pub id: String,
    /// Webhook endpoint URL
    pub url: String,
    /// Whether webhook is active
    pub active: bool,
    /// Events that trigger this webhook
    pub events: Vec<WebhookEvent>,
    /// When webhook was created
    pub created_at: DateTime<Utc>,
}
