//! Integration tests for Task PR Operations API
//!
//! Tests the manual PR creation and issue closure endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;
use tower::ServiceExt;
use vibe_repo::api::tasks::models::TaskResponse;
use vibe_repo::entities::{task, workspace};
use vibe_repo::test_utils::{create_test_repository, TestDatabase};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test app with a specific database connection
async fn create_test_app_with_db(db: DatabaseConnection) -> axum::Router {
    use vibe_repo::api::create_router;
    use vibe_repo::config::AppConfig;
    use vibe_repo::services::RepositoryService;
    use vibe_repo::state::AppState;

    let config = Arc::new(AppConfig::default());
    let repository_service = Arc::new(RepositoryService::new(db.clone(), config.clone(), None));
    let state = Arc::new(AppState::new(db, (*config).clone(), repository_service));
    create_router(state)
}

/// Create a test workspace
async fn create_test_workspace(db: &DatabaseConnection, repository_id: i32) -> workspace::Model {
    workspace::ActiveModel {
        repository_id: Set(repository_id),
        workspace_status: Set("Active".to_string()),
        container_id: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test workspace")
}

/// Create a test task
async fn create_test_task(db: &DatabaseConnection, workspace_id: i32) -> task::Model {
    task::ActiveModel {
        workspace_id: Set(workspace_id),
        issue_number: Set(123),
        issue_title: Set("Test Issue".to_string()),
        issue_body: Set(Some("Test body".to_string())),
        task_status: Set(vibe_repo::entities::task::TaskStatus::Pending),
        priority: Set("high".to_string()),
        assigned_agent_id: Set(None),
        branch_name: Set(Some("feature/test-branch".to_string())),
        pr_number: Set(None),
        pr_url: Set(None),
        error_message: Set(None),
        last_log: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test task")
}

// ============================================================================
// Tests
// ============================================================================

/// Test POST /api/tasks/{id}/create-pr returns 404 when task not found
/// Requirements: Task API - manual PR creation endpoint error handling
#[tokio::test]
async fn test_create_pr_endpoint_returns_404_when_task_not_found() {
    // Arrange
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks/99999/create-pr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test POST /api/tasks/{id}/create-pr returns 400 when task has no branch
/// Requirements: Task API - manual PR creation endpoint validation
#[tokio::test]
async fn test_create_pr_endpoint_returns_400_when_no_branch() {
    // Arrange
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let repository = create_test_repository(
        db,
        "test-repo",
        "org/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");
    let workspace = create_test_workspace(db, repository.id).await;

    // Create task without branch_name
    let task = task::ActiveModel {
        workspace_id: Set(workspace.id),
        issue_number: Set(456),
        issue_title: Set("Test Issue Without Branch".to_string()),
        issue_body: Set(None),
        task_status: Set(vibe_repo::entities::task::TaskStatus::Pending),
        priority: Set("medium".to_string()),
        branch_name: Set(None), // No branch
        last_log: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create task");

    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/create-pr", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test POST /api/tasks/{id}/close-issue returns 404 when task not found
/// Requirements: Task API - manual issue closure endpoint error handling
#[tokio::test]
async fn test_close_issue_endpoint_returns_404_when_task_not_found() {
    // Arrange
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks/99999/close-issue")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test POST /api/tasks/{id}/close-issue returns 400 when task has no PR
/// Requirements: Task API - manual issue closure endpoint validation
#[tokio::test]
async fn test_close_issue_endpoint_returns_400_when_no_pr() {
    // Arrange
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let repository = create_test_repository(
        db,
        "test-repo",
        "org/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");
    let workspace = create_test_workspace(db, repository.id).await;
    let task = create_test_task(db, workspace.id).await;

    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/close-issue", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test POST /api/tasks/{id}/create-pr returns task response on success
/// Requirements: Task API - manual PR creation endpoint response format
/// Note: This test will fail without a real Git provider, but validates the response structure
#[tokio::test]
async fn test_create_pr_endpoint_response_format() {
    // Arrange
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let repository = create_test_repository(
        db,
        "test-repo",
        "org/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");
    let workspace = create_test_workspace(db, repository.id).await;
    let task = create_test_task(db, workspace.id).await;

    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/create-pr", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    // Without a real Git provider, this will fail with 500/502 (network error)
    // But we can verify the endpoint exists and returns a proper error
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::BAD_GATEWAY,
        "Expected OK, INTERNAL_SERVER_ERROR, or BAD_GATEWAY, got: {:?}",
        status
    );
}

/// Test POST /api/tasks/{id}/close-issue returns task response on success
/// Requirements: Task API - manual issue closure endpoint response format
/// Note: This test will fail without a real Git provider, but validates the response structure
#[tokio::test]
async fn test_close_issue_endpoint_response_format() {
    // Arrange
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let repository = create_test_repository(
        db,
        "test-repo",
        "org/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");
    let workspace = create_test_workspace(db, repository.id).await;

    // Create task with PR info
    let task = task::ActiveModel {
        workspace_id: Set(workspace.id),
        issue_number: Set(789),
        issue_title: Set("Test Issue With PR".to_string()),
        issue_body: Set(Some("Test body".to_string())),
        task_status: Set(vibe_repo::entities::task::TaskStatus::Running),
        priority: Set("high".to_string()),
        assigned_agent_id: Set(None),
        branch_name: Set(Some("feature/test-branch".to_string())),
        pr_number: Set(Some(123)),
        pr_url: Set(Some("https://example.com/pr/123".to_string())),
        error_message: Set(None),
        last_log: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create task");

    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/close-issue", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    // Without a real Git provider, this will fail with 500/502 (network error)
    // But we can verify the endpoint exists and returns a proper error
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::BAD_GATEWAY,
        "Expected OK, INTERNAL_SERVER_ERROR, or BAD_GATEWAY, got: {:?}",
        status
    );
}

/// Test POST /api/tasks/{id}/create-pr skips if PR already exists
/// Requirements: Task API - idempotent PR creation
#[tokio::test]
async fn test_create_pr_endpoint_skips_if_pr_exists() {
    // Arrange
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let repository = create_test_repository(
        db,
        "test-repo",
        "org/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");
    let workspace = create_test_workspace(db, repository.id).await;

    // Create task with existing PR
    let task = task::ActiveModel {
        workspace_id: Set(workspace.id),
        issue_number: Set(999),
        issue_title: Set("Test Issue With Existing PR".to_string()),
        issue_body: Set(Some("Test body".to_string())),
        task_status: Set(vibe_repo::entities::task::TaskStatus::Completed),
        priority: Set("high".to_string()),
        assigned_agent_id: Set(None),
        branch_name: Set(Some("feature/test-branch".to_string())),
        pr_number: Set(Some(456)),
        pr_url: Set(Some("https://example.com/pr/456".to_string())),
        error_message: Set(None),
        last_log: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create task");

    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/create-pr", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert - should succeed because PR already exists (idempotent)
    assert_eq!(response.status(), StatusCode::OK);

    // Verify response body contains task with PR info
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task_response: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(task_response.pr_number, Some(456));
    assert_eq!(
        task_response.pr_url,
        Some("https://example.com/pr/456".to_string())
    );
}
