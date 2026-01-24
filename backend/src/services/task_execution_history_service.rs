//! Task Execution Service
//!
//! Manages task execution history and records.

use crate::entities::{prelude::*, task_execution};
use crate::error::{Result, VibeRepoError};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const MAX_SUMMARY_LENGTH: usize = 4096; // 4KB
const LOG_DIR: &str = "./data/vibe-repo/task-logs";

#[derive(Clone)]
pub struct TaskExecutionService {
    db: DatabaseConnection,
}

impl TaskExecutionService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new task execution record
    pub async fn create_execution(
        &self,
        task_id: i32,
        agent_id: Option<i32>,
        command: String,
    ) -> Result<task_execution::Model> {
        let execution = task_execution::ActiveModel {
            task_id: Set(task_id),
            agent_id: Set(agent_id),
            status: Set("running".to_string()),
            command: Set(command),
            started_at: Set(Utc::now()),
            ..Default::default()
        };

        let execution = TaskExecution::insert(execution)
            .exec_with_returning(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(execution)
    }

    /// Update execution with completion data
    #[allow(clippy::too_many_arguments)]
    pub async fn complete_execution(
        &self,
        execution_id: i32,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
        pr_number: Option<i32>,
        pr_url: Option<String>,
        branch_name: Option<String>,
    ) -> Result<task_execution::Model> {
        let execution = self.get_execution_by_id(execution_id).await?;

        // Calculate duration
        let duration_ms = Utc::now()
            .signed_duration_since(execution.started_at)
            .num_milliseconds();

        // Store stdout
        let (stdout_summary, stdout_file_path) =
            self.store_output(execution_id, "stdout", &stdout)?;

        // Store stderr
        let (stderr_summary, stderr_file_path) =
            self.store_output(execution_id, "stderr", &stderr)?;

        let mut execution: task_execution::ActiveModel = execution.into();
        execution.status = Set("completed".to_string());
        execution.exit_code = Set(exit_code);
        execution.stdout_summary = Set(stdout_summary);
        execution.stderr_summary = Set(stderr_summary);
        execution.stdout_file_path = Set(stdout_file_path);
        execution.stderr_file_path = Set(stderr_file_path);
        execution.pr_number = Set(pr_number);
        execution.pr_url = Set(pr_url);
        execution.branch_name = Set(branch_name);
        execution.duration_ms = Set(Some(duration_ms));
        execution.completed_at = Set(Some(Utc::now()));
        execution.updated_at = Set(Utc::now());

        let execution = execution
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(execution)
    }

    /// Mark execution as failed
    pub async fn fail_execution(
        &self,
        execution_id: i32,
        error_message: String,
        stdout: String,
        stderr: String,
    ) -> Result<task_execution::Model> {
        let execution = self.get_execution_by_id(execution_id).await?;

        // Calculate duration
        let duration_ms = Utc::now()
            .signed_duration_since(execution.started_at)
            .num_milliseconds();

        // Store stdout
        let (stdout_summary, stdout_file_path) =
            self.store_output(execution_id, "stdout", &stdout)?;

        // Store stderr
        let (stderr_summary, stderr_file_path) =
            self.store_output(execution_id, "stderr", &stderr)?;

        let mut execution: task_execution::ActiveModel = execution.into();
        execution.status = Set("failed".to_string());
        execution.error_message = Set(Some(error_message));
        execution.stdout_summary = Set(stdout_summary);
        execution.stderr_summary = Set(stderr_summary);
        execution.stdout_file_path = Set(stdout_file_path);
        execution.stderr_file_path = Set(stderr_file_path);
        execution.duration_ms = Set(Some(duration_ms));
        execution.completed_at = Set(Some(Utc::now()));
        execution.updated_at = Set(Utc::now());

        let execution = execution
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(execution)
    }

    /// Get execution by ID
    pub async fn get_execution_by_id(&self, id: i32) -> Result<task_execution::Model> {
        TaskExecution::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Task execution with id {} not found", id))
            })
    }

    /// List executions for a task
    pub async fn list_executions_by_task(
        &self,
        task_id: i32,
    ) -> Result<Vec<task_execution::Model>> {
        TaskExecution::find()
            .filter(task_execution::Column::TaskId.eq(task_id))
            .order_by_desc(task_execution::Column::StartedAt)
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)
    }

    /// Get latest execution for a task
    pub async fn get_latest_execution(
        &self,
        task_id: i32,
    ) -> Result<Option<task_execution::Model>> {
        TaskExecution::find()
            .filter(task_execution::Column::TaskId.eq(task_id))
            .order_by_desc(task_execution::Column::StartedAt)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)
    }

    /// Store output (stdout or stderr)
    /// Returns (summary, file_path)
    fn store_output(
        &self,
        execution_id: i32,
        output_type: &str,
        content: &str,
    ) -> Result<(Option<String>, Option<String>)> {
        if content.is_empty() {
            return Ok((None, None));
        }

        if content.len() <= MAX_SUMMARY_LENGTH {
            // Store in database
            Ok((Some(content.to_string()), None))
        } else {
            // Store summary in database, full content in file
            let summary = content.chars().take(MAX_SUMMARY_LENGTH).collect::<String>();
            let file_path = self.write_output_file(execution_id, output_type, content)?;
            Ok((Some(summary), Some(file_path)))
        }
    }

    /// Write output to file
    fn write_output_file(
        &self,
        execution_id: i32,
        output_type: &str,
        content: &str,
    ) -> Result<String> {
        // Create log directory if it doesn't exist
        fs::create_dir_all(LOG_DIR).map_err(|e| {
            VibeRepoError::Internal(format!("Failed to create log directory: {}", e))
        })?;

        // Generate file path
        let filename = format!("execution_{}_{}.log", execution_id, output_type);
        let file_path = PathBuf::from(LOG_DIR).join(&filename);

        // Write content to file
        let mut file = fs::File::create(&file_path)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create log file: {}", e)))?;

        file.write_all(content.as_bytes())
            .map_err(|e| VibeRepoError::Internal(format!("Failed to write log file: {}", e)))?;

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Read full output from file
    pub fn read_output_file(&self, file_path: &str) -> Result<String> {
        fs::read_to_string(file_path)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to read log file: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{repository, task, workspace};
    use crate::test_utils::db::TestDatabase;
    use sea_orm::Set;

    #[tokio::test]
    async fn test_create_execution() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let task = create_test_task(db).await;
        let service = TaskExecutionService::new(db.clone());

        // Act
        let result = service
            .create_execution(task.id, None, "test command".to_string())
            .await;

        // Assert
        assert!(result.is_ok());
        let execution = result.unwrap();
        assert_eq!(execution.task_id, task.id);
        assert_eq!(execution.status, "running");
        assert_eq!(execution.command, "test command");
    }

    #[tokio::test]
    async fn test_complete_execution() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let task = create_test_task(db).await;
        let service = TaskExecutionService::new(db.clone());

        let execution = service
            .create_execution(task.id, None, "test command".to_string())
            .await
            .unwrap();

        // Act
        let result = service
            .complete_execution(
                execution.id,
                Some(0),
                "stdout output".to_string(),
                "stderr output".to_string(),
                Some(123),
                Some("https://example.com/pr/123".to_string()),
                Some("feature/test".to_string()),
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let completed = result.unwrap();
        assert_eq!(completed.status, "completed");
        assert_eq!(completed.exit_code, Some(0));
        assert_eq!(completed.pr_number, Some(123));
        assert!(completed.completed_at.is_some());
        assert!(completed.duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_fail_execution() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let task = create_test_task(db).await;
        let service = TaskExecutionService::new(db.clone());

        let execution = service
            .create_execution(task.id, None, "test command".to_string())
            .await
            .unwrap();

        // Act
        let result = service
            .fail_execution(
                execution.id,
                "Test error".to_string(),
                "stdout".to_string(),
                "stderr".to_string(),
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let failed = result.unwrap();
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.error_message, Some("Test error".to_string()));
        assert!(failed.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_list_executions_by_task() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let task = create_test_task(db).await;
        let service = TaskExecutionService::new(db.clone());

        // Create multiple executions
        service
            .create_execution(task.id, None, "command 1".to_string())
            .await
            .unwrap();
        service
            .create_execution(task.id, None, "command 2".to_string())
            .await
            .unwrap();

        // Act
        let result = service.list_executions_by_task(task.id).await;

        // Assert
        assert!(result.is_ok());
        let executions = result.unwrap();
        assert_eq!(executions.len(), 2);
    }

    #[tokio::test]
    async fn test_store_output_small() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = TaskExecutionService::new(db.clone());

        let small_content = "Small output";

        // Act
        let result = service.store_output(1, "stdout", small_content);

        // Assert
        assert!(result.is_ok());
        let (summary, file_path) = result.unwrap();
        assert_eq!(summary, Some(small_content.to_string()));
        assert_eq!(file_path, None);
    }

    #[tokio::test]
    async fn test_store_output_large() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = TaskExecutionService::new(db.clone());

        let large_content = "x".repeat(5000);

        // Act
        let result = service.store_output(1, "stdout", &large_content);

        // Assert
        assert!(result.is_ok());
        let (summary, file_path) = result.unwrap();
        assert!(summary.is_some());
        assert_eq!(summary.unwrap().len(), MAX_SUMMARY_LENGTH);
        assert!(file_path.is_some());

        // Cleanup
        if let Some(path) = file_path {
            let _ = fs::remove_file(path);
        }
    }

    async fn create_test_task(db: &DatabaseConnection) -> task::Model {
        let workspace = create_test_workspace(db).await;

        let task = task::ActiveModel {
            workspace_id: Set(workspace.id),
            issue_number: Set(123),
            issue_title: Set("Test task".to_string()),
            issue_body: Set(None),
            task_status: Set("pending".to_string()),
            priority: Set("medium".to_string()),
            ..Default::default()
        };

        Task::insert(task).exec_with_returning(db).await.unwrap()
    }

    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        let repo = create_test_repository(db).await;
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
            workspace_status: Set("Active".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
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
