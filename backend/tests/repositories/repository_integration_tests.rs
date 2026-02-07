//! Integration tests for Repository API
//!
//! Tests the full HTTP request/response cycle for repository operations.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;
use vibe_repo::api::repositories::models::RepositoryResponse;
use vibe_repo::test_utils::{create_test_repository, create_test_state};

// Test list with no filters
// Requirements: 12.1, 12.2, 12.4, 12.5
#[tokio::test]
async fn test_list_repositories_no_filters() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create multiple repositories with different providers
    let _repo1 = create_test_repository(
        &state.db,
        "repo1",
        "owner/repo1",
        "github",
        "https://api.github.com",
        "test_token_1",
    )
    .await
    .expect("Failed to create repo1");

    let _repo2 = create_test_repository(
        &state.db,
        "repo2",
        "owner/repo2",
        "gitea",
        "https://gitea.example.com",
        "test_token_2",
    )
    .await
    .expect("Failed to create repo2");

    // List all repositories
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/repositories")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repositories: Vec<RepositoryResponse> = serde_json::from_slice(&body).unwrap();

    // Should return at least 2 repositories
    assert!(repositories.len() >= 2);

    // Verify all required fields are present
    for repo in repositories {
        assert!(repo.id > 0);
        assert!(!repo.provider_type.is_empty());
        assert!(!repo.provider_base_url.is_empty());
        assert!(!repo.name.is_empty());
        assert!(!repo.full_name.is_empty());
        assert!(!repo.clone_url.is_empty());
        assert!(!repo.default_branch.is_empty());
        assert!(!repo.created_at.is_empty());
        assert!(!repo.updated_at.is_empty());
    }
}

// Test list with validation_status filter
// Requirements: 12.2, 12.3
#[tokio::test]
async fn test_list_repositories_with_status_filter() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create repositories (all created with Valid status by default)
    let _repo1 = create_test_repository(
        &state.db,
        "valid-repo",
        "owner/valid-repo",
        "github",
        "https://api.github.com",
        "test_token_1",
    )
    .await
    .expect("Failed to create valid-repo");

    // List only valid repositories
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/repositories?validation_status=valid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repositories: Vec<RepositoryResponse> = serde_json::from_slice(&body).unwrap();

    // Should return at least one valid repository
    assert!(!repositories.is_empty());
}

// Test list with invalid validation_status
// Requirements: 12.3
#[tokio::test]
async fn test_list_repositories_with_invalid_status() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/repositories?validation_status=invalid_status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Test list returns empty array when no repositories
// Requirements: 12.4
#[tokio::test]
async fn test_list_repositories_empty() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/repositories")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repositories: Vec<RepositoryResponse> = serde_json::from_slice(&body).unwrap();

    assert_eq!(repositories.len(), 0);
}

// Test get repository by ID
// Requirements: 13.1, 13.3
#[tokio::test]
async fn test_get_repository_by_id() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create a repository
    let repo = create_test_repository(
        &state.db,
        "test-repo",
        "owner/test-repo",
        "github",
        "https://api.github.com",
        "test_token_12345678",
    )
    .await
    .expect("Failed to create test-repo");

    // Get the repository
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/repositories/{}", repo.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repository: RepositoryResponse = serde_json::from_slice(&body).unwrap();

    // Verify all fields
    assert_eq!(repository.id, repo.id);
    assert_eq!(repository.provider_type, "github");
    assert_eq!(repository.provider_base_url, "https://api.github.com");
    assert_eq!(repository.name, "test-repo");
    assert_eq!(repository.full_name, "owner/test-repo");
    assert!(!repository.created_at.is_empty());
    assert!(!repository.updated_at.is_empty());
}

// Test get non-existent repository
// Requirements: 13.2
#[tokio::test]
async fn test_get_nonexistent_repository() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/repositories/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test refresh repository updates validation
// Requirements: 14.1, 14.2, 14.3, 14.4, 14.5, 14.7
#[tokio::test]
#[ignore] // Ignore by default as it requires external Git provider instance
async fn test_refresh_repository_updates_validation() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create a repository with valid test credentials
    let repo = create_test_repository(
        &state.db,
        "test-repo",
        "testuser/test-repo",
        "gitea",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await
    .expect("Failed to create test-repo");

    // Refresh the repository
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/repositories/{}/refresh", repo.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repository: RepositoryResponse = serde_json::from_slice(&body).unwrap();

    // Updated timestamp should be recent
    assert!(!repository.updated_at.is_empty());
}

// Test refresh non-existent repository
// Requirements: 14.6
#[tokio::test]
async fn test_refresh_nonexistent_repository() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories/99999/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test POST /api/repositories - Add new repository
// Requirements: 10.3 - New integration test for POST endpoint
#[tokio::test]
#[ignore] // Ignore by default as it requires external Git provider instance
async fn test_add_repository_success() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let request_body = json!({
        "provider_type": "github",
        "provider_base_url": "https://api.github.com",
        "access_token": "valid_github_token",
        "full_name": "owner/test-repo",
        "branch_name": "vibe-dev"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repository: RepositoryResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(repository.provider_type, "github");
    assert_eq!(repository.provider_base_url, "https://api.github.com");
    assert_eq!(repository.full_name, "owner/test-repo");
}

// Test POST /api/repositories - Token validation failure
// Requirements: 10.4 - Test token validation failure scenario
#[tokio::test]
#[ignore] // Ignore by default as it requires external Git provider instance
async fn test_add_repository_invalid_token() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let request_body = json!({
        "provider_type": "github",
        "provider_base_url": "https://api.github.com",
        "access_token": "invalid_token",
        "full_name": "owner/test-repo",
        "branch_name": "vibe-dev"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 401 Unauthorized for invalid token
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// Test POST /api/repositories - Repository not found
// Requirements: 10.5 - Test repository not found scenario
#[tokio::test]
#[ignore] // Ignore by default as it requires external Git provider instance
async fn test_add_repository_not_found() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let request_body = json!({
        "provider_type": "github",
        "provider_base_url": "https://api.github.com",
        "access_token": "valid_token",
        "full_name": "nonexistent/nonexistent-repo",
        "branch_name": "vibe-dev"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 404 Not Found for nonexistent repository
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test POST /api/repositories - Insufficient permissions
// Requirements: 10.6 - Test insufficient permissions scenario
#[tokio::test]
#[ignore] // Ignore by default as it requires external Git provider instance
async fn test_add_repository_insufficient_permissions() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let request_body = json!({
        "provider_type": "github",
        "provider_base_url": "https://api.github.com",
        "access_token": "read_only_token",
        "full_name": "owner/test-repo",
        "branch_name": "vibe-dev"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 403 Forbidden for insufficient permissions
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// Test POST /api/repositories - Invalid request body
#[tokio::test]
async fn test_add_repository_invalid_request() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    let request_body = json!({
        "provider_type": "invalid_provider",
        // Missing required fields
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 422 Unprocessable Entity for invalid request body (Axum's default for JSON deserialization errors)
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// Test POST /api/repositories - Duplicate repository
#[tokio::test]
async fn test_add_repository_duplicate() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Create a repository first
    let _repo = create_test_repository(
        &state.db,
        "test-repo",
        "owner/test-repo",
        "github",
        "https://api.github.com",
        "test_token",
    )
    .await
    .expect("Failed to create test-repo");

    let request_body = json!({
        "provider_type": "github",
        "provider_base_url": "https://api.github.com",
        "access_token": "test_token",
        "full_name": "owner/test-repo",
        "branch_name": "vibe-dev"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repositories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 409 Conflict for duplicate repository
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
