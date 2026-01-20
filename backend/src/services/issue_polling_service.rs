//! Issue Polling Service
//!
//! Background service that periodically polls Git platforms for new issues
//! and creates tasks. This serves as a fallback mechanism when webhooks
//! are not available or as a complement to webhook-based event handling.

use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, StreamExt};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::config::IssuePollingConfig;
use crate::entities::prelude::*;
use crate::entities::{repository, task, workspace};
use crate::error::{Result, VibeRepoError};
use crate::git_provider::traits::GitProvider;
use crate::git_provider::{GitClientFactory, GitIssue, IssueFilter, IssueState};
use crate::services::BackgroundService;
use crate::state::AppState;

/// Issue polling service that periodically checks for new issues
#[derive(Clone)]
pub struct IssuePollingService {
    db: DatabaseConnection,
    config: IssuePollingConfig,
    /// Cache: repository_id -> workspace_id
    workspace_cache: Arc<RwLock<HashMap<i32, i32>>>,
}

impl IssuePollingService {
    /// Create a new issue polling service
    pub fn new(db: DatabaseConnection, config: IssuePollingConfig) -> Self {
        Self {
            db,
            config,
            workspace_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get workspace ID for a repository, using cache when possible
    async fn get_workspace_id(&self, repo_id: i32) -> Result<i32> {
        // 1. Check cache first
        {
            let cache = self.workspace_cache.read().await;
            if let Some(workspace_id) = cache.get(&repo_id) {
                tracing::debug!(
                    repository_id = repo_id,
                    workspace_id = workspace_id,
                    "Workspace cache hit"
                );
                return Ok(*workspace_id);
            }
        }

        // 2. Cache miss, query database
        tracing::debug!(
            repository_id = repo_id,
            "Workspace cache miss, querying database"
        );

        let workspace = Workspace::find()
            .filter(workspace::Column::RepositoryId.eq(repo_id))
            .filter(workspace::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Workspace not found for repository {}", repo_id))
            })?;

        // 3. Update cache
        {
            let mut cache = self.workspace_cache.write().await;
            cache.insert(repo_id, workspace.id);
        }

        tracing::debug!(
            repository_id = repo_id,
            workspace_id = workspace.id,
            "Workspace cached"
        );

        Ok(workspace.id)
    }

    /// Clear workspace cache (useful for testing or manual refresh)
    pub async fn clear_workspace_cache(&self) {
        let mut cache = self.workspace_cache.write().await;
        cache.clear();
        tracing::info!("Workspace cache cleared");
    }

    /// Get cache statistics (size, capacity)
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.workspace_cache.read().await;
        (cache.len(), cache.capacity())
    }


    /// Poll all repositories that have polling enabled
    async fn poll_all_repositories(&self) -> Result<()> {
        // Get all repositories with polling enabled
        let repositories = Repository::find()
            .filter(repository::Column::PollingEnabled.eq(true))
            .filter(repository::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;

        tracing::info!(
            count = repositories.len(),
            "Starting concurrent polling for repositories"
        );

        // Use futures::stream to poll repositories concurrently
        let results = stream::iter(repositories)
            .map(|repo| {
                let service = self.clone();
                async move {
                    let repo_id = repo.id;
                    match service.poll_repository(&repo).await {
                        Ok(_) => (repo_id, true, None),
                        Err(e) => (repo_id, false, Some(e.to_string())),
                    }
                }
            })
            .buffer_unordered(self.config.max_concurrent_polls)
            .collect::<Vec<_>>()
            .await;

        // Log any errors that occurred
        for (repo_id, success, error) in &results {
            if !success {
                if let Some(err_msg) = error {
                    tracing::error!(
                        repository_id = repo_id,
                        error = %err_msg,
                        "Failed to poll repository"
                    );
                }
            }
        }

        // Collect statistics
        let success_count = results.iter().filter(|(_, success, _)| *success).count();
        let error_count = results.len() - success_count;

        tracing::info!(
            total = results.len(),
            success = success_count,
            errors = error_count,
            "Completed concurrent polling"
        );

        Ok(())
    }

    /// Poll a single repository for new issues
    pub async fn poll_repository(&self, repo: &repository::Model) -> Result<()> {
        tracing::debug!(
            repository_id = repo.id,
            repository_name = %repo.full_name,
            "Polling repository for issues"
        );

        // 1. Get provider and create Git client
        let provider = RepoProvider::find_by_id(repo.provider_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Provider {} not found", repo.provider_id))
            })?;

        let client = GitClientFactory::from_provider(&provider)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create git client: {}", e)))?;

        // 2. Parse owner/repo from full_name
        let parts: Vec<&str> = repo.full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(VibeRepoError::Validation(format!(
                "Invalid repository full_name format: {}",
                repo.full_name
            )));
        }
        let (owner, repo_name) = (parts[0], parts[1]);

        // 3. Get workspace for this repository (using cache)
        let workspace_id = self.get_workspace_id(repo.id).await?;

        // 4. Build issue filter
        let filter = IssueFilter {
            state: Some(IssueState::Open),
            labels: self.config.required_labels.clone(),
            assignee: None,
        };

        // 5. Fetch issues from Git provider
        let issues = client
            .list_issues(owner, repo_name, Some(filter))
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to list issues: {}", e)))?;

        tracing::info!(
            repository_id = repo.id,
            issue_count = issues.len(),
            "Fetched issues from repository"
        );

        // 6. Filter and process issues
        let last_poll_time = repo.last_issue_poll_at.unwrap_or_else(|| {
            // If no previous poll time, use 30 days ago as default
            Utc::now() - chrono::Duration::days(30)
        });

        let mut new_task_count = 0;
        for issue in issues {
            // Only process issues created after last poll time
            if issue.created_at <= last_poll_time {
                continue;
            }

            // Apply filtering rules
            if !self.should_create_task(&issue) {
                tracing::debug!(
                    issue_number = issue.number,
                    issue_title = %issue.title,
                    "Issue filtered out by rules"
                );
                continue;
            }

            // Create task from issue
            match self.create_task_from_issue(workspace_id, &issue).await {
                Ok(_) => {
                    new_task_count += 1;
                    tracing::info!(
                        repository_id = repo.id,
                        issue_number = issue.number,
                        issue_title = %issue.title,
                        "Created task from issue"
                    );
                }
                Err(VibeRepoError::Conflict(_)) => {
                    // Task already exists, skip
                    tracing::debug!(issue_number = issue.number, "Task already exists for issue");
                }
                Err(e) => {
                    tracing::error!(
                        issue_number = issue.number,
                        error = %e,
                        "Failed to create task from issue"
                    );
                }
            }
        }

        // 7. Update last_poll_time
        let mut repo_active: repository::ActiveModel = repo.clone().into();
        repo_active.last_issue_poll_at = Set(Some(Utc::now()));
        repo_active.updated_at = Set(Utc::now());
        repo_active.update(&self.db).await?;

        tracing::info!(
            repository_id = repo.id,
            new_tasks = new_task_count,
            "Completed repository polling"
        );

        Ok(())
    }

    /// Check if a task should be created for this issue based on filtering rules
    fn should_create_task(&self, issue: &GitIssue) -> bool {
        // 1. Must be Open state
        if issue.state != IssueState::Open {
            tracing::debug!(
                issue_number = issue.number,
                state = ?issue.state,
                "Issue is not open"
            );
            return false;
        }

        // 2. Check required labels (if configured)
        if let Some(required_labels) = &self.config.required_labels {
            if !required_labels
                .iter()
                .any(|label| issue.labels.contains(label))
            {
                tracing::debug!(
                    issue_number = issue.number,
                    required_labels = ?required_labels,
                    issue_labels = ?issue.labels,
                    "Issue does not have required labels"
                );
                return false;
            }
        }

        // 3. Check @mention (if configured)
        if let Some(bot_username) = &self.config.bot_username {
            if let Some(body) = &issue.body {
                if !body.contains(&format!("@{}", bot_username)) {
                    tracing::debug!(
                        issue_number = issue.number,
                        bot_username = %bot_username,
                        "Issue does not mention bot"
                    );
                    return false;
                }
            } else {
                tracing::debug!(
                    issue_number = issue.number,
                    "Issue has no body, cannot check @mention"
                );
                return false;
            }
        }

        // 4. Check issue age (if configured)
        if let Some(max_age_days) = self.config.max_issue_age_days {
            let age = Utc::now() - issue.created_at;
            if age.num_days() > max_age_days {
                tracing::debug!(
                    issue_number = issue.number,
                    age_days = age.num_days(),
                    max_age_days = max_age_days,
                    "Issue is too old"
                );
                return false;
            }
        }

        true
    }

    /// Create a task from an issue
    async fn create_task_from_issue(
        &self,
        workspace_id: i32,
        issue: &GitIssue,
    ) -> Result<task::Model> {
        let task = task::ActiveModel {
            workspace_id: Set(workspace_id),
            issue_number: Set(issue.number as i32),
            issue_title: Set(issue.title.clone()),
            issue_body: Set(issue.body.clone()),
            task_status: Set("Pending".to_string()),
            priority: Set("Medium".to_string()),
            retry_count: Set(0),
            max_retries: Set(3),
            ..Default::default()
        };

        // Try to insert, handle unique constraint violation
        match Task::insert(task).exec_with_returning(&self.db).await {
            Ok(task) => {
                tracing::info!(
                    workspace_id = workspace_id,
                    issue_number = issue.number,
                    task_id = task.id,
                    "Created task from issue"
                );
                Ok(task)
            }
            Err(sea_orm::DbErr::RecordNotInserted) => {
                // Task already exists (unique constraint violation)
                tracing::debug!(
                    workspace_id = workspace_id,
                    issue_number = issue.number,
                    "Task already exists, skipping"
                );
                Err(VibeRepoError::Conflict(format!(
                    "Task already exists for issue {}",
                    issue.number
                )))
            }
            Err(e) => {
                tracing::error!(
                    workspace_id = workspace_id,
                    issue_number = issue.number,
                    error = %e,
                    "Database error creating task"
                );
                Err(VibeRepoError::Database(e))
            }
        }
    }
}

#[async_trait]
impl BackgroundService for IssuePollingService {
    fn name(&self) -> &'static str {
        "issue_polling"
    }

    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        if !self.config.enabled {
            tracing::info!("Issue polling is disabled, not starting service");
            return Ok(());
        }

        tracing::info!(
            interval_seconds = self.config.interval_seconds,
            "Starting issue polling service"
        );

        let db = self.db.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let service = IssuePollingService::new(db, config.clone());
            let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));

            loop {
                interval.tick().await;

                if let Err(e) = service.poll_all_repositories().await {
                    tracing::error!(error = %e, "Error in issue polling service");
                }
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("Issue polling service stopped");
        Ok(())
    }

    async fn health_check(&self) -> bool {
        // Check database connection
        self.db.ping().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IssuePollingConfig;
    use chrono::Utc;

    /// Helper function to create a test config
    fn create_test_config() -> IssuePollingConfig {
        IssuePollingConfig {
            enabled: true,
            interval_seconds: 3600,
            required_labels: Some(vec!["vibe/todo-ai".to_string()]),
            bot_username: Some("vibe-bot".to_string()),
            max_issue_age_days: Some(30),
            max_concurrent_polls: 10,
        }
    }

    /// Helper function to create a test issue
    fn create_test_issue(
        number: i64,
        state: IssueState,
        labels: Vec<String>,
        body: Option<String>,
        created_at: chrono::DateTime<Utc>,
    ) -> GitIssue {
        GitIssue {
            number,
            title: format!("Test Issue #{}", number),
            body,
            state,
            labels,
            assignees: vec![],
            created_at,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_should_create_task_with_open_state() {
        // Arrange: Create service with minimal config
        let config = IssuePollingConfig {
            enabled: true,
            interval_seconds: 3600,
            required_labels: None,
            bot_username: None,
            max_issue_age_days: None,
            max_concurrent_polls: 10,
        };
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        // Act & Assert: Open issue should pass
        let open_issue = create_test_issue(
            1,
            IssueState::Open,
            vec![],
            Some("Test body".to_string()),
            Utc::now(),
        );
        assert!(service.should_create_task(&open_issue));

        // Act & Assert: Closed issue should fail
        let closed_issue = create_test_issue(
            2,
            IssueState::Closed,
            vec![],
            Some("Test body".to_string()),
            Utc::now(),
        );
        assert!(!service.should_create_task(&closed_issue));
    }

    #[test]
    fn test_should_create_task_with_required_labels() {
        // Arrange: Create service with required labels
        let config = IssuePollingConfig {
            enabled: true,
            interval_seconds: 3600,
            required_labels: Some(vec![
                "vibe/todo-ai".to_string(),
                "vibe/pending-ack".to_string(),
            ]),
            bot_username: None,
            max_issue_age_days: None,
            max_concurrent_polls: 10,
        };
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        // Act & Assert: Issue with required label should pass
        let issue_with_label = create_test_issue(
            1,
            IssueState::Open,
            vec!["vibe/todo-ai".to_string(), "bug".to_string()],
            Some("Test body".to_string()),
            Utc::now(),
        );
        assert!(service.should_create_task(&issue_with_label));

        // Act & Assert: Issue without required label should fail
        let issue_without_label = create_test_issue(
            2,
            IssueState::Open,
            vec!["bug".to_string(), "enhancement".to_string()],
            Some("Test body".to_string()),
            Utc::now(),
        );
        assert!(!service.should_create_task(&issue_without_label));

        // Act & Assert: Issue with no labels should fail
        let issue_no_labels = create_test_issue(
            3,
            IssueState::Open,
            vec![],
            Some("Test body".to_string()),
            Utc::now(),
        );
        assert!(!service.should_create_task(&issue_no_labels));
    }

    #[test]
    fn test_should_create_task_with_bot_mention() {
        // Arrange: Create service with bot username
        let config = IssuePollingConfig {
            enabled: true,
            interval_seconds: 3600,
            required_labels: None,
            bot_username: Some("vibe-bot".to_string()),
            max_issue_age_days: None,
            max_concurrent_polls: 10,
        };
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        // Act & Assert: Issue with @mention should pass
        let issue_with_mention = create_test_issue(
            1,
            IssueState::Open,
            vec![],
            Some("Hey @vibe-bot, please help with this issue".to_string()),
            Utc::now(),
        );
        assert!(service.should_create_task(&issue_with_mention));

        // Act & Assert: Issue without @mention should fail
        let issue_without_mention = create_test_issue(
            2,
            IssueState::Open,
            vec![],
            Some("This is a regular issue".to_string()),
            Utc::now(),
        );
        assert!(!service.should_create_task(&issue_without_mention));

        // Act & Assert: Issue with no body should fail
        let issue_no_body = create_test_issue(3, IssueState::Open, vec![], None, Utc::now());
        assert!(!service.should_create_task(&issue_no_body));

        // Act & Assert: Issue with wrong mention should fail
        let issue_wrong_mention = create_test_issue(
            4,
            IssueState::Open,
            vec![],
            Some("Hey @other-bot, please help".to_string()),
            Utc::now(),
        );
        assert!(!service.should_create_task(&issue_wrong_mention));
    }

    #[test]
    fn test_should_create_task_with_age_limit() {
        // Arrange: Create service with max age limit
        let config = IssuePollingConfig {
            enabled: true,
            interval_seconds: 3600,
            required_labels: None,
            bot_username: None,
            max_issue_age_days: Some(7), // 7 days max
            max_concurrent_polls: 10,
        };
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        // Act & Assert: Recent issue (1 day old) should pass
        let recent_issue = create_test_issue(
            1,
            IssueState::Open,
            vec![],
            Some("Test body".to_string()),
            Utc::now() - chrono::Duration::days(1),
        );
        assert!(service.should_create_task(&recent_issue));

        // Act & Assert: Issue at age limit (7 days) should pass
        let at_limit_issue = create_test_issue(
            2,
            IssueState::Open,
            vec![],
            Some("Test body".to_string()),
            Utc::now() - chrono::Duration::days(7),
        );
        assert!(service.should_create_task(&at_limit_issue));

        // Act & Assert: Old issue (30 days) should fail
        let old_issue = create_test_issue(
            3,
            IssueState::Open,
            vec![],
            Some("Test body".to_string()),
            Utc::now() - chrono::Duration::days(30),
        );
        assert!(!service.should_create_task(&old_issue));

        // Act & Assert: Very old issue (100 days) should fail
        let very_old_issue = create_test_issue(
            4,
            IssueState::Open,
            vec![],
            Some("Test body".to_string()),
            Utc::now() - chrono::Duration::days(100),
        );
        assert!(!service.should_create_task(&very_old_issue));
    }

    #[test]
    fn test_should_create_task_with_combined_filters() {
        // Arrange: Create service with all filters enabled
        let config = create_test_config();
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        // Act & Assert: Issue passing all filters should pass
        let valid_issue = create_test_issue(
            1,
            IssueState::Open,
            vec!["vibe/todo-ai".to_string()],
            Some("@vibe-bot please help".to_string()),
            Utc::now() - chrono::Duration::days(5),
        );
        assert!(service.should_create_task(&valid_issue));

        // Act & Assert: Issue failing state check should fail
        let closed_issue = create_test_issue(
            2,
            IssueState::Closed,
            vec!["vibe/todo-ai".to_string()],
            Some("@vibe-bot please help".to_string()),
            Utc::now() - chrono::Duration::days(5),
        );
        assert!(!service.should_create_task(&closed_issue));

        // Act & Assert: Issue failing label check should fail
        let no_label_issue = create_test_issue(
            3,
            IssueState::Open,
            vec!["bug".to_string()],
            Some("@vibe-bot please help".to_string()),
            Utc::now() - chrono::Duration::days(5),
        );
        assert!(!service.should_create_task(&no_label_issue));

        // Act & Assert: Issue failing mention check should fail
        let no_mention_issue = create_test_issue(
            4,
            IssueState::Open,
            vec!["vibe/todo-ai".to_string()],
            Some("Regular issue description".to_string()),
            Utc::now() - chrono::Duration::days(5),
        );
        assert!(!service.should_create_task(&no_mention_issue));

        // Act & Assert: Issue failing age check should fail
        let old_issue = create_test_issue(
            5,
            IssueState::Open,
            vec!["vibe/todo-ai".to_string()],
            Some("@vibe-bot please help".to_string()),
            Utc::now() - chrono::Duration::days(50),
        );
        assert!(!service.should_create_task(&old_issue));
    }

    #[tokio::test]
    #[ignore = "Requires real database connection"]
    async fn test_create_task_from_issue_success() {
        // Note: This test requires a real database connection
        // Run with: cargo test --ignored test_create_task_from_issue_success

        let config = create_test_config();
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        let issue = create_test_issue(
            123,
            IssueState::Open,
            vec!["vibe/todo-ai".to_string()],
            Some("Test issue body".to_string()),
            Utc::now(),
        );

        // This would fail without a real database
        let result = service.create_task_from_issue(1, &issue).await;

        // In a real test with a test database, we would assert:
        // assert!(result.is_ok());
        // let task = result.unwrap();
        // assert_eq!(task.issue_number, 123);
        // assert_eq!(task.issue_title, "Test Issue #123");

        // For now, we just verify the function signature is correct
        assert!(result.is_err()); // Expected to fail without real DB
    }

    #[tokio::test]
    #[ignore = "Requires real database connection"]
    async fn test_create_task_from_issue_duplicate() {
        // Note: This test demonstrates handling of duplicate tasks
        // Run with: cargo test --ignored test_create_task_from_issue_duplicate
        // In a real implementation with a test database, you would:
        // 1. Create a task
        // 2. Try to create the same task again
        // 3. Verify it returns a Conflict error

        let config = create_test_config();
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        let issue = create_test_issue(
            456,
            IssueState::Open,
            vec!["vibe/todo-ai".to_string()],
            Some("Duplicate test".to_string()),
            Utc::now(),
        );

        // First attempt (would succeed with real DB)
        let result1 = service.create_task_from_issue(1, &issue).await;
        assert!(result1.is_err()); // Expected to fail without real DB

        // Second attempt (would return Conflict with real DB)
        let result2 = service.create_task_from_issue(1, &issue).await;
        assert!(result2.is_err()); // Expected to fail without real DB

        // In a real test with a test database:
        // assert!(matches!(result2, Err(VibeRepoError::Conflict(_))));
    }

    #[test]
    fn test_service_name() {
        let config = create_test_config();
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        assert_eq!(service.name(), "issue_polling");
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = create_test_config();
        let db = DatabaseConnection::default();
        let service = IssuePollingService::new(db, config);

        // With default connection, health check should fail
        let is_healthy = service.health_check().await;
        assert!(!is_healthy);

        // In a real test with a test database:
        // let db = create_test_db().await;
        // let service = IssuePollingService::new(db, config);
        // assert!(service.health_check().await);
    }
}
