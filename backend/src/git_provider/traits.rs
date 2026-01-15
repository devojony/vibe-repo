use async_trait::async_trait;

use super::{
    error::GitProviderError,
    models::{
        CreateBranchRequest, CreateIssueRequest, CreateLabelRequest, CreatePullRequestRequest,
        GitBranch, GitIssue, GitLabel, GitPullRequest, GitRepository, GitUser, IssueFilter,
        MergeOptions, PullRequestFilter, UpdateIssueRequest, UpdatePullRequestRequest,
    },
};

/// Unified Git provider interface for interacting with different Git platforms
///
/// This trait defines a common API for Git operations across multiple platforms
/// (Gitea, GitHub, GitLab). All methods are async and return Results with
/// descriptive GitProviderError variants.
///
/// The GitClient enum implements this trait using a manual dispatch macro,
/// enabling compile-time polymorphism and eliminating virtual dispatch overhead
/// while maintaining a clean abstraction layer.
///
/// Note: We use a manual macro instead of the static-dispatch crate due to
/// compatibility issues between static-dispatch and async_trait's lifetime handling.
#[async_trait]
pub trait GitProvider: Send + Sync {
    // ==================== User Operations ====================

    /// Validate the provider's access token
    ///
    /// Returns (true, Some(user)) if token is valid, (false, None) if invalid (401),
    /// or an error for other failures.
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError>;

    /// Get the current authenticated user
    async fn get_current_user(&self) -> Result<GitUser, GitProviderError>;

    // ==================== Repository Operations ====================

    /// List all repositories accessible to the authenticated user
    async fn list_repositories(&self) -> Result<Vec<GitRepository>, GitProviderError>;

    /// Get a specific repository by owner and name
    async fn get_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<GitRepository, GitProviderError>;

    // ==================== Branch Operations ====================

    /// List all branches in a repository
    async fn list_branches(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitBranch>, GitProviderError>;

    /// Get a specific branch by name
    async fn get_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<GitBranch, GitProviderError>;

    /// Create a new branch
    ///
    /// Returns BranchAlreadyExists error if the branch already exists.
    async fn create_branch(
        &self,
        owner: &str,
        repo: &str,
        req: CreateBranchRequest,
    ) -> Result<GitBranch, GitProviderError>;

    /// Delete a branch
    ///
    /// Returns NotFound error if the branch doesn't exist.
    async fn delete_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<(), GitProviderError>;

    // ==================== Issue Operations ====================

    /// List issues in a repository with optional filters
    async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        filter: Option<IssueFilter>,
    ) -> Result<Vec<GitIssue>, GitProviderError>;

    /// Get a specific issue by number
    async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GitIssue, GitProviderError>;

    /// Create a new issue
    async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: CreateIssueRequest,
    ) -> Result<GitIssue, GitProviderError>;

    /// Update an existing issue
    async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: UpdateIssueRequest,
    ) -> Result<GitIssue, GitProviderError>;

    /// Add labels to an issue
    async fn add_issue_labels(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        labels: Vec<String>,
    ) -> Result<Vec<GitLabel>, GitProviderError>;

    /// Remove a label from an issue
    async fn remove_issue_label(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        label: &str,
    ) -> Result<(), GitProviderError>;

    // ==================== Pull Request Operations ====================

    /// List pull requests in a repository with optional filters
    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        filter: Option<PullRequestFilter>,
    ) -> Result<Vec<GitPullRequest>, GitProviderError>;

    /// Get a specific pull request by number
    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GitPullRequest, GitProviderError>;

    /// Create a new pull request
    async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        req: CreatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError>;

    /// Update an existing pull request
    async fn update_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: UpdatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError>;

    /// Merge a pull request
    ///
    /// Returns NotMergeable error if the PR cannot be merged.
    async fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        opts: MergeOptions,
    ) -> Result<(), GitProviderError>;

    // ==================== Label Operations ====================

    /// List all labels in a repository
    async fn list_labels(&self, owner: &str, repo: &str)
        -> Result<Vec<GitLabel>, GitProviderError>;

    /// Create a new label
    ///
    /// Returns LabelAlreadyExists error if the label already exists.
    async fn create_label(
        &self,
        owner: &str,
        repo: &str,
        req: CreateLabelRequest,
    ) -> Result<GitLabel, GitProviderError>;

    /// Delete a label
    ///
    /// Returns NotFound error if the label doesn't exist.
    async fn delete_label(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> Result<(), GitProviderError>;

    // ==================== Provider Info ====================

    /// Get the provider type (e.g., "gitea", "github", "gitlab")
    fn provider_type(&self) -> &'static str;

    /// Get the base URL of the provider
    fn base_url(&self) -> &str;
}
