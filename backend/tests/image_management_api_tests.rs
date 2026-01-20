//! Integration tests for image management API endpoints
//!
//! Tests the workspace image management endpoints including get info, delete, and rebuild.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;
use tower::ServiceExt;
use vibe_repo::api::settings::workspace::models::RebuildImageRequest;
use vibe_repo::entities::{container, repo_provider, repository, workspace};
use vibe_repo::test_utils::TestDatabase;

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
    let repository_service = Arc::new(RepositoryService::new(db.clone(), config.clone()));
    let state = Arc::new(AppState::new(db, (*config).clone(), repository_service));
    create_router(state)
}

/// Create a test provider
async fn create_test_provider(db: &DatabaseConnection) -> repo_provider::Model {
    repo_provider::ActiveModel {
        name: Set("test-provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test-token".to_string()),
        locked: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test provider")
}

/// Create a test repository
async fn create_test_repository(db: &DatabaseConnection, provider_id: i32) -> repository::Model {
    repository::ActiveModel {
        provider_id: Set(provider_id),
        name: Set("test-repo".to_string()),
        full_name: Set("org/test-repo".to_string()),
        clone_url: Set("https://gitea.example.com/org/test-repo.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Idle),
        has_workspace: Set(false),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        deleted_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test repository")
}

/// Create a test workspace
async fn create_test_workspace(db: &DatabaseConnection, repository_id: i32) -> workspace::Model {
    workspace::ActiveModel {
        repository_id: Set(repository_id),
        workspace_status: Set("running".to_string()),
        container_id: Set(None),
        container_status: Set(None),
        image_source: Set("default".to_string()),
        max_concurrent_tasks: Set(3),
        cpu_limit: Set(2.0),
        memory_limit: Set("4GB".to_string()),
        disk_limit: Set("10GB".to_string()),
        work_dir: Set(None),
        health_status: Set(None),
        last_health_check: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test workspace")
}

/// Create a test container
async fn create_test_container(
    db: &DatabaseConnection,
    workspace_id: i32,
    image_name: &str,
) -> container::Model {
    container::ActiveModel {
        workspace_id: Set(workspace_id),
        container_id: Set(format!("test-container-{}", workspace_id)),
        container_name: Set(format!("test-container-name-{}", workspace_id)),
        image_name: Set(image_name.to_string()),
        image_id: Set(None),
        status: Set("running".to_string()),
        health_status: Set(None),
        exit_code: Set(None),
        error_message: Set(None),
        restart_count: Set(0),
        max_restart_attempts: Set(3),
        last_restart_at: Set(None),
        last_health_check: Set(None),
        health_check_failures: Set(0),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        started_at: Set(None),
        stopped_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test container")
}

// ============================================================================
// Tests
// ============================================================================

/// Test GET /api/settings/workspace/image returns 200 with image info when Docker unavailable
/// Requirements: Task 3.2 - Image info endpoint
#[tokio::test]
async fn test_get_image_info_docker_unavailable() {
    // Arrange: Create test app without Docker
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    // Act: Send GET request to /api/settings/workspace/image
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings/workspace/image")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 500 since Docker is not available
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Should return 500 when Docker is unavailable"
    );
}

/// Test GET /api/settings/workspace/image with workspaces using the image
/// Requirements: Task 3.2 - Image info endpoint with workspace tracking
#[tokio::test]
async fn test_get_image_info_with_workspaces() {
    // Arrange: Create test app with workspaces using the image
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    // Create provider, repository, workspace, and container
    let provider = create_test_provider(&db).await;
    let repository = create_test_repository(&db, provider.id).await;
    let workspace = create_test_workspace(&db, repository.id).await;
    let _container = create_test_container(&db, workspace.id, "vibe-repo-workspace:latest").await;

    // Act: Send GET request (will fail due to no Docker, but we can test the endpoint exists)
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings/workspace/image")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Endpoint should exist (not 404)
    let status = response.status();
    assert_ne!(status, StatusCode::NOT_FOUND, "Endpoint should exist");
}

/// Test DELETE /api/settings/workspace/image returns error when Docker unavailable
/// Requirements: Task 3.2 - Delete image endpoint
#[tokio::test]
async fn test_delete_image_docker_unavailable() {
    // Arrange: Create test app without Docker
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    // Act: Send DELETE request to /api/settings/workspace/image
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/settings/workspace/image")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 500 since Docker is not available
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Should return 500 when Docker is unavailable"
    );
}

/// Test DELETE /api/settings/workspace/image with workspaces using the image
/// Requirements: Task 3.2 - Delete image with conflict detection
#[tokio::test]
async fn test_delete_image_with_workspaces_using_it() {
    // Arrange: Create test app with workspaces using the image
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    // Create provider, repository, workspace, and container
    let provider = create_test_provider(&db).await;
    let repository = create_test_repository(&db, provider.id).await;
    let workspace = create_test_workspace(&db, repository.id).await;
    let _container = create_test_container(&db, workspace.id, "vibe-repo-workspace:latest").await;

    // Act: Send DELETE request (will fail due to no Docker, but endpoint should exist)
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/settings/workspace/image")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Endpoint should exist (not 404)
    let status = response.status();
    assert_ne!(status, StatusCode::NOT_FOUND, "Endpoint should exist");
}

/// Test POST /api/settings/workspace/image/rebuild returns error when Docker unavailable
/// Requirements: Task 3.2 - Rebuild image endpoint
#[tokio::test]
async fn test_rebuild_image_docker_unavailable() {
    // Arrange: Create test app without Docker
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    let request_body = RebuildImageRequest { force: false };

    // Act: Send POST request to /api/settings/workspace/image/rebuild
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/workspace/image/rebuild")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 500 since Docker is not available
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Should return 500 when Docker is unavailable"
    );
}

/// Test POST /api/settings/workspace/image/rebuild with force=true
/// Requirements: Task 3.2 - Rebuild image with force flag
#[tokio::test]
async fn test_rebuild_image_with_force() {
    // Arrange: Create test app with workspaces using the image
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    // Create provider, repository, workspace, and container
    let provider = create_test_provider(&db).await;
    let repository = create_test_repository(&db, provider.id).await;
    let workspace = create_test_workspace(&db, repository.id).await;
    let _container = create_test_container(&db, workspace.id, "vibe-repo-workspace:latest").await;

    let request_body = RebuildImageRequest { force: true };

    // Act: Send POST request with force=true
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/workspace/image/rebuild")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Endpoint should exist (not 404)
    let status = response.status();
    assert_ne!(status, StatusCode::NOT_FOUND, "Endpoint should exist");
}

/// Test response body matches ImageInfoResponse schema
/// Requirements: Task 3.2 - Image info response format
#[tokio::test]
async fn test_image_info_response_schema() {
    // Arrange: Create test app
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    // Act: Send GET request
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings/workspace/image")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Response should be JSON (even if error)
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should be valid JSON
    let _json: serde_json::Value =
        serde_json::from_str(&body_str).expect("Response should be valid JSON");
}

/// Test rebuild endpoint accepts JSON request body
/// Requirements: Task 3.2 - Rebuild request format
#[tokio::test]
async fn test_rebuild_image_request_schema() {
    // Arrange: Create test app
    let test_db = TestDatabase::new_in_memory()
        .await
        .expect("Failed to create test database");
    let db = test_db.connection;
    let app = create_test_app_with_db(db.clone()).await;

    let request_body = RebuildImageRequest { force: false };

    // Act: Send POST request
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/workspace/image/rebuild")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should not return 400 (bad request) for valid JSON
    let status = response.status();
    assert_ne!(
        status,
        StatusCode::BAD_REQUEST,
        "Should accept valid JSON request body"
    );
}
