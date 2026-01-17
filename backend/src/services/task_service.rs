use crate::entities::{prelude::*, task};
use crate::error::{GitAutoDevError, Result};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

#[derive(Clone)]
pub struct TaskService {
    db: DatabaseConnection,
}

impl TaskService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_task(
        &self,
        workspace_id: i32,
        issue_number: i32,
        issue_title: String,
        issue_body: Option<String>,
        assigned_agent_id: Option<i32>,
        priority: String,
    ) -> Result<task::Model> {
        let task = task::ActiveModel {
            workspace_id: Set(workspace_id),
            issue_number: Set(issue_number),
            issue_title: Set(issue_title),
            issue_body: Set(issue_body),
            task_status: Set("pending".to_string()),
            priority: Set(priority),
            assigned_agent_id: Set(assigned_agent_id),
            retry_count: Set(0),
            max_retries: Set(3),
            ..Default::default()
        };

        let task = Task::insert(task)
            .exec_with_returning(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?;

        Ok(task)
    }

    pub async fn get_task_by_id(&self, id: i32) -> Result<task::Model> {
        Task::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?
            .ok_or_else(|| GitAutoDevError::NotFound(format!("Task with id {} not found", id)))
    }

    pub async fn list_tasks_by_workspace(&self, workspace_id: i32) -> Result<Vec<task::Model>> {
        Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_id))
            .all(&self.db)
            .await
            .map_err(GitAutoDevError::Database)
    }

    pub async fn update_task_status(&self, id: i32, status: String) -> Result<task::Model> {
        let task = self.get_task_by_id(id).await?;

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set(status);
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?;

        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{repository, workspace};
    use crate::test_utils::db::TestDatabase;

    /// Test create_task creates a task with correct fields
    /// Requirements: Task API - create task
    #[tokio::test]
    async fn test_create_task_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace
        let workspace = create_test_workspace(db).await;

        let service = TaskService::new(db.clone());

        // Act
        let result = service
            .create_task(
                workspace.id,
                123,
                "Test Issue".to_string(),
                Some("Issue body".to_string()),
                None,
                "high".to_string(),
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.workspace_id, workspace.id);
        assert_eq!(task.issue_number, 123);
        assert_eq!(task.issue_title, "Test Issue");
        assert_eq!(task.issue_body, Some("Issue body".to_string()));
        assert_eq!(task.task_status, "pending");
        assert_eq!(task.priority, "high");
        assert_eq!(task.assigned_agent_id, None);
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
    }

    /// Test get_task_by_id returns task when exists
    /// Requirements: Task API - get task
    #[tokio::test]
    async fn test_get_task_by_id_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());
        let created = service
            .create_task(
                workspace.id,
                456,
                "Another Issue".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service.get_task_by_id(created.id).await;

        // Assert
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.id, created.id);
        assert_eq!(task.issue_number, 456);
        assert_eq!(task.issue_title, "Another Issue");
    }

    /// Test get_task_by_id returns NotFound error when task doesn't exist
    /// Requirements: Task API - error handling
    #[tokio::test]
    async fn test_get_task_by_id_not_found() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = TaskService::new(db.clone());

        // Act
        let result = service.get_task_by_id(99999).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            GitAutoDevError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    /// Test list_tasks_by_workspace returns all tasks for workspace
    /// Requirements: Task API - list tasks
    #[tokio::test]
    async fn test_list_tasks_by_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create multiple tasks
        service
            .create_task(
                workspace.id,
                1,
                "Task 1".to_string(),
                None,
                None,
                "high".to_string(),
            )
            .await
            .unwrap();
        service
            .create_task(
                workspace.id,
                2,
                "Task 2".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service.list_tasks_by_workspace(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    /// Test list_tasks_by_workspace returns empty list when no tasks
    /// Requirements: Task API - list tasks
    #[tokio::test]
    async fn test_list_tasks_by_workspace_empty() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Act
        let result = service.list_tasks_by_workspace(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    /// Test update_task_status updates status correctly
    /// Requirements: Task API - update task status
    #[tokio::test]
    async fn test_update_task_status_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());
        let task = service
            .create_task(
                workspace.id,
                789,
                "Status Test".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service
            .update_task_status(task.id, "in_progress".to_string())
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.task_status, "in_progress");
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test update_task_status returns NotFound for non-existent task
    /// Requirements: Task API - error handling
    #[tokio::test]
    async fn test_update_task_status_not_found() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = TaskService::new(db.clone());

        // Act
        let result = service
            .update_task_status(99999, "completed".to_string())
            .await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            GitAutoDevError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    // Helper functions
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

        // Create a test provider first
        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider).exec(db).await.unwrap();

        let repo = repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.last_insert_id),
            ..Default::default()
        };
        Repository::insert(repo)
            .exec_with_returning(db)
            .await
            .unwrap()
    }
}
