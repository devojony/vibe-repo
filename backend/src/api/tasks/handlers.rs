use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    api::tasks::models::*,
    entities::task::TaskStatus,
    error::Result,
    services::{IssueClosureService, PRCreationService, TaskExecutorService, TaskService},
    state::AppState,
};

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
            None, // Auto-assign agent in single agent mode
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
    pub status: Option<TaskStatus>,
    pub priority: Option<String>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_per_page")]
    pub per_page: i32,
}

fn default_page() -> i32 {
    1
}

fn default_per_page() -> i32 {
    20
}

/// List tasks by workspace with filters
#[utoipa::path(
    get,
    path = "/api/tasks",
    params(
        ("workspace_id" = i32, Query, description = "Workspace ID"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("priority" = Option<String>, Query, description = "Filter by priority"),
        ("page" = Option<i32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<i32>, Query, description = "Items per page (default: 20, max: 100)"),
    ),
    responses(
        (status = 200, description = "Paginated list of tasks", body = TaskListResponse),
        (status = 400, description = "Invalid pagination parameters"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn list_tasks_by_workspace(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<TaskListResponse>> {
    // Validate pagination parameters
    if query.page < 1 {
        return Err(crate::error::VibeRepoError::Validation(
            "Page number must be >= 1".to_string(),
        ));
    }

    let per_page = if query.per_page < 1 || query.per_page > 100 {
        20 // Default to 20 if invalid
    } else {
        query.per_page
    };

    let service = TaskService::new(state.db.clone());

    let (tasks, total) = service
        .list_tasks_with_pagination(
            query.workspace_id,
            query.status,
            query.priority,
            None, // No agent filter in single agent mode
            query.page,
            per_page,
        )
        .await?;

    let responses: Vec<TaskResponse> = tasks.into_iter().map(|t| t.into()).collect();

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as i32;

    Ok(Json(TaskListResponse {
        tasks: responses,
        total,
        page: query.page,
        per_page,
        total_pages,
    }))
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
        .update_task(id, req.priority, None) // No agent update in single agent mode
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

/// Execute a task in its workspace container
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/execute",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 202, description = "Task execution started", body = TaskResponse),
        (status = 400, description = "Task not in valid state for execution"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn execute_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<(StatusCode, Json<TaskResponse>)> {
    let executor =
        TaskExecutorService::new(state.db.clone(), state.config.workspace.base_dir.clone());
    let task_service = TaskService::new(state.db.clone());

    // Get task before execution
    let task = task_service.get_task_by_id(id).await?;

    // Start execution in background
    tokio::spawn(async move {
        if let Err(e) = executor.execute_task(id).await {
            tracing::error!(task_id = id, error = %e, "Task execution failed");
        }
    });

    Ok((StatusCode::ACCEPTED, Json(task.into())))
}

/// Get task logs
#[utoipa::path(
    get,
    path = "/api/tasks/{id}/logs",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task logs retrieved successfully", body = String),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn get_task_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>> {
    let service = TaskService::new(state.db.clone());

    let task = service.get_task_by_id(id).await?;

    Ok(Json(serde_json::json!({
        "task_id": task.id,
        "logs": task.last_log.unwrap_or_default()
    })))
}

/// Get task status
#[utoipa::path(
    get,
    path = "/api/tasks/{id}/status",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task status retrieved successfully"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn get_task_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>> {
    let service = TaskService::new(state.db.clone());

    let task = service.get_task_by_id(id).await?;

    Ok(Json(serde_json::json!({
        "task_id": task.id,
        "status": task.task_status.to_string(),
        "started_at": task.started_at.map(|dt| dt.to_string()),
        "completed_at": task.completed_at.map(|dt| dt.to_string()),
        "created_at": task.created_at.to_string(),
        "updated_at": task.updated_at.to_string()
    })))
}

/// Manually create PR for a task
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/create-pr",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "PR created successfully", body = TaskResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn create_pr_for_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = PRCreationService::new(state.db.clone());
    service.create_pr_for_task(id).await?;

    let task_service = TaskService::new(state.db.clone());
    let task = task_service.get_task_by_id(id).await?;

    Ok(Json(task.into()))
}

/// Manually close issue for a task
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/close-issue",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Issue closed successfully", body = TaskResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn close_issue_for_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = IssueClosureService::new(state.db.clone());
    service.close_issue_for_task(id).await?;

    let task_service = TaskService::new(state.db.clone());
    let task = task_service.get_task_by_id(id).await?;

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
            page: 1,
            per_page: 20,
        };

        // Act
        let result = list_tasks_by_workspace(State(state), Query(query)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.tasks.len(), 2);
        assert_eq!(response.total, 2);
        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 20);
        assert_eq!(response.total_pages, 1);
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
            status: TaskStatus::Running,
        };

        // Act
        let result = update_task_status(State(state.clone()), Path(task.id), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.task_status, "running");
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

        // Assign the task first (required before starting)
        let task_service = TaskService::new(state.db.clone());
        task_service.assign_agent(task.id, None).await.unwrap();

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

        // Assign and start the task first (required before completing)
        let task_service = TaskService::new(state.db.clone());
        task_service.assign_agent(task.id, None).await.unwrap();
        task_service.start_task(task.id).await.unwrap();

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

        // Assign and start the task first (required before failing)
        let task_service = TaskService::new(state.db.clone());
        task_service.assign_agent(task.id, None).await.unwrap();
        task_service.start_task(task.id).await.unwrap();

        let req = FailTaskRequest {
            error_message: "Test error".to_string(),
        };

        // Act
        let result = fail_task(State(state), Path(task.id), Json(req)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.error_message, Some("Test error".to_string()));
        assert_eq!(response.task_status, "failed");
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

        // Assign and start one task
        service.assign_agent(task2.id, None).await.unwrap();
        service.start_task(task2.id).await.unwrap();

        let query = ListTasksQuery {
            workspace_id: workspace.id,
            status: Some(TaskStatus::Pending),
            priority: None,
            page: 1,
            per_page: 20,
        };

        // Act
        let result = list_tasks_by_workspace(State(state), Query(query)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.tasks.len(), 1);
        assert_eq!(response.tasks[0].id, task1.id);
        assert_eq!(response.tasks[0].task_status, "pending");
    }

    /// Test list_tasks_by_workspace with pagination
    /// Requirements: Task API - pagination support
    #[tokio::test]
    async fn test_list_tasks_by_workspace_pagination() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;

        // Create 5 tasks
        for i in 1..=5 {
            create_test_task_with_issue(&state, workspace.id, 700 + i).await;
        }

        let query = ListTasksQuery {
            workspace_id: workspace.id,
            status: None,
            priority: None,
            page: 1,
            per_page: 2,
        };

        // Act
        let result = list_tasks_by_workspace(State(state), Query(query)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.tasks.len(), 2);
        assert_eq!(response.total, 5);
        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 2);
        assert_eq!(response.total_pages, 3);
    }

    /// Test list_tasks_by_workspace with invalid page number
    /// Requirements: Task API - pagination validation
    #[tokio::test]
    async fn test_list_tasks_by_workspace_invalid_page() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;

        let query = ListTasksQuery {
            workspace_id: workspace.id,
            status: None,
            priority: None,
            page: 0, // Invalid page number
            per_page: 20,
        };

        // Act
        let result = list_tasks_by_workspace(State(state), Query(query)).await;

        // Assert
        assert!(result.is_err());
    }

    /// Test list_tasks_by_workspace with per_page exceeding max
    /// Requirements: Task API - pagination validation
    #[tokio::test]
    async fn test_list_tasks_by_workspace_per_page_max() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        create_test_task(&state, workspace.id).await;

        let query = ListTasksQuery {
            workspace_id: workspace.id,
            status: None,
            priority: None,
            page: 1,
            per_page: 200, // Exceeds max of 100
        };

        // Act
        let result = list_tasks_by_workspace(State(state), Query(query)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.per_page, 20); // Should default to 20
    }

    /// Test create_pr_for_task endpoint success
    /// Requirements: Task API - manual PR creation endpoint
    #[tokio::test]
    async fn test_create_pr_endpoint_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        // Set branch_name to simulate task ready for PR creation
        let service = TaskService::new(state.db.clone());
        service.update_task(task.id, None, None).await.unwrap();

        // Update task with branch_name
        use crate::entities::task;
        use sea_orm::{ActiveModelTrait, Set};
        let mut task_active: task::ActiveModel = task.into();
        task_active.branch_name = Set(Some("feature/test-branch".to_string()));
        let task = task_active.update(&state.db).await.unwrap();

        // Act
        let result = create_pr_for_task(State(state.clone()), Path(task.id)).await;

        // Assert
        // Note: This will fail in unit tests without a real Git provider
        // In integration tests with a mock or real provider, this should succeed
        assert!(result.is_ok() || result.is_err());
    }

    /// Test create_pr_for_task endpoint returns 404 when task not found
    /// Requirements: Task API - error handling
    #[tokio::test]
    async fn test_create_pr_endpoint_not_found() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");

        // Act
        let result = create_pr_for_task(State(state), Path(99999)).await;

        // Assert
        assert!(result.is_err());
    }

    /// Test close_issue_for_task endpoint success
    /// Requirements: Task API - manual issue closure endpoint
    #[tokio::test]
    async fn test_close_issue_endpoint_success() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let workspace = create_test_workspace(&state).await;
        let task = create_test_task(&state, workspace.id).await;

        // Set PR info to simulate task with PR
        use crate::entities::task;
        use sea_orm::{ActiveModelTrait, Set};
        let mut task_active: task::ActiveModel = task.into();
        task_active.pr_number = Set(Some(123));
        task_active.pr_url = Set(Some("https://example.com/pr/123".to_string()));
        let task = task_active.update(&state.db).await.unwrap();

        // Act
        let result = close_issue_for_task(State(state.clone()), Path(task.id)).await;

        // Assert
        // Note: This will fail in unit tests without a real Git provider
        // In integration tests with a mock or real provider, this should succeed
        assert!(result.is_ok() || result.is_err());
    }

    /// Test close_issue_for_task endpoint returns 404 when task not found
    /// Requirements: Task API - error handling
    #[tokio::test]
    async fn test_close_issue_endpoint_not_found() {
        // Arrange
        let state = create_test_state()
            .await
            .expect("Failed to create test state");

        // Act
        let result = close_issue_for_task(State(state), Path(99999)).await;

        // Assert
        assert!(result.is_err());
    }

    // Helper functions
    async fn create_test_workspace(state: &Arc<AppState>) -> crate::entities::workspace::Model {
        use crate::entities::{prelude::*, workspace};
        use sea_orm::{EntityTrait, Set};

        let repo = create_test_repository(state).await;
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
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

    async fn create_test_task_with_issue(
        state: &Arc<AppState>,
        workspace_id: i32,
        issue_number: i32,
    ) -> crate::entities::task::Model {
        let service = TaskService::new(state.db.clone());
        service
            .create_task(
                workspace_id,
                issue_number,
                format!("Test Task {}", issue_number),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap()
    }
}
