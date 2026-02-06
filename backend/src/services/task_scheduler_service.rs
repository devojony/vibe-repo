//! Task Scheduler Service
//!
//! Background service that automatically executes pending tasks based on priority.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info};

use crate::entities::{
    prelude::*,
    task::{self, TaskStatus},
    workspace,
};
use crate::error::{Result, VibeRepoError};
use crate::services::{BackgroundService, TaskExecutorService};
use crate::state::AppState;

/// Task scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Polling interval in seconds (default: 30)
    pub polling_interval_seconds: u64,
    /// Whether the scheduler is enabled (default: true)
    pub enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            polling_interval_seconds: 30,
            enabled: true,
        }
    }
}

/// Task scheduler service
pub struct TaskSchedulerService {
    db: DatabaseConnection,
    config: SchedulerConfig,
    running: Arc<RwLock<bool>>,
    workspace_base_dir: String,
}

impl TaskSchedulerService {
    /// Create a new task scheduler service
    pub fn new(
        db: DatabaseConnection,
        config: Option<SchedulerConfig>,
        workspace_base_dir: String,
    ) -> Self {
        Self {
            db,
            config: config.unwrap_or_default(),
            running: Arc::new(RwLock::new(false)),
            workspace_base_dir,
        }
    }

    /// Poll for pending tasks and execute them
    async fn poll_and_execute(&self) -> Result<()> {
        info!("Polling for pending tasks...");

        // Get all workspaces with pending tasks
        let workspaces = self.get_workspaces_with_pending_tasks().await?;

        if workspaces.is_empty() {
            info!("No workspaces with pending tasks found");
            return Ok(());
        }

        info!("Found {} workspace(s) with pending tasks", workspaces.len());

        // Process each workspace
        for workspace in workspaces {
            if let Err(e) = self.process_workspace(&workspace).await {
                error!(
                    workspace_id = workspace.id,
                    error = %e,
                    "Failed to process workspace"
                );
            }
        }

        Ok(())
    }

    /// Get workspaces that have pending tasks
    async fn get_workspaces_with_pending_tasks(&self) -> Result<Vec<workspace::Model>> {
        // Find all workspaces that have at least one pending task
        let tasks = Task::find()
            .filter(task::Column::TaskStatus.eq(TaskStatus::Pending))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        if tasks.is_empty() {
            return Ok(Vec::new());
        }

        // Get unique workspace IDs
        let workspace_ids: Vec<i32> = tasks
            .iter()
            .map(|t| t.workspace_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Fetch workspace details
        let workspaces = Workspace::find()
            .filter(workspace::Column::Id.is_in(workspace_ids))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(workspaces)
    }

    /// Process a single workspace
    async fn process_workspace(&self, workspace: &workspace::Model) -> Result<()> {
        info!(workspace_id = workspace.id, "Processing workspace");

        // Get running tasks count for this workspace
        let running_count = self.get_running_tasks_count(workspace.id).await?;

        // Use hardcoded value of 3 for max concurrent tasks in simplified MVP
        let max_concurrent_tasks = 3;

        info!(
            workspace_id = workspace.id,
            running_count = running_count,
            max_concurrent = max_concurrent_tasks,
            "Workspace task status"
        );

        // Check if we can execute more tasks
        if running_count >= max_concurrent_tasks {
            info!(
                workspace_id = workspace.id,
                "Workspace at max concurrent task limit, skipping"
            );
            return Ok(());
        }

        // Calculate how many tasks we can start
        let available_slots = max_concurrent_tasks - running_count;

        // Get pending tasks ordered by priority
        let pending_tasks = self
            .get_pending_tasks_by_priority(workspace.id, available_slots as usize)
            .await?;

        if pending_tasks.is_empty() {
            info!(workspace_id = workspace.id, "No pending tasks found");
            return Ok(());
        }

        info!(
            workspace_id = workspace.id,
            task_count = pending_tasks.len(),
            "Found pending tasks to execute"
        );

        // Execute tasks
        let executor = TaskExecutorService::new(self.db.clone(), self.workspace_base_dir.clone());

        for task in pending_tasks {
            info!(
                workspace_id = workspace.id,
                task_id = task.id,
                priority = task.priority,
                "Starting task execution"
            );

            // Execute task in background (non-blocking)
            let executor_clone = executor.clone();
            let task_id = task.id;
            tokio::spawn(async move {
                if let Err(e) = executor_clone.execute_task(task_id).await {
                    error!(task_id = task_id, error = %e, "Task execution failed");
                }
            });
        }

        Ok(())
    }

    /// Get count of running tasks for a workspace
    async fn get_running_tasks_count(&self, workspace_id: i32) -> Result<i32> {
        let query = Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_id))
            .filter(task::Column::TaskStatus.eq(TaskStatus::Running));

        let count: u64 = query
            .count(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(count as i32)
    }

    /// Get pending tasks ordered by priority
    async fn get_pending_tasks_by_priority(
        &self,
        workspace_id: i32,
        limit: usize,
    ) -> Result<Vec<task::Model>> {
        let tasks = Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_id))
            .filter(task::Column::TaskStatus.eq(TaskStatus::Pending))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        // Sort by priority (high > medium > low)
        let mut tasks = tasks;
        tasks.sort_by(|a, b| {
            let priority_order = |p: &str| match p {
                "high" => 0,
                "medium" => 1,
                "low" => 2,
                _ => 3,
            };
            priority_order(&a.priority).cmp(&priority_order(&b.priority))
        });

        // Take only the requested number of tasks
        Ok(tasks.into_iter().take(limit).collect())
    }
}

#[async_trait]
impl BackgroundService for TaskSchedulerService {
    fn name(&self) -> &'static str {
        "TaskSchedulerService"
    }

    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        if !self.config.enabled {
            info!("Task scheduler is disabled");
            return Ok(());
        }

        info!(
            "Starting task scheduler with {}s polling interval",
            self.config.polling_interval_seconds
        );

        // Set running flag
        *self.running.write().await = true;

        // Spawn background task
        let db = self.db.clone();
        let config = self.config.clone();
        let running = self.running.clone();
        let workspace_base_dir = self.workspace_base_dir.clone();

        tokio::spawn(async move {
            let scheduler = TaskSchedulerService {
                db,
                config: config.clone(),
                running: running.clone(),
                workspace_base_dir,
            };

            let mut ticker = interval(Duration::from_secs(config.polling_interval_seconds));

            loop {
                ticker.tick().await;

                // Check if still running
                if !*running.read().await {
                    info!("Task scheduler stopped");
                    break;
                }

                // Poll and execute tasks
                if let Err(e) = scheduler.poll_and_execute().await {
                    error!(error = %e, "Failed to poll and execute tasks");
                }
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!("Stopping task scheduler...");
        *self.running.write().await = false;
        Ok(())
    }

    async fn health_check(&self) -> bool {
        *self.running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{repository, workspace};
    use crate::services::TaskService;
    use crate::test_utils::db::TestDatabase;
    use sea_orm::Set;

    #[tokio::test]
    async fn test_get_workspaces_with_pending_tasks() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let task_service = TaskService::new(db.clone());

        // Create a pending task
        task_service
            .create_task(
                workspace.id,
                101,
                "Test task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        let scheduler =
            TaskSchedulerService::new(db.clone(), None, "/tmp/test-workspace".to_string());

        // Act
        let result = scheduler.get_workspaces_with_pending_tasks().await;

        // Assert
        assert!(result.is_ok());
        let workspaces = result.unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].id, workspace.id);
    }

    #[tokio::test]
    async fn test_get_running_tasks_count() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let task_service = TaskService::new(db.clone());

        // Create tasks with different statuses
        let task1 = task_service
            .create_task(
                workspace.id,
                101,
                "Task 1".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        let _task2 = task_service
            .create_task(
                workspace.id,
                102,
                "Task 2".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Start task1 (make it running)
        task_service.start_task(task1.id).await.unwrap();

        let scheduler =
            TaskSchedulerService::new(db.clone(), None, "/tmp/test-workspace".to_string());

        // Act
        let result = scheduler.get_running_tasks_count(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_get_pending_tasks_by_priority() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let task_service = TaskService::new(db.clone());

        // Create tasks with different priorities
        let _low = task_service
            .create_task(
                workspace.id,
                101,
                "Low priority".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        let high = task_service
            .create_task(
                workspace.id,
                102,
                "High priority".to_string(),
                None,
                None,
                "high".to_string(),
            )
            .await
            .unwrap();

        let medium = task_service
            .create_task(
                workspace.id,
                103,
                "Medium priority".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        let scheduler =
            TaskSchedulerService::new(db.clone(), None, "/tmp/test-workspace".to_string());

        // Act
        let result = scheduler
            .get_pending_tasks_by_priority(workspace.id, 10)
            .await;

        // Assert
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 3);
        // Should be ordered: high, medium, low
        assert_eq!(tasks[0].id, high.id);
        assert_eq!(tasks[1].id, medium.id);
        assert_eq!(tasks[2].id, _low.id);
    }

    #[tokio::test]
    async fn test_get_pending_tasks_respects_limit() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let task_service = TaskService::new(db.clone());

        // Create 5 tasks
        for i in 1..=5 {
            task_service
                .create_task(
                    workspace.id,
                    100 + i,
                    format!("Task {}", i),
                    None,
                    None,
                    "medium".to_string(),
                )
                .await
                .unwrap();
        }

        let scheduler =
            TaskSchedulerService::new(db.clone(), None, "/tmp/test-workspace".to_string());

        // Act - request only 2 tasks
        let result = scheduler
            .get_pending_tasks_by_priority(workspace.id, 2)
            .await;

        // Assert
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_scheduler_config_default() {
        // Act
        let config = SchedulerConfig::default();

        // Assert
        assert_eq!(config.polling_interval_seconds, 30);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_health_check_when_running() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let scheduler =
            TaskSchedulerService::new(db.clone(), None, "/tmp/test-workspace".to_string());
        *scheduler.running.write().await = true;

        // Act
        let healthy = scheduler.health_check().await;

        // Assert
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_health_check_when_stopped() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let scheduler =
            TaskSchedulerService::new(db.clone(), None, "/tmp/test-workspace".to_string());
        *scheduler.running.write().await = false;

        // Act
        let healthy = scheduler.health_check().await;

        // Assert
        assert!(!healthy);
    }

    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        let repo = create_test_repository(db).await;
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
            ..Default::default()
        };
        Workspace::insert(ws).exec_with_returning(db).await.unwrap()
    }

    async fn create_test_repository(db: &DatabaseConnection) -> repository::Model {
        use crate::entities::repo_provider;

        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec_with_returning(db)
            .await
            .unwrap();

        let repo = repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.id),
            ..Default::default()
        };
        Repository::insert(repo)
            .exec_with_returning(db)
            .await
            .unwrap()
    }
}
