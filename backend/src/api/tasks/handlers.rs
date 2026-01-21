use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{api::tasks::models::*, error::Result, services::TaskService, state::AppState};

/// Create a new task
#[utoipa::path(
    post,
    path = "/api/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created successfully", body = TaskResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>)> {
    let service = TaskService::new(state.db.clone());

    let task = service
        .create_task(
            req.workspace_id,
            req.issue_number,
            req.issue_title,
            req.issue_body,
            req.assigned_agent_id,
            req.priority,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(task.into())))
}

/// Get task by ID
#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task found", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service.get_task_by_id(id).await?;

    Ok(Json(task.into()))
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub workspace_id: i32,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assigned_agent_id: Option<i32>,
}

/// List tasks by workspace with filters
#[utoipa::path(
    get,
    path = "/api/tasks",
    params(
        ("workspace_id" = i32, Query, description = "Workspace ID"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("priority" = Option<String>, Query, description = "Filter by priority"),
        ("assigned_agent_id" = Option<i32>, Query, description = "Filter by assigned agent"),
    ),
    responses(
        (status = 200, description = "List of tasks", body = Vec<TaskResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn list_tasks_by_workspace(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<TaskResponse>>> {
    let service = TaskService::new(state.db.clone());

    let tasks = service
        .list_tasks_with_filters(
            query.workspace_id,
            query.status,
            query.priority,
            query.assigned_agent_id,
        )
        .await?;

    let responses: Vec<TaskResponse> = tasks.into_iter().map(|t| t.into()).collect();

    Ok(Json(responses))
}

/// Update task status
#[utoipa::path(
    patch,
    path = "/api/tasks/{id}/status",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = UpdateTaskStatusRequest,
    responses(
        (status = 200, description = "Task status updated", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn update_task_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTaskStatusRequest>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service.update_task_status(id, req.status).await?;

    Ok(Json(task.into()))
}

/// Update task
#[utoipa::path(
    patch,
    path = "/api/tasks/{id}",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "Task updated successfully", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn update_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service
        .update_task(id, req.priority, req.assigned_agent_id.map(Some))
        .await?;

    Ok(Json(task.into()))
}

/// Delete task (soft delete)
#[utoipa::path(
    delete,
    path = "/api/tasks/{id}",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 204, description = "Task deleted successfully"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<StatusCode> {
    let service = TaskService::new(state.db.clone());

    service.soft_delete_task(id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Assign agent to task
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/assign",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = AssignAgentRequest,
    responses(
        (status = 200, description = "Agent assigned successfully", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn assign_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<AssignAgentRequest>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service.assign_agent(id, req.agent_id).await?;

    Ok(Json(task.into()))
}

/// Start task execution
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/start",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task started successfully", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn start_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service.start_task(id).await?;

    Ok(Json(task.into()))
}

/// Complete task with PR information
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/complete",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = CompleteTaskRequest,
    responses(
        (status = 200, description = "Task completed successfully", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn complete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<CompleteTaskRequest>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service
        .complete_task(id, req.pr_number, req.pr_url, req.branch_name)
        .await?;

    Ok(Json(task.into()))
}

/// Mark task as failed
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/fail",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = FailTaskRequest,
    responses(
        (status = 200, description = "Task marked as failed", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn fail_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<FailTaskRequest>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service.fail_task(id, req.error_message).await?;

    Ok(Json(task.into()))
}

/// Retry a failed task
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/retry",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task retry initiated", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn retry_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service.retry_task(id).await?;

    Ok(Json(task.into()))
}

/// Cancel a task
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/cancel",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task cancelled successfully", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn cancel_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());

    let task = service.cancel_task(id).await?;

    Ok(Json(task.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::state::create_test_state;
    use axum::extract::Query;

    /// Test create_task handler creates task successfully
    /// Requirements: Task API - create task endpoint
    #[tokio::test]
    async fn test_create_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;

        let req = CreateTaskRequest {
            workspace_id: workspace.id,
            issue_number: 123,
            issue_title: "Test Issue".to_string(),
            issue_body: Some("Issue body".to_string()),
            assigned_agent_id: None,
            priority: "high".to_string(),
        };

        // Act
        let result = create_task(State(state), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.workspace_id, workspace.id);
        assert_eq!(response.issue_number, 123);
        assert_eq!(response.issue_title, "Test Issue");
        assert_eq!(response.task_status, "pending");
    }

    /// Test get_task handler returns task when exists
    /// Requirements: Task API - get task endpoint
    #[tokio::test]
    async fn test_get_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        // Act
        let result = get_task(State(state), Path(task.id)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.id, task.id);
        assert_eq!(response.issue_number, task.issue_number);
    }

    /// Test get_task handler returns 404 when task not found
    /// Requirements: Task API - error handling
    #[tokio::test]
    async fn test_get_task_handler_not_found() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");

        // Act
        let result = get_task(State(state), Path(99999)).await;

        // Assert
        assert!(result.is_err());
    }

    /// Test list_tasks_by_workspace handler returns tasks
    /// Requirements: Task API - list tasks endpoint
    #[tokio::test]
    async fn test_list_tasks_by_workspace_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        create_test_task(&state, workspace.id).await;
        create_test_task(&state, workspace.id).await;

        let query = ListTasksQuery {
            workspace_id: workspace.id,
            status: None,
            priority: None,
            assigned_agent_id: None,
        };

        // Act
        let result = list_tasks_by_workspace(State(state), Query(query)).await;

        // Assert
        assert!(result.is_ok());
        let Json(responses) = result.unwrap();
        assert_eq!(responses.len(), 2);
    }

    /// Test update_task_status handler updates status
    /// Requirements: Task API - update task status endpoint
    #[tokio::test]
    async fn test_update_task_status_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        let req = UpdateTaskStatusRequest {
            status: "in_progress".to_string(),
        };

        // Act
        let result = update_task_status(State(state), Path(task.id), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.task_status, "in_progress");
    }

    /// Test update_task handler updates priority
    /// Requirements: Task API - update task endpoint
    #[tokio::test]
    async fn test_update_task_handler_priority() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        let req = UpdateTaskRequest {
            priority: Some("high".to_string()),
            assigned_agent_id: None,
        };

        // Act
        let result = update_task(State(state), Path(task.id), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.priority, "high");
    }

    /// Test delete_task handler soft deletes task
    /// Requirements: Task API - delete task endpoint
    #[tokio::test]
    async fn test_delete_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        // Act
        let result = delete_task(State(state.clone()), Path(task.id)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        // Verify task is marked as deleted
        let service = TaskService::new(state.db.clone());
        let deleted_task = service.get_task_by_id(task.id).await.unwrap();
        assert!(deleted_task.deleted_at.is_some());
    }

    /// Test assign_agent handler assigns agent
    /// Requirements: Task API - assign agent endpoint
    #[tokio::test]
    async fn test_assign_agent_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        let req = AssignAgentRequest { agent_id: None };

        // Act
        let result = assign_agent(State(state), Path(task.id), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.assigned_agent_id, None);
        assert_eq!(response.task_status, "assigned");
    }

    /// Test start_task handler starts task
    /// Requirements: Task API - start task endpoint
    #[tokio::test]
    async fn test_start_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        // Act
        let result = start_task(State(state), Path(task.id)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.task_status, "running");
        assert!(response.started_at.is_some());
    }

    /// Test complete_task handler completes task
    /// Requirements: Task API - complete task endpoint
    #[tokio::test]
    async fn test_complete_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        let req = CompleteTaskRequest {
            pr_number: 456,
            pr_url: "https://git.example.com/owner/repo/pulls/456".to_string(),
            branch_name: "fix/test-branch".to_string(),
        };

        // Act
        let result = complete_task(State(state), Path(task.id), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.task_status, "completed");
        assert_eq!(response.pr_number, Some(456));
        assert_eq!(
            response.pr_url,
            Some("https://git.example.com/owner/repo/pulls/456".to_string())
        );
        assert_eq!(response.branch_name, Some("fix/test-branch".to_string()));
        assert!(response.completed_at.is_some());
    }

    /// Test fail_task handler marks task as failed
    /// Requirements: Task API - fail task endpoint
    #[tokio::test]
    async fn test_fail_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        let req = FailTaskRequest {
            error_message: "Test error".to_string(),
        };

        // Act
        let result = fail_task(State(state), Path(task.id), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.retry_count, 1);
        assert_eq!(response.error_message, Some("Test error".to_string()));
        assert_eq!(response.task_status, "pending"); // Should retry
    }

    /// Test retry_task handler retries task
    /// Requirements: Task API - retry task endpoint
    #[tokio::test]
    async fn test_retry_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        // Fail the task first
        let service = TaskService::new(state.db.clone());
        service
            .fail_task(task.id, "Test error".to_string())
            .await
            .unwrap();

        // Act
        let result = retry_task(State(state), Path(task.id)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.task_status, "pending");
        assert_eq!(response.error_message, None);
    }

    /// Test cancel_task handler cancels task
    /// Requirements: Task API - cancel task endpoint
    #[tokio::test]
    async fn test_cancel_task_handler_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        // Act
        let result = cancel_task(State(state), Path(task.id)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.task_status, "cancelled");
    }

    /// Test list_tasks_by_workspace with filters
    /// Requirements: Task API - list tasks with filters
    #[tokio::test]
    async fn test_list_tasks_by_workspace_with_filters() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;

        // Create tasks with different statuses
        let service = TaskService::new(state.db.clone());
        let task1 = service
            .create_task(
                workspace.id,
                200,
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
                201,
                "Task 2".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Start one task
        service.start_task(task2.id).await.unwrap();

        let query = ListTasksQuery {
            workspace_id: workspace.id,
            status: Some("pending".to_string()),
            priority: None,
            assigned_agent_id: None,
        };

        // Act
        let result = list_tasks_by_workspace(State(state), Query(query)).await;

        // Assert
        assert!(result.is_ok());
        let Json(responses) = result.unwrap();
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].id, task1.id);
        assert_eq!(responses[0].task_status, "pending");
    }

    // Helper functions
    async fn create_test_workspace(state: &Arc<AppState>) -> crate::entities::workspace::Model {
        use crate::entities::{prelude::*, workspace};
        use sea_orm::{EntityTrait, Set};

        let repo = create_test_repository(state).await;
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
        Workspace::insert(ws)
            .exec_with_returning(&state.db)
            .await
            .unwrap()
    }

    async fn create_test_repository(state: &Arc<AppState>) -> crate::entities::repository::Model {
        use crate::entities::{prelude::*, repo_provider, repository};
        use sea_orm::{EntityTrait, Set};

        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec(&state.db)
            .await
            .unwrap();

        let repo = repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.last_insert_id),
            ..Default::default()
        };
        Repository::insert(repo)
            .exec_with_returning(&state.db)
            .await
            .unwrap()
    }

    async fn create_test_task(
        state: &Arc<AppState>,
        workspace_id: i32,
    ) -> crate::entities::task::Model {
        use std::time::{SystemTime, UNIX_EPOCH};
        let service = TaskService::new(state.db.clone());
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i32;
        service
            .create_task(
                workspace_id,
                timestamp.abs(),
                "Test Task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap()
    }
}
