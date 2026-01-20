//! Integration tests for Repository Polling API
//!
//! Tests the full HTTP request/response cycle for repository polling operations.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;
use vibe_repo::api::repositories::models::{PollIssuesResponse, RepositoryResponse};
use vibe_repo::entities::repository::ValidationStatus;
use vibe_repo::test_utils::state::create_test_state;

/// Helper function to create a test provider and return its ID
async fn create_test_provider(app: axum::Router, name: &str, base_url: &str, token: &str) -> i32 {
    let request_body = json!({
        "name": name,
        "provider_type": "gitea",
        "base_url": base_url,
        "access_token": token
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/providers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let provider: serde_json::Value = serde_json::from_slice(&body).unwrap();
    provider["id"].as_i64().unwrap() as i32
}

/// Helper function to create a test repository directly in the database
async fn create_test_repository(
    state: std::sync::Arc<vibe_repo::state::AppState>,
    provider_id: i32,
    name: &str,
    full_name: &str,
    validation_status: ValidationStatus,
) -> i32 {
    use sea_orm::{ActiveModelTrait, Set};
    use vibe_repo::entities::repository;

    let repo = repository::ActiveModel {
        provider_id: Set(provider_id),
        name: Set(name.to_string()),
        full_name: Set(full_name.to_string()),
        clone_url: Set(format!("https://gitea.example.com/{}.git", full_name)),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main", "dev"])),
        validation_status: Set(validation_status),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        polling_enabled: Set(false),
        polling_interval_seconds: Set(Some(300)), // Default value from migration
        ..Default::default()
    };

    let created = repo.insert(&state.db).await.unwrap();
    created.id
}

/// Test PATCH /api/repositories/:id/polling - Success case
///
/// Verifies that polling configuration can be successfully updated.
#[tokio::test]
async fn test_update_repository_polling_success() {
    // Arrange: Create test database and app
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create test provider and repository
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    let repo_id = create_test_repository(
        state.clone(),
        provider_id,
        "test-repo",
        "owner/test-repo",
        ValidationStatus::Valid,
    )
    .await;

    // Act: Send PATCH request to update polling configuration
    let request_body = json!({
        "enabled": true,
        "interval_seconds": 300
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/repositories/{}/polling", repo_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Verify response status code 200
    assert_eq!(response.status(), StatusCode::OK);

    // Assert: Verify response body contains updated configuration
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repo_response: RepositoryResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(repo_response.id, repo_id);
    // Note: RepositoryResponse doesn't include polling fields yet
    // This will need to be updated when the model is extended

    // Assert: Verify database was updated
    use sea_orm::EntityTrait;
    use vibe_repo::entities::prelude::Repository;

    let updated_repo = Repository::find_by_id(repo_id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert!(updated_repo.polling_enabled);
    assert_eq!(updated_repo.polling_interval_seconds, Some(300));
}

/// Test PATCH /api/repositories/:id/polling - Repository not found
///
/// Verifies that updating polling for a non-existent repository returns 404.
#[tokio::test]
async fn test_update_repository_polling_not_found() {
    // Arrange: Create test database and app
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Send PATCH request to non-existent repository
    let request_body = json!({
        "enabled": true,
        "interval_seconds": 300
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/repositories/99999/polling")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Verify response status code 404
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test PATCH /api/repositories/:id/polling - Invalid interval
///
/// Verifies that interval_seconds < 60 returns 400 Bad Request.
#[tokio::test]
async fn test_update_repository_polling_invalid_interval() {
    // Arrange: Create test database and app
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create test provider and repository
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    let repo_id = create_test_repository(
        state.clone(),
        provider_id,
        "test-repo",
        "owner/test-repo",
        ValidationStatus::Valid,
    )
    .await;

    // Act: Send PATCH request with interval_seconds < 60
    let request_body = json!({
        "enabled": true,
        "interval_seconds": 30
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/repositories/{}/polling", repo_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Verify response status code 400
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test POST /api/repositories/:id/poll-issues - Success case
///
/// Verifies that manual issue polling can be successfully triggered.
#[tokio::test]
async fn test_trigger_issue_polling_success() {
    // Arrange: Create test database and app
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create test provider and repository
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    let repo_id = create_test_repository(
        state.clone(),
        provider_id,
        "test-repo",
        "owner/test-repo",
        ValidationStatus::Valid,
    )
    .await;

    // Act: Send POST request to trigger polling
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/repositories/{}/poll-issues", repo_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Verify response status code 200
    assert_eq!(response.status(), StatusCode::OK);

    // Assert: Verify response body contains success=true
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let poll_response: PollIssuesResponse = serde_json::from_slice(&body).unwrap();

    assert!(poll_response.success);
    assert_eq!(poll_response.message, "Polling triggered");
}

/// Test POST /api/repositories/:id/poll-issues - Repository not found
///
/// Verifies that triggering polling for a non-existent repository returns 404.
#[tokio::test]
async fn test_trigger_issue_polling_not_found() {
    // Arrange: Create test database and app
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Send POST request to non-existent repository
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories/99999/poll-issues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Verify response status code 404
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test PATCH /api/repositories/:id/polling - Disable polling
///
/// Verifies that polling can be disabled.
#[tokio::test]
async fn test_update_repository_polling_disable() {
    // Arrange: Create test database and app
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create test provider and repository
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    let repo_id = create_test_repository(
        state.clone(),
        provider_id,
        "test-repo",
        "owner/test-repo",
        ValidationStatus::Valid,
    )
    .await;

    // First enable polling
    let enable_body = json!({
        "enabled": true,
        "interval_seconds": 300
    });

    let _enable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/repositories/{}/polling", repo_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&enable_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Act: Disable polling
    let disable_body = json!({
        "enabled": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/repositories/{}/polling", repo_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&disable_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Verify response status code 200
    assert_eq!(response.status(), StatusCode::OK);

    // Assert: Verify database was updated
    use sea_orm::EntityTrait;
    use vibe_repo::entities::prelude::Repository;

    let updated_repo = Repository::find_by_id(repo_id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert!(!updated_repo.polling_enabled);
}

/// Test PATCH /api/repositories/:id/polling - Update interval only
///
/// Verifies that interval can be updated without changing enabled status.
#[tokio::test]
async fn test_update_repository_polling_interval_only() {
    // Arrange: Create test database and app
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create test provider and repository
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    let repo_id = create_test_repository(
        state.clone(),
        provider_id,
        "test-repo",
        "owner/test-repo",
        ValidationStatus::Valid,
    )
    .await;

    // First enable polling with 300 seconds
    let initial_body = json!({
        "enabled": true,
        "interval_seconds": 300
    });

    let _initial_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/repositories/{}/polling", repo_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&initial_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Act: Update interval to 600 seconds while keeping enabled=true
    let update_body = json!({
        "enabled": true,
        "interval_seconds": 600
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/repositories/{}/polling", repo_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Verify response status code 200
    assert_eq!(response.status(), StatusCode::OK);

    // Assert: Verify database was updated with new interval
    use sea_orm::EntityTrait;
    use vibe_repo::entities::prelude::Repository;

    let updated_repo = Repository::find_by_id(repo_id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();

    assert!(updated_repo.polling_enabled);
    assert_eq!(updated_repo.polling_interval_seconds, Some(600));
}
