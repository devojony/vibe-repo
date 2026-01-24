// Git Provider abstraction module
pub mod error;
pub mod factory;
pub mod gitea;
pub mod models;
pub mod traits;

pub use error::GitProviderError;
pub use factory::GitClientFactory;
pub use models::*;
pub use traits::GitProvider;
// Note: GitClient, GitHubClient, and GitLabClient are defined in this module
// and are already public via their `pub` declarations below

use async_trait::async_trait;
use gitea::GiteaClient;

/// Unified Git client enum for compile-time polymorphism
///
/// This enum wraps all Git provider implementations and uses static dispatch
/// to eliminate virtual dispatch overhead while maintaining a clean abstraction.
/// Method calls are forwarded to the appropriate variant using a dispatch macro.
///
/// Note: We use a manual macro instead of the static-dispatch crate due to
/// compatibility issues between static-dispatch and async_trait's lifetime handling.
///
/// # Variants
/// - `Gitea` - Gitea Git provider implementation
/// - `GitHub` - GitHub Git provider implementation (placeholder for future)
/// - `GitLab` - GitLab Git provider implementation (placeholder for future)
///
/// # Example
/// ```ignore
/// use vibe_repo::git_provider::{GitClient, GitProvider};
///
/// let client = GitClient::Gitea(GiteaClient::new("https://gitea.example.com", "token")?);
/// let user = client.get_current_user().await?;
/// ```
pub enum GitClient {
    /// Gitea Git provider
    Gitea(GiteaClient),
    /// GitHub Git provider (placeholder for future implementation)
    GitHub(GitHubClient),
    /// GitLab Git provider (placeholder for future implementation)
    GitLab(GitLabClient),
}

/// Placeholder for GitHub client implementation
///
/// This struct exists to allow the GitClient enum to have a GitHub variant
/// for future implementation. It implements GitProvider with stub methods
/// that return UnsupportedProvider errors.
pub struct GitHubClient {
    base_url: String,
    #[allow(dead_code)]
    access_token: String,
}

impl GitHubClient {
    /// Create a new GitHub client (placeholder)
    pub fn new(base_url: &str, access_token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            access_token: access_token.to_string(),
        }
    }
}

/// Placeholder for GitLab client implementation
///
/// This struct exists to allow the GitClient enum to have a GitLab variant
/// for future implementation. It implements GitProvider with stub methods
/// that return UnsupportedProvider errors.
pub struct GitLabClient {
    base_url: String,
    #[allow(dead_code)]
    access_token: String,
}

impl GitLabClient {
    /// Create a new GitLab client (placeholder)
    pub fn new(base_url: &str, access_token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            access_token: access_token.to_string(),
        }
    }
}

/// Macro for dispatching method calls to the appropriate GitClient variant
///
/// This macro generates match arms for each variant, forwarding the method call
/// to the underlying client implementation.
macro_rules! dispatch_git_provider {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            GitClient::Gitea(client) => client.$method($($arg),*),
            GitClient::GitHub(client) => client.$method($($arg),*),
            GitClient::GitLab(client) => client.$method($($arg),*),
        }
    };
}

// Manual GitProvider implementation for GitClient using dispatch macro
#[async_trait]
impl GitProvider for GitClient {
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError> {
        dispatch_git_provider!(self, validate_token).await
    }

    async fn get_current_user(&self) -> Result<GitUser, GitProviderError> {
        dispatch_git_provider!(self, get_current_user).await
    }

    async fn list_repositories(&self) -> Result<Vec<GitRepository>, GitProviderError> {
        dispatch_git_provider!(self, list_repositories).await
    }

    async fn get_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<GitRepository, GitProviderError> {
        dispatch_git_provider!(self, get_repository, owner, repo).await
    }

    async fn list_branches(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitBranch>, GitProviderError> {
        dispatch_git_provider!(self, list_branches, owner, repo).await
    }

    async fn get_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<GitBranch, GitProviderError> {
        dispatch_git_provider!(self, get_branch, owner, repo, branch).await
    }

    async fn create_branch(
        &self,
        owner: &str,
        repo: &str,
        req: CreateBranchRequest,
    ) -> Result<GitBranch, GitProviderError> {
        dispatch_git_provider!(self, create_branch, owner, repo, req).await
    }

    async fn delete_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<(), GitProviderError> {
        dispatch_git_provider!(self, delete_branch, owner, repo, branch).await
    }

    async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        filter: Option<IssueFilter>,
    ) -> Result<Vec<GitIssue>, GitProviderError> {
        dispatch_git_provider!(self, list_issues, owner, repo, filter).await
    }

    async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GitIssue, GitProviderError> {
        dispatch_git_provider!(self, get_issue, owner, repo, number).await
    }

    async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: CreateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        dispatch_git_provider!(self, create_issue, owner, repo, req).await
    }

    async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: UpdateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        dispatch_git_provider!(self, update_issue, owner, repo, number, req).await
    }

    async fn add_issue_labels(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        labels: Vec<String>,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        dispatch_git_provider!(self, add_issue_labels, owner, repo, number, labels).await
    }

    async fn remove_issue_label(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        label: &str,
    ) -> Result<(), GitProviderError> {
        dispatch_git_provider!(self, remove_issue_label, owner, repo, number, label).await
    }

    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        filter: Option<PullRequestFilter>,
    ) -> Result<Vec<GitPullRequest>, GitProviderError> {
        dispatch_git_provider!(self, list_pull_requests, owner, repo, filter).await
    }

    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GitPullRequest, GitProviderError> {
        dispatch_git_provider!(self, get_pull_request, owner, repo, number).await
    }

    async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        req: CreatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        dispatch_git_provider!(self, create_pull_request, owner, repo, req).await
    }

    async fn update_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: UpdatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        dispatch_git_provider!(self, update_pull_request, owner, repo, number, req).await
    }

    async fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        opts: MergeOptions,
    ) -> Result<(), GitProviderError> {
        dispatch_git_provider!(self, merge_pull_request, owner, repo, number, opts).await
    }

    async fn list_labels(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        dispatch_git_provider!(self, list_labels, owner, repo).await
    }

    async fn create_label(
        &self,
        owner: &str,
        repo: &str,
        req: CreateLabelRequest,
    ) -> Result<GitLabel, GitProviderError> {
        dispatch_git_provider!(self, create_label, owner, repo, req).await
    }

    async fn delete_label(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> Result<(), GitProviderError> {
        dispatch_git_provider!(self, delete_label, owner, repo, name).await
    }

    async fn create_webhook(
        &self,
        owner: &str,
        repo: &str,
        req: CreateWebhookRequest,
    ) -> Result<GitWebhook, GitProviderError> {
        dispatch_git_provider!(self, create_webhook, owner, repo, req).await
    }

    async fn delete_webhook(
        &self,
        owner: &str,
        repo: &str,
        webhook_id: &str,
    ) -> Result<(), GitProviderError> {
        dispatch_git_provider!(self, delete_webhook, owner, repo, webhook_id).await
    }

    async fn list_webhooks(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitWebhook>, GitProviderError> {
        dispatch_git_provider!(self, list_webhooks, owner, repo).await
    }

    fn provider_type(&self) -> &'static str {
        dispatch_git_provider!(self, provider_type)
    }

    fn base_url(&self) -> &str {
        dispatch_git_provider!(self, base_url)
    }
}

// Placeholder implementations for GitHub and GitLab clients
// These return UnsupportedProvider errors until properly implemented

#[async_trait]
impl GitProvider for GitHubClient {
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn get_current_user(&self) -> Result<GitUser, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn list_repositories(&self) -> Result<Vec<GitRepository>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn get_repository(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<GitRepository, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn list_branches(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitBranch>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn get_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<GitBranch, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn create_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateBranchRequest,
    ) -> Result<GitBranch, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn delete_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn list_issues(
        &self,
        _owner: &str,
        _repo: &str,
        _filter: Option<IssueFilter>,
    ) -> Result<Vec<GitIssue>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn get_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
    ) -> Result<GitIssue, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn create_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn update_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _req: UpdateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn add_issue_labels(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _labels: Vec<String>,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn remove_issue_label(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _label: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn list_pull_requests(
        &self,
        _owner: &str,
        _repo: &str,
        _filter: Option<PullRequestFilter>,
    ) -> Result<Vec<GitPullRequest>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn get_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
    ) -> Result<GitPullRequest, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn create_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn update_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _req: UpdatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn merge_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _opts: MergeOptions,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn list_labels(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn create_label(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateLabelRequest,
    ) -> Result<GitLabel, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn delete_label(
        &self,
        _owner: &str,
        _repo: &str,
        _name: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn create_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateWebhookRequest,
    ) -> Result<GitWebhook, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn delete_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        _webhook_id: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    async fn list_webhooks(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitWebhook>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("github".to_string()))
    }

    fn provider_type(&self) -> &'static str {
        "github"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[async_trait]
impl GitProvider for GitLabClient {
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn get_current_user(&self) -> Result<GitUser, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn list_repositories(&self) -> Result<Vec<GitRepository>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn get_repository(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<GitRepository, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn list_branches(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitBranch>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn get_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<GitBranch, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn create_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateBranchRequest,
    ) -> Result<GitBranch, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn delete_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn list_issues(
        &self,
        _owner: &str,
        _repo: &str,
        _filter: Option<IssueFilter>,
    ) -> Result<Vec<GitIssue>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn get_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
    ) -> Result<GitIssue, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn create_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn update_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _req: UpdateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn add_issue_labels(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _labels: Vec<String>,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn remove_issue_label(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _label: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn list_pull_requests(
        &self,
        _owner: &str,
        _repo: &str,
        _filter: Option<PullRequestFilter>,
    ) -> Result<Vec<GitPullRequest>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn get_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
    ) -> Result<GitPullRequest, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn create_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn update_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _req: UpdatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn merge_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _opts: MergeOptions,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn list_labels(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn create_label(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateLabelRequest,
    ) -> Result<GitLabel, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn delete_label(
        &self,
        _owner: &str,
        _repo: &str,
        _name: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn create_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateWebhookRequest,
    ) -> Result<GitWebhook, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn delete_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        _webhook_id: &str,
    ) -> Result<(), GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    async fn list_webhooks(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitWebhook>, GitProviderError> {
        Err(GitProviderError::UnsupportedProvider("gitlab".to_string()))
    }

    fn provider_type(&self) -> &'static str {
        "gitlab"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_client_gitea_variant() {
        let client = GitClient::Gitea(GiteaClient::new("https://gitea.example.com", "token").unwrap());
        assert_eq!(client.provider_type(), "gitea");
        assert_eq!(client.base_url(), "https://gitea.example.com");
    }

    #[test]
    fn test_git_client_github_variant() {
        let client = GitClient::GitHub(GitHubClient::new("https://github.com", "token"));
        assert_eq!(client.provider_type(), "github");
        assert_eq!(client.base_url(), "https://github.com");
    }

    #[test]
    fn test_git_client_gitlab_variant() {
        let client = GitClient::GitLab(GitLabClient::new("https://gitlab.com", "token"));
        assert_eq!(client.provider_type(), "gitlab");
        assert_eq!(client.base_url(), "https://gitlab.com");
    }

    #[test]
    fn test_github_client_new() {
        let client = GitHubClient::new("https://github.com/", "token");
        assert_eq!(client.base_url, "https://github.com");
    }

    #[test]
    fn test_gitlab_client_new() {
        let client = GitLabClient::new("https://gitlab.com/", "token");
        assert_eq!(client.base_url, "https://gitlab.com");
    }

    #[test]
    fn test_git_client_send_sync() {
        // Verify GitClient is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GitClient>();
    }
}
