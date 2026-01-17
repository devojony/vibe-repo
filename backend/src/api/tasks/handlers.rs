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
}

/// List tasks by workspace
#[utoipa::path(
    get,
    path = "/api/tasks",
    params(
        ("workspace_id" = i32, Query, description = "Workspace ID")
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

    let tasks = service.list_tasks_by_workspace(query.workspace_id).await?;

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
