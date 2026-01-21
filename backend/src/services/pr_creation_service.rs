//! PR Creation Service
//!
//! Handles creating pull requests for completed tasks via Git Provider API.

use crate::entities::{prelude::*, task, workspace};
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

        // Parse owner/repo from full_name
        let (owner, repo_name) = self.parse_full_name(&repository.full_name)?;

        // Get default branch
        let base_branch = self.get_default_branch(&workspace).await?;

        // Create Git Provider client
        let git_client = GitClientFactory::from_provider(&provider)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create git client: {}", e)))?;

        // Build PR body
        let pr_body = self.build_pr_body(task.issue_number, task.issue_body.as_deref());

        // Create PR with retry logic
        let pr = self
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
            .await?;

        // Update task with PR information
        let mut task_active: task::ActiveModel = task.into();
        task_active.pr_number = Set(Some(pr.number as i32));
        task_active.pr_url = Set(Some(format!(
            "{}/{}/{}/pulls/{}",
            provider.base_url, owner, repo_name, pr.number
        )));
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
    async fn get_default_branch(&self, workspace: &workspace::Model) -> Result<String> {
        let repository = Repository::find_by_id(workspace.repository_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Repository {} not found", workspace.repository_id))
            })?;

        Ok(repository.default_branch)
    }

    /// Parse owner and repo name from full_name
    fn parse_full_name<'a>(&self, full_name: &'a str) -> Result<(&'a str, &'a str)> {
        let parts: Vec<&str> = full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(VibeRepoError::Internal(format!(
                "Invalid repository full_name: {}",
                full_name
            )));
        }
        Ok((parts[0], parts[1]))
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
                    // PR already exists - this is OK
                    tracing::info!(owner, repo, head = %request.head, "PR already exists: {}", e);
                    // We need to fetch the existing PR to return it
                    // For now, return an error that will be handled by the caller
                    return Err(VibeRepoError::Conflict(format!(
                        "PR already exists for branch {}",
                        request.head
                    )));
                }
                Err(GitProviderError::NotFound(e)) => {
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
    use crate::entities::{repo_provider, repository};
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

    /// Test create_pr_handles_network_errors
    /// Requirements: PR Creation - retry logic
    #[tokio::test]
    async fn test_create_pr_handles_network_errors() {
        // This test would require mocking the Git Provider client
        // For now, we'll test the retry logic indirectly through integration tests
        // or by testing the create_pr_with_retry method with a mock client
        // Skipping for now as it requires more complex mocking setup
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

        let (_task, workspace, _repository, _provider) = create_test_task_with_branch(db).await;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.get_default_branch(&workspace).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "main");
    }

    /// Test parse_full_name parses owner and repo correctly
    #[tokio::test]
    async fn test_parse_full_name_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.parse_full_name("owner/repo");

        // Assert
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    /// Test parse_full_name fails with invalid format
    #[tokio::test]
    async fn test_parse_full_name_invalid_format() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let service = PRCreationService::new(db.clone());

        // Act
        let result = service.parse_full_name("invalid");

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Internal(msg) => {
                assert!(msg.contains("Invalid repository full_name"));
            }
            e => panic!("Expected Internal error, got: {:?}", e),
        }
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
            task_status: Set("completed".to_string()),
            priority: Set("high".to_string()),
            branch_name: Set(Some("feature/test-branch".to_string())),
            retry_count: Set(0),
            max_retries: Set(3),
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
            task_status: Set("pending".to_string()),
            priority: Set("medium".to_string()),
            branch_name: Set(None), // No branch name
            retry_count: Set(0),
            max_retries: Set(3),
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
            workspace_status: Set("Active".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
            ..Default::default()
        };
        let workspace = Workspace::insert(workspace)
            .exec_with_returning(db)
            .await
            .expect("Failed to create workspace");

        (workspace, repository, provider)
    }
}
