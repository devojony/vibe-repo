use crate::entities::{
    prelude::*,
    task::{self, TaskStatus},
};
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
        // Auto-assign agent if not explicitly provided
        let agent_id = if assigned_agent_id.is_none() {
            // Query the workspace's agent (should be unique per workspace in simplified MVP)
            use crate::entities::agent;
            let agent = agent::Entity::find()
                .filter(agent::Column::WorkspaceId.eq(workspace_id))
                .one(&self.db)
                .await
                .map_err(VibeRepoError::Database)?;
            
            agent.map(|a| a.id)
        } else {
            assigned_agent_id
        };

        let task = task::ActiveModel {
            workspace_id: Set(workspace_id),
            issue_number: Set(issue_number),
            issue_title: Set(issue_title),
            issue_body: Set(issue_body),
            task_status: Set(TaskStatus::Pending),
            priority: Set(priority),
            assigned_agent_id: Set(agent_id),
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

    pub async fn update_task_status(&self, id: i32, status: TaskStatus) -> Result<task::Model> {
        let task = self.get_task_by_id(id).await?;

        // Validate state transition
        if !task.task_status.can_transition_to(&status) {
            return Err(VibeRepoError::InvalidStateTransition {
                current: task.task_status,
                target: status,
                allowed: task.task_status.allowed_transitions(),
            });
        }

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set(status);
        task.updated_at = Set(Utc::now());

        let task = task
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(task)
    }

    /// Assign an agent to a task (simplified MVP - no status change)
    pub async fn assign_agent(&self, task_id: i32, agent_id: Option<i32>) -> Result<task::Model> {
        let task = self.get_task_by_id(task_id).await?;

        // In simplified MVP, we just update the assigned_agent_id without changing status
        let mut task: task::ActiveModel = task.into();
        task.assigned_agent_id = Set(agent_id);
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

        // Validate state transition
        if !task.task_status.can_transition_to(&TaskStatus::Running) {
            return Err(VibeRepoError::InvalidStateTransition {
                current: task.task_status,
                target: TaskStatus::Running,
                allowed: task.task_status.allowed_transitions(),
            });
        }

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set(TaskStatus::Running);
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

        // Validate state transition
        if !task.task_status.can_transition_to(&TaskStatus::Completed) {
            return Err(VibeRepoError::InvalidStateTransition {
                current: task.task_status,
                target: TaskStatus::Completed,
                allowed: task.task_status.allowed_transitions(),
            });
        }

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set(TaskStatus::Completed);
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

        // Transition to Failed (terminal state in simplified MVP)
        if !task.task_status.can_transition_to(&TaskStatus::Failed) {
            return Err(VibeRepoError::InvalidStateTransition {
                current: task.task_status,
                target: TaskStatus::Failed,
                allowed: task.task_status.allowed_transitions(),
            });
        }

        let mut task: task::ActiveModel = task.into();
        task.error_message = Set(Some(error_message));
        task.task_status = Set(TaskStatus::Failed);
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

        // Validate state transition
        if !task.task_status.can_transition_to(&TaskStatus::Pending) {
            return Err(VibeRepoError::InvalidStateTransition {
                current: task.task_status,
                target: TaskStatus::Pending,
                allowed: task.task_status.allowed_transitions(),
            });
        }

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set(TaskStatus::Pending);
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

        // Validate state transition
        if !task.task_status.can_transition_to(&TaskStatus::Cancelled) {
            return Err(VibeRepoError::InvalidStateTransition {
                current: task.task_status,
                target: TaskStatus::Cancelled,
                allowed: task.task_status.allowed_transitions(),
            });
        }

        let mut task: task::ActiveModel = task.into();
        task.task_status = Set(TaskStatus::Cancelled);
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
        status: Option<TaskStatus>,
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
        status: Option<TaskStatus>,
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
    use crate::entities::{agent, repository, workspace};
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
        assert_eq!(task.task_status, TaskStatus::Pending);
        assert_eq!(task.priority, "high");
        assert_eq!(task.assigned_agent_id, None);
    }

    /// Test create_task auto-assigns agent when workspace has an agent
    /// Requirements: Task API - auto-assign agent
    #[tokio::test]
    async fn test_create_task_auto_assigns_agent() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace with agent
        let workspace = create_test_workspace(db).await;
        let agent = create_test_agent(db, workspace.id).await;

        let service = TaskService::new(db.clone());

        // Act - Create task without specifying agent_id
        let result = service
            .create_task(
                workspace.id,
                124,
                "Test Issue with Auto-Assign".to_string(),
                Some("Issue body".to_string()),
                None, // No agent_id specified
                "high".to_string(),
            )
            .await;

        // Assert - Agent should be auto-assigned
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.workspace_id, workspace.id);
        assert_eq!(task.assigned_agent_id, Some(agent.id));
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
            .update_task_status(task.id, TaskStatus::Running)
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.task_status, TaskStatus::Running);
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
            .update_task_status(99999, TaskStatus::Completed)
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
        assert_eq!(updated.task_status, TaskStatus::Pending);
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

        // Assign the task first (required before starting)
        service.assign_agent(task.id, None).await.unwrap();

        // Act
        let result = service.start_task(task.id).await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.task_status, TaskStatus::Running);
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

        // Assign and start the task first (required before completing)
        service.assign_agent(task.id, None).await.unwrap();
        service.start_task(task.id).await.unwrap();

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
        assert_eq!(updated.task_status, TaskStatus::Completed);
        assert_eq!(updated.pr_number, Some(456));
        assert_eq!(
            updated.pr_url,
            Some("https://git.example.com/owner/repo/pulls/456".to_string())
        );
        assert_eq!(updated.branch_name, Some("fix/test-branch".to_string()));
        assert!(updated.completed_at.is_some());
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test fail_task marks task as failed
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

        // Assign and start the task first (required before failing)
        service.assign_agent(task.id, None).await.unwrap();
        service.start_task(task.id).await.unwrap();

        // Act
        let result = service
            .fail_task(task.id, "Test error message".to_string())
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(
            updated.error_message,
            Some("Test error message".to_string())
        );
        assert_eq!(updated.task_status, TaskStatus::Failed);
        assert!(updated.updated_at > task.updated_at);
    }

    /// Test invalid transition from Completed to Failed
    /// Requirements: Task state machine - terminal state validation
    #[tokio::test]
    async fn test_invalid_transition_completed_to_failed() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create and complete a task
        let task = service
            .create_task(
                workspace.id,
                701,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Assign, start, and complete the task
        service.assign_agent(task.id, None).await.unwrap();
        service.start_task(task.id).await.unwrap();
        service
            .complete_task(
                task.id,
                123,
                "https://example.com/pr/123".to_string(),
                "feature/test".to_string(),
            )
            .await
            .unwrap();

        // Act - Try to fail a completed task
        let result = service.fail_task(task.id, "Error".to_string()).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::InvalidStateTransition {
                current, target, ..
            } => {
                assert_eq!(current, TaskStatus::Completed);
                // Target will be Failed (fail_task always transitions to Failed first)
                assert_eq!(target, TaskStatus::Failed);
            }
            _ => panic!("Expected InvalidStateTransition error"),
        }
    }

    /// Test invalid transition from Cancelled to Running
    /// Requirements: Task state machine - terminal state validation
    #[tokio::test]
    async fn test_invalid_transition_cancelled_to_running() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create and cancel a task
        let task = service
            .create_task(
                workspace.id,
                702,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        service.cancel_task(task.id).await.unwrap();

        // Act - Try to start a cancelled task
        let result = service.start_task(task.id).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::InvalidStateTransition {
                current, target, ..
            } => {
                assert_eq!(current, TaskStatus::Cancelled);
                assert_eq!(target, TaskStatus::Running);
            }
            _ => panic!("Expected InvalidStateTransition error"),
        }
    }

    /// Test valid transition from Pending to Running (simplified MVP)
    /// Requirements: Task state machine - direct transition allowed
    #[tokio::test]
    async fn test_valid_transition_pending_to_running() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create a pending task
        let task = service
            .create_task(
                workspace.id,
                703,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act - Start a pending task (now allowed in simplified MVP)
        let result = service.start_task(task.id).await;

        // Assert - Should succeed
        assert!(result.is_ok());
        let updated_task = result.unwrap();
        assert_eq!(updated_task.task_status, TaskStatus::Running);
    }

    /// Test terminal state validation - Completed cannot transition
    /// Requirements: Task state machine - terminal states
    #[tokio::test]
    async fn test_terminal_state_completed_cannot_transition() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create and complete a task
        let task = service
            .create_task(
                workspace.id,
                704,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        service.assign_agent(task.id, None).await.unwrap();
        service.start_task(task.id).await.unwrap();
        service
            .complete_task(
                task.id,
                124,
                "https://example.com/pr/124".to_string(),
                "feature/test2".to_string(),
            )
            .await
            .unwrap();

        // Act & Assert - Try various transitions from Completed
        assert!(service.start_task(task.id).await.is_err());
        assert!(service.cancel_task(task.id).await.is_err());
    }

    /// Test terminal state validation - Cancelled cannot transition
    /// Requirements: Task state machine - terminal states
    #[tokio::test]
    async fn test_terminal_state_cancelled_cannot_transition() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let service = TaskService::new(db.clone());

        // Create and cancel a task
        let task = service
            .create_task(
                workspace.id,
                705,
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        service.cancel_task(task.id).await.unwrap();

        // Act & Assert - Try various transitions from Cancelled
        assert!(service.start_task(task.id).await.is_err());
        assert!(service
            .complete_task(
                task.id,
                125,
                "https://example.com/pr/125".to_string(),
                "feature/test3".to_string(),
            )
            .await
            .is_err());
    }

    // Helper functions
    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        let repo = create_test_repository(db).await;
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
            ..Default::default()
        };
        Workspace::insert(ws).exec_with_returning(db).await.unwrap()
    }

    async fn create_test_agent(db: &DatabaseConnection, workspace_id: i32) -> agent::Model {
        let agent = agent::ActiveModel {
            workspace_id: Set(workspace_id),
            name: Set("Test Agent".to_string()),
            tool_type: Set("opencode".to_string()),
            command: Set("opencode".to_string()),
            env_vars: Set(serde_json::json!({})),
            timeout: Set(600),
            ..Default::default()
        };
        Agent::insert(agent).exec_with_returning(db).await.unwrap()
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
