//! Integration tests for Repository API
//!
//! Tests the full HTTP request/response cycle for repository operations.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use gitautodev::api::repositories::models::RepositoryResponse;
use gitautodev::entities::repository::ValidationStatus;
use gitautodev::test_utils::state::create_test_state;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

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
    state: std::sync::Arc<gitautodev::state::AppState>,
    provider_id: i32,
    name: &str,
    full_name: &str,
    validation_status: ValidationStatus,
) -> i32 {
    use gitautodev::entities::repository;
    use sea_orm::{ActiveModelTrait, Set};

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
        ..Default::default()
    };

    let created = repo.insert(&state.db).await.unwrap();
    created.id
}

// Test list with no filters
// Requirements: 12.1, 12.2, 12.4, 12.5
#[tokio::test]
async fn test_list_repositories_no_filters() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Create a provider
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    // Create multiple repositories
    let _repo1 = create_test_repository(
        state.clone(),
        provider_id,
        "repo1",
        "owner/repo1",
        ValidationStatus::Valid,
    )
    .await;

    let _repo2 = create_test_repository(
        state.clone(),
        provider_id,
        "repo2",
        "owner/repo2",
        ValidationStatus::Invalid,
    )
    .await;

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
        assert_eq!(repo.provider_id, provider_id);
        assert!(!repo.name.is_empty());
        assert!(!repo.full_name.is_empty());
        assert!(!repo.clone_url.is_empty());
        assert!(!repo.default_branch.is_empty());
        assert!(!repo.created_at.is_empty());
        assert!(!repo.updated_at.is_empty());
    }
}

// Test list with provider_id filter
// Requirements: 12.2, 12.3
#[tokio::test]
async fn test_list_repositories_with_provider_filter() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Create two providers
    let provider1_id = create_test_provider(
        app.clone(),
        "Provider 1",
        "https://gitea1.example.com",
        "token1_12345678",
    )
    .await;

    let provider2_id = create_test_provider(
        app.clone(),
        "Provider 2",
        "https://gitea2.example.com",
        "token2_12345678",
    )
    .await;

    // Create repositories for both providers
    let _repo1 = create_test_repository(
        state.clone(),
        provider1_id,
        "repo1",
        "owner/repo1",
        ValidationStatus::Valid,
    )
    .await;

    let _repo2 = create_test_repository(
        state.clone(),
        provider2_id,
        "repo2",
        "owner/repo2",
        ValidationStatus::Valid,
    )
    .await;

    // List repositories for provider1 only
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/repositories?provider_id={}", provider1_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repositories: Vec<RepositoryResponse> = serde_json::from_slice(&body).unwrap();

    // Should return only repositories for provider1
    assert!(repositories.len() >= 1);
    for repo in repositories {
        assert_eq!(repo.provider_id, provider1_id);
    }
}

// Test list with validation_status filter
// Requirements: 12.2, 12.3
#[tokio::test]
async fn test_list_repositories_with_status_filter() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Create a provider
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    // Create repositories with different statuses
    let _repo1 = create_test_repository(
        state.clone(),
        provider_id,
        "valid-repo",
        "owner/valid-repo",
        ValidationStatus::Valid,
    )
    .await;

    let _repo2 = create_test_repository(
        state.clone(),
        provider_id,
        "invalid-repo",
        "owner/invalid-repo",
        ValidationStatus::Invalid,
    )
    .await;

    let _repo3 = create_test_repository(
        state.clone(),
        provider_id,
        "pending-repo",
        "owner/pending-repo",
        ValidationStatus::Pending,
    )
    .await;

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

    // Should return only valid repositories
    assert!(repositories.len() >= 1);
    for repo in repositories {
        assert_eq!(repo.validation_status, ValidationStatus::Valid);
    }
}

// Test list with invalid validation_status
// Requirements: 12.3
#[tokio::test]
async fn test_list_repositories_with_invalid_status() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

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
    let app = gitautodev::api::create_router(state.clone());

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
    let app = gitautodev::api::create_router(state.clone());

    // Create a provider
    let provider_id = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    // Create a repository
    let repo_id = create_test_repository(
        state.clone(),
        provider_id,
        "test-repo",
        "owner/test-repo",
        ValidationStatus::Valid,
    )
    .await;

    // Get the repository
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/repositories/{}", repo_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repository: RepositoryResponse = serde_json::from_slice(&body).unwrap();

    // Verify all fields
    assert_eq!(repository.id, repo_id);
    assert_eq!(repository.provider_id, provider_id);
    assert_eq!(repository.name, "test-repo");
    assert_eq!(repository.full_name, "owner/test-repo");
    assert_eq!(repository.validation_status, ValidationStatus::Valid);
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
    let app = gitautodev::api::create_router(state.clone());

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
#[ignore] // Ignore by default as it requires external Gitea instance
async fn test_refresh_repository_updates_validation() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Create a provider with valid test credentials
    let provider_id = create_test_provider(
        app.clone(),
        "Test Gitea",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    // Create a repository (assuming a real repository exists on the test instance)
    let repo_id = create_test_repository(
        state.clone(),
        provider_id,
        "test-repo",
        "testuser/test-repo",
        ValidationStatus::Pending,
    )
    .await;

    // Refresh the repository
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/repositories/{}/refresh", repo_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repository: RepositoryResponse = serde_json::from_slice(&body).unwrap();

    // Validation status should be updated (not pending anymore)
    assert_ne!(repository.validation_status, ValidationStatus::Pending);
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
    let app = gitautodev::api::create_router(state.clone());

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

// Test list with combined filters
// Requirements: 12.2, 12.3
#[tokio::test]
async fn test_list_repositories_with_combined_filters() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Create two providers
    let provider1_id = create_test_provider(
        app.clone(),
        "Provider 1",
        "https://gitea1.example.com",
        "token1_12345678",
    )
    .await;

    let provider2_id = create_test_provider(
        app.clone(),
        "Provider 2",
        "https://gitea2.example.com",
        "token2_12345678",
    )
    .await;

    // Create repositories with different combinations
    let _repo1 = create_test_repository(
        state.clone(),
        provider1_id,
        "valid-repo1",
        "owner/valid-repo1",
        ValidationStatus::Valid,
    )
    .await;

    let _repo2 = create_test_repository(
        state.clone(),
        provider1_id,
        "invalid-repo1",
        "owner/invalid-repo1",
        ValidationStatus::Invalid,
    )
    .await;

    let _repo3 = create_test_repository(
        state.clone(),
        provider2_id,
        "valid-repo2",
        "owner/valid-repo2",
        ValidationStatus::Valid,
    )
    .await;

    // List repositories for provider1 with valid status
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/api/repositories?provider_id={}&validation_status=valid",
                    provider1_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let repositories: Vec<RepositoryResponse> = serde_json::from_slice(&body).unwrap();

    // Should return only valid repositories for provider1
    assert!(repositories.len() >= 1);
    for repo in repositories {
        assert_eq!(repo.provider_id, provider1_id);
        assert_eq!(repo.validation_status, ValidationStatus::Valid);
    }
}
