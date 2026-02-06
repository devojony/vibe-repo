//! PR Creation Service
//!
//! Handles creating pull requests for completed tasks via Git Provider API.

use crate::entities::{
    prelude::*,
    repository,
    task::{self},
};
use crate::error::{Result, VibeRepoError};
use crate::git_provider::{
    models::CreatePullRequestRequest, traits::GitProvider, GitClientFactory,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

/// Service for creating pull requests for completed tasks
#[derive(Clone)]
pub struct PRCreationService {
    db: DatabaseConnection,
}

impl PRCreationService {
    /// Create a new PRCreationService
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create PR for a completed task
    ///
    /// Returns Ok(()) if PR created or already exists
    /// Returns Err if creation fails
    pub async fn create_pr_for_task(&self, task_id: i32) -> Result<()> {
        tracing::info!(task_id, "Creating PR for task");

        // Load task
        let task = Task::find_by_id(task_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound(format!("Task {} not found", task_id)))?;

        // Check if PR already exists
        if self.pr_already_exists(&task).await? {
            tracing::info!(
                task_id,
                pr_number = task.pr_number,
                "PR already exists for task"
            );
            return Ok(());
        }

        // Validate branch_name exists
        let branch_name = task.branch_name.as_ref().ok_or_else(|| {
            VibeRepoError::Validation(format!("Task {} has no branch_name", task_id))
        })?;

        // Load workspace and repository
        let workspace = Workspace::find_by_id(task.workspace_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Workspace {} not found", task.workspace_id))
            })?;

        let repository = Repository::find_by_id(workspace.repository_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Repository {} not found", workspace.repository_id))
            })?;

        // Load provider
        let provider = RepoProvider::find_by_id(repository.provider_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Provider {} not found", repository.provider_id))
            })?;

        // Parse owner/repo from clone_url
        let (owner, repo_name) = self.parse_clone_url(&repository.clone_url)?;

        // Get default branch
        let base_branch = self.get_default_branch(&repository).await?;

        // Create Git Provider client
        let git_client = GitClientFactory::from_provider(&provider)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create git client: {}", e)))?;

        // Build PR body
        let pr_body = self.build_pr_body(task.issue_number, task.issue_body.as_deref());

        // Create PR with retry logic
        let pr = match self
            .create_pr_with_retry(
                &git_client,
                owner,
                repo_name,
                CreatePullRequestRequest {
                    title: task.issue_title.clone(),
                    body: Some(pr_body),
                    head: branch_name.clone(),
                    base: base_branch,
                },
            )
            .await
        {
            Ok(pr) => pr,
            Err(VibeRepoError::NotFound(e)) => {
                // Branch not found - log warning but don't fail the task
                tracing::warn!(
                    task_id,
                    branch = %branch_name,
                    "Branch not found, skipping PR creation: {}",
                    e
                );
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        // Update task with PR information
        let mut task_active: task::ActiveModel = task.into();
        task_active.pr_number = Set(Some(pr.number as i32));

        // Use the web URL returned by the Git provider API
        // This works for all providers (Gitea, GitHub, GitLab) as each provider
        // returns the correct URL format in their API response
        task_active.pr_url = Set(pr.html_url.or_else(|| {
            // Fallback: construct URL manually if API doesn't provide it
            // Note: This fallback uses Gitea's format (/pulls/) and may not work for all providers
            tracing::warn!(
                task_id = task_active.id.as_ref(),
                pr_number = pr.number,
                "PR API response missing html_url, falling back to manual URL construction"
            );
            let web_base_url = provider.base_url.trim_end_matches("/api/v1");
            Some(format!(
                "{}/{}/{}/pulls/{}",
                web_base_url, owner, repo_name, pr.number
            ))
        }));
        task_active.update(&self.db).await?;

        tracing::info!(
            task_id,
            pr_number = pr.number,
            "Successfully created PR for task"
        );

        Ok(())
    }

    /// Check if PR already exists for this task
    async fn pr_already_exists(&self, task: &task::Model) -> Result<bool> {
        Ok(task.pr_number.is_some())
    }

    /// Build PR body with issue reference
    fn build_pr_body(&self, issue_number: i32, additional_context: Option<&str>) -> String {
        let mut body = format!("Closes #{}", issue_number);
        if let Some(context) = additional_context {
            body.push_str("\n\n");
            body.push_str(context);
        }
        body
    }

    /// Get repository default branch
    async fn get_default_branch(&self, repository: &repository::Model) -> Result<String> {
        Ok(repository.default_branch.clone())
    }

    /// Parse owner and repo name from clone_url
    /// Supports both HTTPS and SSH formats:
    /// - https://gitea.com/owner/repo.git
    /// - git@gitea.com:owner/repo.git
    fn parse_clone_url<'a>(&self, clone_url: &'a str) -> Result<(&'a str, &'a str)> {
        // Remove .git suffix if present
        let url = clone_url.strip_suffix(".git").unwrap_or(clone_url);

        // Try HTTPS format first: https://gitea.com/owner/repo
        if let Some(path) = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
        {
            // Find the first slash after the domain
            if let Some(slash_pos) = path.find('/') {
                let repo_path = &path[slash_pos + 1..];
                let parts: Vec<&str> = repo_path.split('/').collect();
                if parts.len() >= 2 {
                    return Ok((parts[0], parts[1]));
                }
            }
        }

        // Try SSH format: git@gitea.com:owner/repo
        if let Some(path) = url.strip_prefix("git@") {
            if let Some(colon_pos) = path.find(':') {
                let repo_path = &path[colon_pos + 1..];
                let parts: Vec<&str> = repo_path.split('/').collect();
                if parts.len() >= 2 {
                    return Ok((parts[0], parts[1]));
                }
            }
        }

        Err(VibeRepoError::Internal(format!(
            "Invalid repository clone_url format: {}",
            clone_url
        )))
    }

    /// Create PR with retry logic for network errors
    async fn create_pr_with_retry(
        &self,
        git_client: &crate::git_provider::GitClient,
        owner: &str,
        repo: &str,
        request: CreatePullRequestRequest,
    ) -> Result<crate::git_provider::models::GitPullRequest> {
        use crate::git_provider::error::GitProviderError;

        let mut retries = 0;
        let max_retries = 3;

        loop {
            match git_client
                .create_pull_request(owner, repo, request.clone())
                .await
            {
                Ok(pr) => return Ok(pr),
                Err(GitProviderError::NetworkError(e)) if retries < max_retries => {
                    retries += 1;
                    tracing::warn!(
                        owner,
                        repo,
                        head = %request.head,
                        retry = retries,
                        max_retries,
                        error = %e,
                        "Network error creating PR, retrying"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(1 << retries)).await;
                }
                Err(GitProviderError::Conflict(e)) => {
                    // PR already exists - this is OK, try to fetch it
                    tracing::info!(owner, repo, head = %request.head, "PR already exists: {}", e);

                    // Try to fetch the existing PR by listing PRs and filtering by branch
                    // Note: This only fetches the first page of PRs. If the PR is not in the
                    // first page (due to pagination), we'll return an error. This is acceptable
                    // because the PR was successfully created (just not by us).
                    match git_client.list_pull_requests(owner, repo, None).await {
                        Ok(prs) => {
                            // Find PR with matching source branch
                            if let Some(existing_pr) =
                                prs.iter().find(|pr| pr.source_branch == request.head)
                            {
                                tracing::info!(
                                    owner,
                                    repo,
                                    head = %request.head,
                                    pr_number = existing_pr.number,
                                    "Found existing PR"
                                );
                                return Ok(existing_pr.clone());
                            } else {
                                tracing::warn!(
                                    owner,
                                    repo,
                                    head = %request.head,
                                    "PR exists but couldn't find it in first page (may be paginated)"
                                );
                                // PR exists but not in first page - this is a conflict error
                                // The task should handle this gracefully (PR was created, just not by us)
                                return Err(VibeRepoError::Conflict(format!(
                                    "PR already exists for branch {} but couldn't fetch details (may be paginated)",
                                    request.head
                                )));
                            }
                        }
                        Err(list_err) => {
                            tracing::warn!(
                                owner,
                                repo,
                                head = %request.head,
                                error = %list_err,
                                "Failed to list PRs after conflict"
                            );
                            // PR exists but we couldn't list PRs - this is a conflict error
                            // The task should handle this gracefully (PR was created, just not by us)
                            return Err(VibeRepoError::Conflict(format!(
                                "PR already exists for branch {} but couldn't fetch details: {}",
                                request.head, list_err
                            )));
                        }
                    }
                }
                Err(GitProviderError::NotFound(e)) => {
                    // Branch not found - return error to be handled by caller
                    tracing::warn!(owner, repo, head = %request.head, "Branch not found: {}", e);
                    return Err(VibeRepoError::NotFound(format!(
                        "Branch {} not found",
                        request.head
                    )));
                }
                Err(e) => {
                    tracing::error!(owner, repo, head = %request.head, error = %e, "Failed to create PR");
                    return Err(VibeRepoError::GitProvider(e));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{repo_provider, repository, task::TaskStatus, workspace};
    use crate::test_utils::db::TestDatabase;

    /// Test create_pr_for_task_success
    /// Requirements: PR Creation - create PR for completed task
    /// Note: This test requires a real Git provider and is skipped in unit tests
    /// It should be run as an integration test with a mock or real Git provider
    #[tokio::test]
    #[ignore] // Requires real Git provider - run as integration test
    async fn test_create_pr_for_task_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test data
        let (task, _workspace, _repository, _provider) = create_test_task_with_branch(db).await;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.create_pr_for_task(task.id).await;

        // Assert
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);

        // Verify task was updated with PR info
        let updated_task = Task::find_by_id(task.id).one(db).await.unwrap().unwrap();
        assert!(updated_task.pr_number.is_some());
        assert!(updated_task.pr_url.is_some());
    }

    /// Test create_pr_skips_if_already_exists
    /// Requirements: PR Creation - idempotent operation
    #[tokio::test]
    async fn test_create_pr_skips_if_already_exists() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let (mut task, _workspace, _repository, _provider) = create_test_task_with_branch(db).await;

        // Set PR info to simulate already created PR
        let mut task_active: task::ActiveModel = task.clone().into();
        task_active.pr_number = Set(Some(123));
        task_active.pr_url = Set(Some("https://example.com/pr/123".to_string()));
        task = task_active.update(db).await.unwrap();

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.create_pr_for_task(task.id).await;

        // Assert
        assert!(result.is_ok());

        // Verify PR info unchanged
        let updated_task = Task::find_by_id(task.id).one(db).await.unwrap().unwrap();
        assert_eq!(updated_task.pr_number, Some(123));
        assert_eq!(
            updated_task.pr_url,
            Some("https://example.com/pr/123".to_string())
        );
    }

    /// Test create_pr_fails_if_no_branch_name
    /// Requirements: PR Creation - validation
    #[tokio::test]
    async fn test_create_pr_fails_if_no_branch_name() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let (task, _workspace, _repository, _provider) = create_test_task_without_branch(db).await;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.create_pr_for_task(task.id).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Validation(msg) => {
                assert!(msg.contains("no branch_name"));
            }
            e => panic!("Expected Validation error, got: {:?}", e),
        }
    }

    /// Test create_pr_builds_correct_body
    /// Requirements: PR Creation - PR body format
    #[tokio::test]
    async fn test_create_pr_builds_correct_body() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let service = PRCreationService::new(db.clone());

        // Act - without additional context
        let body1 = service.build_pr_body(123, None);

        // Assert
        assert_eq!(body1, "Closes #123");

        // Act - with additional context
        let body2 = service.build_pr_body(456, Some("Additional context here"));

        // Assert
        assert_eq!(body2, "Closes #456\n\nAdditional context here");
    }

    /// Test pr_already_exists returns true when pr_number is set
    #[tokio::test]
    async fn test_pr_already_exists_returns_true() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let (mut task, _workspace, _repository, _provider) = create_test_task_with_branch(db).await;

        let mut task_active: task::ActiveModel = task.clone().into();
        task_active.pr_number = Set(Some(123));
        task = task_active.update(db).await.unwrap();

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.pr_already_exists(&task).await;

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    /// Test pr_already_exists returns false when pr_number is None
    #[tokio::test]
    async fn test_pr_already_exists_returns_false() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let (task, _workspace, _repository, _provider) = create_test_task_with_branch(db).await;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.pr_already_exists(&task).await;

        // Assert
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    /// Test get_default_branch returns repository default branch
    #[tokio::test]
    async fn test_get_default_branch_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let (_task, _workspace, repository, _provider) = create_test_task_with_branch(db).await;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.get_default_branch(&repository).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "main");
    }

    /// Test parse_clone_url parses HTTPS URLs correctly
    #[tokio::test]
    async fn test_parse_clone_url_https() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let service = PRCreationService::new(db.clone());

        // Act & Assert - with .git suffix
        let result = service.parse_clone_url("https://gitea.com/owner/repo.git");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");

        // Act & Assert - without .git suffix
        let result = service.parse_clone_url("https://gitea.com/owner/repo");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");

        // Act & Assert - http protocol
        let result = service.parse_clone_url("http://gitea.com/owner/repo.git");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    /// Test parse_clone_url parses SSH URLs correctly
    #[tokio::test]
    async fn test_parse_clone_url_ssh() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let service = PRCreationService::new(db.clone());

        // Act & Assert - with .git suffix
        let result = service.parse_clone_url("git@gitea.com:owner/repo.git");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");

        // Act & Assert - without .git suffix
        let result = service.parse_clone_url("git@gitea.com:owner/repo");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    /// Test parse_clone_url fails with invalid format
    #[tokio::test]
    async fn test_parse_clone_url_invalid_format() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.parse_clone_url("invalid-url");

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Internal(msg) => {
                assert!(msg.contains("Invalid repository clone_url format"));
            }
            e => panic!("Expected Internal error, got: {:?}", e),
        }
    }

    /// Test create_pr_handles_network_errors
    /// Requirements: PR Creation - retry logic for network errors
    ///
    /// TODO: This test requires a mock Git provider implementation to properly test
    /// the retry logic in create_pr_with_retry (lines 200-296).
    ///
    /// What should be tested:
    /// - Network errors trigger retry logic (up to 3 retries)
    /// - Exponential backoff is applied between retries (2^retry seconds)
    /// - Success after retry returns the PR successfully
    /// - Failure after max retries returns the network error
    ///
    /// Why it's not tested now:
    /// - GitClient is an enum wrapping real provider implementations (Gitea, GitHub, GitLab)
    /// - No mocking infrastructure exists for Git provider trait
    /// - Creating a mock would require significant refactoring to support dependency injection
    ///
    /// How it should be tested in the future:
    /// - Option 1: Add a MockGitProvider that implements GitProvider trait
    ///   - Allow injecting mock into PRCreationService via constructor
    ///   - Mock can simulate network errors and track retry attempts
    /// - Option 2: Integration tests with a test Git provider server
    ///   - Use wiremock or similar to simulate network failures
    ///   - Verify retry behavior with real HTTP requests
    /// - Option 3: Extract retry logic into a separate testable function
    ///   - Create a generic retry wrapper that can be unit tested
    ///   - Apply wrapper to Git provider calls
    ///
    /// Related code:
    /// - create_pr_with_retry method (lines 200-296)
    /// - GitProviderError::NetworkError handling (line 219)
    /// - Exponential backoff implementation (line 230)
    #[tokio::test]
    #[ignore] // Requires mock Git provider infrastructure
    async fn test_create_pr_handles_network_errors() {
        // This test is intentionally left unimplemented pending mock infrastructure
        // See TODO comment above for implementation plan
    }

    // Helper functions

    async fn create_test_task_with_branch(
        db: &DatabaseConnection,
    ) -> (
        task::Model,
        workspace::Model,
        repository::Model,
        repo_provider::Model,
    ) {
        let (workspace, repository, provider) = create_test_workspace(db).await;

        let task = task::ActiveModel {
            workspace_id: Set(workspace.id),
            issue_number: Set(123),
            issue_title: Set("Test Issue".to_string()),
            issue_body: Set(Some("Test issue body".to_string())),
            task_status: Set(TaskStatus::Completed),
            priority: Set("high".to_string()),
            branch_name: Set(Some("feature/test-branch".to_string())),
            ..Default::default()
        };

        let task = Task::insert(task)
            .exec_with_returning(db)
            .await
            .expect("Failed to create test task");

        (task, workspace, repository, provider)
    }

    async fn create_test_task_without_branch(
        db: &DatabaseConnection,
    ) -> (
        task::Model,
        workspace::Model,
        repository::Model,
        repo_provider::Model,
    ) {
        let (workspace, repository, provider) = create_test_workspace(db).await;

        let task = task::ActiveModel {
            workspace_id: Set(workspace.id),
            issue_number: Set(456),
            issue_title: Set("Test Issue Without Branch".to_string()),
            issue_body: Set(None),
            task_status: Set(TaskStatus::Pending),
            priority: Set("medium".to_string()),
            branch_name: Set(None), // No branch name
            ..Default::default()
        };

        let task = Task::insert(task)
            .exec_with_returning(db)
            .await
            .expect("Failed to create test task");

        (task, workspace, repository, provider)
    }

    async fn create_test_workspace(
        db: &DatabaseConnection,
    ) -> (workspace::Model, repository::Model, repo_provider::Model) {
        // Create provider
        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec(db)
            .await
            .expect("Failed to create provider");
        let provider = RepoProvider::find_by_id(provider.last_insert_id)
            .one(db)
            .await
            .expect("Failed to fetch provider")
            .expect("Provider not found");

        // Create repository
        let repository = repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.id),
            ..Default::default()
        };
        let repository = Repository::insert(repository)
            .exec_with_returning(db)
            .await
            .expect("Failed to create repository");

        // Create workspace
        let workspace = workspace::ActiveModel {
            repository_id: Set(repository.id),
            ..Default::default()
        };
        let workspace = Workspace::insert(workspace)
            .exec_with_returning(db)
            .await
            .expect("Failed to create workspace");

        (workspace, repository, provider)
    }
}
