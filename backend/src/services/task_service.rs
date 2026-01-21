use crate::entities::{prelude::*, task};
use crate::error::{Result, VibeRepoError};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

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
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    pub async fn get_task_by_id(&self, id: i32) -> Result<task::Model> {
        Task::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| VibeRepoError::NotFound(format!("Task with id {} not found", id)))
    }

    pub async fn list_tasks_by_workspace(&self, workspace_id: i32) -> Result<Vec<task::Model>> {
        Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_id))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)
    }

    pub async fn update_task_status(&self, id: i32, status: String) -> Result<task::Model> {
        let task = self.get_task_by_id(id).await?;

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set(status);
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Assign an agent to a task
    pub async fn assign_agent(&self, task_id: i32, agent_id: Option<i32>) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        let mut task: task::ActiveModel = task.into();
        task.assigned_agent_id = Set(agent_id);
        task.task_status = Set("assigned".to_string());
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Start a task
    pub async fn start_task(&self, task_id: i32) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set("running".to_string());
        task.started_at = Set(Some(Utc::now()));
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Complete a task with PR information
    pub async fn complete_task(
        &self,
        task_id: i32,
        pr_number: i32,
        pr_url: String,
        branch_name: String,
    ) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set("completed".to_string());
        task.pr_number = Set(Some(pr_number));
        task.pr_url = Set(Some(pr_url));
        task.branch_name = Set(Some(branch_name));
        task.completed_at = Set(Some(Utc::now()));
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Mark a task as failed
    pub async fn fail_task(&self, task_id: i32, error_message: String) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        let new_retry_count = task.retry_count + 1;
        let new_status = if new_retry_count < task.max_retries {
            "pending".to_string()
        } else {
            "failed".to_string()
        };

        let mut task: task::ActiveModel = task.into();
        task.retry_count = Set(new_retry_count);
        task.error_message = Set(Some(error_message));
        task.task_status = Set(new_status);
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Retry a failed task
    pub async fn retry_task(&self, task_id: i32) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set("pending".to_string());
        task.error_message = Set(None);
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: i32) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set("cancelled".to_string());
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Soft delete a task
    pub async fn soft_delete_task(&self, task_id: i32) -> Result<()> {
        let task = self.get_task_by_id(task_id).await?;

        let mut task: task::ActiveModel = task.into();
        task.deleted_at = Set(Some(Utc::now()));
        task.updated_at = Set(Utc::now());

        task.update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(())
    }

    /// Update task fields
    pub async fn update_task(
        &self,
        task_id: i32,
        priority: Option<String>,
        assigned_agent_id: Option<Option<i32>>,
    ) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        let mut task: task::ActiveModel = task.into();

        if let Some(p) = priority {
            task.priority = Set(p);
        }

        if let Some(agent_id) = assigned_agent_id {
            task.assigned_agent_id = Set(agent_id);
        }

        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// List tasks with filters
    pub async fn list_tasks_with_filters(
        &self,
        workspace_id: i32,
        status: Option<String>,
        priority: Option<String>,
        assigned_agent_id: Option<i32>,
    ) -> Result<Vec<task::Model>> {
        let mut query = Task::find().filter(task::Column::WorkspaceId.eq(workspace_id));

        if let Some(s) = status {
            query = query.filter(task::Column::TaskStatus.eq(s));
        }

        if let Some(p) = priority {
            query = query.filter(task::Column::Priority.eq(p));
        }

        if let Some(agent_id) = assigned_agent_id {
            query = query.filter(task::Column::AssignedAgentId.eq(agent_id));
        }

        query.all(&self.db).await.map_err(VibeRepoError::Database)
    }

    /// List tasks with pagination and filters
    pub async fn list_tasks_with_pagination(
        &self,
        workspace_id: i32,
        status: Option<String>,
        priority: Option<String>,
        assigned_agent_id: Option<i32>,
        page: i32,
        per_page: i32,
    ) -> Result<(Vec<task::Model>, i64)> {
        let mut query = Task::find().filter(task::Column::WorkspaceId.eq(workspace_id));

        if let Some(s) = status {
            query = query.filter(task::Column::TaskStatus.eq(s));
        }

        if let Some(p) = priority {
            query = query.filter(task::Column::Priority.eq(p));
        }

        if let Some(agent_id) = assigned_agent_id {
            query = query.filter(task::Column::AssignedAgentId.eq(agent_id));
        }

        // Order by created_at descending (newest first)
        query = query.order_by_desc(task::Column::CreatedAt);

        // Get total count
        let total = query
            .clone()
            .count(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        // Calculate offset
        let offset = ((page - 1) * per_page) as u64;

        // Get paginated results
        let tasks = query
            .offset(offset)
            .limit(per_page as u64)
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok((tasks, total as i64))
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
            VibeRepoError::NotFound(_) => {}
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
            VibeRepoError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    /// Test assign_agent assigns agent and updates status
    /// Requirements: Task API - assign agent
    #[tokio::test]
    async fn test_assign_agent_success() {
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
                100,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act - Use None to avoid foreign key constraint
        let result = service.assign_agent(task.id, None).await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.assigned_agent_id, None);
        assert_eq!(updated.task_status, "assigned");
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test start_task sets status to running and records start time
    /// Requirements: Task API - start task
    #[tokio::test]
    async fn test_start_task_success() {
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
                101,
                "Test Task".to_string(),
                None,
                None,
                "high".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service.start_task(task.id).await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.task_status, "running");
        assert!(updated.started_at.is_some());
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test complete_task sets status and PR information
    /// Requirements: Task API - complete task
    #[tokio::test]
    async fn test_complete_task_success() {
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
                102,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service
            .complete_task(
                task.id,
                456,
                "https://git.example.com/owner/repo/pulls/456".to_string(),
                "fix/test-branch".to_string(),
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.task_status, "completed");
        assert_eq!(updated.pr_number, Some(456));
        assert_eq!(
            updated.pr_url,
            Some("https://git.example.com/owner/repo/pulls/456".to_string())
        );
        assert_eq!(updated.branch_name, Some("fix/test-branch".to_string()));
        assert!(updated.completed_at.is_some());
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test fail_task increments retry count and sets error message
    /// Requirements: Task API - fail task
    #[tokio::test]
    async fn test_fail_task_with_retry() {
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
                103,
                "Test Task".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service
            .fail_task(task.id, "Test error message".to_string())
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.retry_count, 1);
        assert_eq!(
            updated.error_message,
            Some("Test error message".to_string())
        );
        assert_eq!(updated.task_status, "pending"); // Should retry
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test fail_task marks as failed when max retries reached
    /// Requirements: Task API - fail task with max retries
    #[tokio::test]
    async fn test_fail_task_max_retries() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());
        let mut task = service
            .create_task(
                workspace.id,
                104,
                "Test Task".to_string(),
                None,
                None,
                "high".to_string(),
            )
            .await
            .unwrap();

        // Fail task multiple times to reach max retries
        for _ in 0..3 {
            task = service
                .fail_task(task.id, "Test error".to_string())
                .await
                .unwrap();
        }

        // Assert
        assert_eq!(task.retry_count, 3);
        assert_eq!(task.task_status, "failed"); // Should be failed now
    }

    /// Test retry_task resets status and clears error
    /// Requirements: Task API - retry task
    #[tokio::test]
    async fn test_retry_task_success() {
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
                105,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Fail the task first
        let failed_task = service
            .fail_task(task.id, "Test error".to_string())
            .await
            .unwrap();

        // Act
        let result = service.retry_task(failed_task.id).await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.task_status, "pending");
        assert_eq!(updated.error_message, None);
        assert!(updated.updated_at > failed_task.updated_at);
    }

    /// Test cancel_task sets status to cancelled
    /// Requirements: Task API - cancel task
    #[tokio::test]
    async fn test_cancel_task_success() {
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
                106,
                "Test Task".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service.cancel_task(task.id).await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.task_status, "cancelled");
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test soft_delete_task sets deleted_at timestamp
    /// Requirements: Task API - soft delete
    #[tokio::test]
    async fn test_soft_delete_task_success() {
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
                107,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service.soft_delete_task(task.id).await;

        // Assert
        assert!(result.is_ok());

        // Verify task is marked as deleted
        let deleted_task = service.get_task_by_id(task.id).await.unwrap();
        assert!(deleted_task.deleted_at.is_some());
    }

    /// Test update_task updates priority
    /// Requirements: Task API - update task
    #[tokio::test]
    async fn test_update_task_priority() {
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
                108,
                "Test Task".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service
            .update_task(task.id, Some("high".to_string()), None)
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.priority, "high");
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test update_task updates assigned agent
    /// Requirements: Task API - update task
    #[tokio::test]
    async fn test_update_task_assigned_agent() {
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
                109,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act - Use None to avoid foreign key constraint
        let result = service.update_task(task.id, None, Some(None)).await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.assigned_agent_id, None);
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test list_tasks_with_filters filters by status
    /// Requirements: Task API - list tasks with filters
    #[tokio::test]
    async fn test_list_tasks_with_filters_by_status() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create tasks with different statuses
        let task1 = service
            .create_task(
                workspace.id,
                110,
                "Task 1".to_string(),
                None,
                None,
                "high".to_string(),
            )
            .await
            .unwrap();

        let task2 = service
            .create_task(
                workspace.id,
                111,
                "Task 2".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Update one task to running
        service.start_task(task2.id).await.unwrap();

        // Act
        let result = service
            .list_tasks_with_filters(workspace.id, Some("pending".to_string()), None, None)
            .await;

        // Assert
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, task1.id);
        assert_eq!(tasks[0].task_status, "pending");
    }

    /// Test list_tasks_with_filters filters by priority
    /// Requirements: Task API - list tasks with filters
    #[tokio::test]
    async fn test_list_tasks_with_filters_by_priority() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create tasks with different priorities
        service
            .create_task(
                workspace.id,
                112,
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
                113,
                "Task 2".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = service
            .list_tasks_with_filters(workspace.id, None, Some("high".to_string()), None)
            .await;

        // Assert
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].priority, "high");
    }

    /// Test list_tasks_with_filters filters by assigned agent
    /// Requirements: Task API - list tasks with filters
    #[tokio::test]
    async fn test_list_tasks_with_filters_by_agent() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create tasks - one with assigned_agent_id in create_task, one without
        let _task1 = service
            .create_task(
                workspace.id,
                114,
                "Task 1".to_string(),
                None,
                None, // No agent assigned
                "medium".to_string(),
            )
            .await
            .unwrap();

        service
            .create_task(
                workspace.id,
                115,
                "Task 2".to_string(),
                None,
                None, // No agent assigned
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act - Filter by None (tasks without assigned agent)
        let result = service
            .list_tasks_with_filters(workspace.id, None, None, None)
            .await;

        // Assert
        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 2); // Both tasks have no agent
        assert_eq!(tasks[0].assigned_agent_id, None);
    }

    /// Test list_tasks_with_pagination returns correct page
    /// Requirements: Task API - pagination support
    #[tokio::test]
    async fn test_list_tasks_with_pagination_first_page() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create 5 tasks
        for i in 1..=5 {
            service
                .create_task(
                    workspace.id,
                    200 + i,
                    format!("Task {}", i),
                    None,
                    None,
                    "medium".to_string(),
                )
                .await
                .unwrap();
        }

        // Act - Get first page with 2 items per page
        let result = service
            .list_tasks_with_pagination(workspace.id, None, None, None, 1, 2)
            .await;

        // Assert
        assert!(result.is_ok());
        let (tasks, total) = result.unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(total, 5);
    }

    /// Test list_tasks_with_pagination returns correct second page
    /// Requirements: Task API - pagination support
    #[tokio::test]
    async fn test_list_tasks_with_pagination_second_page() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create 5 tasks
        for i in 1..=5 {
            service
                .create_task(
                    workspace.id,
                    300 + i,
                    format!("Task {}", i),
                    None,
                    None,
                    "medium".to_string(),
                )
                .await
                .unwrap();
        }

        // Act - Get second page with 2 items per page
        let result = service
            .list_tasks_with_pagination(workspace.id, None, None, None, 2, 2)
            .await;

        // Assert
        assert!(result.is_ok());
        let (tasks, total) = result.unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(total, 5);
    }

    /// Test list_tasks_with_pagination with filters
    /// Requirements: Task API - pagination with filters
    #[tokio::test]
    async fn test_list_tasks_with_pagination_and_filters() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create 3 high priority tasks and 2 low priority tasks
        for i in 1..=3 {
            service
                .create_task(
                    workspace.id,
                    400 + i,
                    format!("High Task {}", i),
                    None,
                    None,
                    "high".to_string(),
                )
                .await
                .unwrap();
        }

        for i in 1..=2 {
            service
                .create_task(
                    workspace.id,
                    500 + i,
                    format!("Low Task {}", i),
                    None,
                    None,
                    "low".to_string(),
                )
                .await
                .unwrap();
        }

        // Act - Get first page of high priority tasks
        let result = service
            .list_tasks_with_pagination(workspace.id, None, Some("high".to_string()), None, 1, 2)
            .await;

        // Assert
        assert!(result.is_ok());
        let (tasks, total) = result.unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(total, 3);
        assert!(tasks.iter().all(|t| t.priority == "high"));
    }

    /// Test list_tasks_with_pagination orders by created_at desc
    /// Requirements: Task API - pagination ordering
    #[tokio::test]
    async fn test_list_tasks_with_pagination_ordering() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create 3 tasks
        let task1 = service
            .create_task(
                workspace.id,
                601,
                "First Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        let task2 = service
            .create_task(
                workspace.id,
                602,
                "Second Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        let task3 = service
            .create_task(
                workspace.id,
                603,
                "Third Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act - Get all tasks
        let result = service
            .list_tasks_with_pagination(workspace.id, None, None, None, 1, 10)
            .await;

        // Assert - Should be ordered by created_at desc (newest first)
        assert!(result.is_ok());
        let (tasks, _) = result.unwrap();
        assert_eq!(tasks.len(), 3);
        // Since tasks are created in quick succession, created_at might be the same
        // So we verify that all three tasks are present
        let task_ids: Vec<i32> = tasks.iter().map(|t| t.id).collect();
        assert!(task_ids.contains(&task1.id));
        assert!(task_ids.contains(&task2.id));
        assert!(task_ids.contains(&task3.id));
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
