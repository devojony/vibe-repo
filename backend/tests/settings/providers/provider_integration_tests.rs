//! Integration tests for RepoProvider API
//!
//! Tests the full HTTP request/response cycle for provider CRUD operations.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use vibe_repo::api::settings::providers::models::{ProviderResponse, ValidationResponse};
use vibe_repo::entities::repo_provider::ProviderType;
use vibe_repo::test_utils::state::create_test_app;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

/// Helper function to create a test provider
async fn create_test_provider(
    app: axum::Router,
    name: &str,
    base_url: &str,
    token: &str,
) -> ProviderResponse {
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
    serde_json::from_slice(&body).unwrap()
}

// Test create with valid data
// Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8
#[tokio::test]
async fn test_create_provider_with_valid_data() {
    let app = create_test_app().await.expect("Failed to create test app");

    let request_body = json!({
        "name": "Test Gitea",
        "provider_type": "gitea",
        "base_url": "https://gitea.example.com",
        "access_token": "test_token_12345678"
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

    // Should return 201 Created
    assert_eq!(response.status(), StatusCode::CREATED);

    // Parse response body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

    // Verify response fields
    assert_eq!(provider.name, "Test Gitea");
    assert_eq!(provider.provider_type, ProviderType::Gitea);
    assert_eq!(provider.base_url, "https://gitea.example.com");
    assert_eq!(provider.access_token, "test_tok***"); // Token should be masked
    assert!(!provider.locked); // Should default to false
    assert!(!provider.created_at.is_empty());
    assert!(!provider.updated_at.is_empty());
}

// Test create with missing name field
// Requirements: 1.2
#[tokio::test]
async fn test_create_provider_with_missing_name() {
    let app = create_test_app().await.expect("Failed to create test app");

    let request_body = json!({
        "provider_type": "gitea",
        "base_url": "https://gitea.example.com",
        "access_token": "test_token"
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

    // Should return 400 Bad Request or 422 Unprocessable Entity
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

// Test create with empty name
// Requirements: 1.2
#[tokio::test]
async fn test_create_provider_with_empty_name() {
    let app = create_test_app().await.expect("Failed to create test app");

    let request_body = json!({
        "name": "",
        "provider_type": "gitea",
        "base_url": "https://gitea.example.com",
        "access_token": "test_token"
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

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Test create with invalid provider type
// Requirements: 1.3
#[tokio::test]
async fn test_create_provider_with_invalid_type() {
    let app = create_test_app().await.expect("Failed to create test app");

    let request_body = json!({
        "name": "Test Provider",
        "provider_type": "github",
        "base_url": "https://github.com",
        "access_token": "test_token"
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

    // Should return 400 Bad Request or 422 Unprocessable Entity
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

// Test create with missing access_token
// Requirements: 1.4
#[tokio::test]
async fn test_create_provider_with_missing_token() {
    let app = create_test_app().await.expect("Failed to create test app");

    let request_body = json!({
        "name": "Test Gitea",
        "provider_type": "gitea",
        "base_url": "https://gitea.example.com"
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

    // Should return 400 Bad Request or 422 Unprocessable Entity
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

// Test create with empty access_token
// Requirements: 1.4
#[tokio::test]
async fn test_create_provider_with_empty_token() {
    let app = create_test_app().await.expect("Failed to create test app");

    let request_body = json!({
        "name": "Test Gitea",
        "provider_type": "gitea",
        "base_url": "https://gitea.example.com",
        "access_token": ""
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

    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Test create with missing base_url for gitea
// Requirements: 1.5
#[tokio::test]
async fn test_create_provider_with_missing_base_url() {
    let app = create_test_app().await.expect("Failed to create test app");

    let request_body = json!({
        "name": "Test Gitea",
        "provider_type": "gitea",
        "access_token": "test_token"
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

    // Should return 400 Bad Request or 422 Unprocessable Entity
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

// Test list returns all providers
// Requirements: 2.1, 2.2
#[tokio::test]
async fn test_list_returns_all_providers() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create multiple providers
    let _provider1 = create_test_provider(
        app.clone(),
        "Provider 1",
        "https://gitea1.example.com",
        "token1_12345678",
    )
    .await;

    let _provider2 = create_test_provider(
        app.clone(),
        "Provider 2",
        "https://gitea2.example.com",
        "token2_12345678",
    )
    .await;

    // List providers
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let providers: Vec<ProviderResponse> = serde_json::from_slice(&body).unwrap();

    // Should return at least 2 providers
    assert!(providers.len() >= 2);

    // Verify tokens are masked
    for provider in providers {
        assert!(provider.access_token.ends_with("***"));
    }
}

// Test list returns empty array when no providers
// Requirements: 2.3
#[tokio::test]
async fn test_list_returns_empty_array() {
    let app = create_test_app().await.expect("Failed to create test app");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let providers: Vec<ProviderResponse> = serde_json::from_slice(&body).unwrap();

    assert_eq!(providers.len(), 0);
}

// Test get provider by ID
// Requirements: 3.1, 3.2, 3.4
#[tokio::test]
async fn test_get_provider_by_id() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create a provider
    let created = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    // Get the provider
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/settings/providers/{}", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(provider.id, created.id);
    assert_eq!(provider.name, "Test Provider");
    assert_eq!(provider.access_token, "test_tok***"); // Token should be masked
}

// Test get non-existent provider
// Requirements: 3.3
#[tokio::test]
async fn test_get_nonexistent_provider() {
    let app = create_test_app().await.expect("Failed to create test app");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings/providers/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test update provider with partial data
// Requirements: 4.1, 4.2, 4.5
#[tokio::test]
async fn test_update_provider_partial_data() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create a provider
    let created = create_test_provider(
        app.clone(),
        "Original Name",
        "https://gitea.example.com",
        "original_token_12345678",
    )
    .await;

    // Update only the name
    let update_body = json!({
        "name": "Updated Name"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", created.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated: ProviderResponse = serde_json::from_slice(&body).unwrap();

    // Name should be updated
    assert_eq!(updated.name, "Updated Name");
    // Other fields should remain unchanged
    assert_eq!(updated.base_url, created.base_url);
    assert_eq!(updated.provider_type, created.provider_type);
}

// Test update locked field
// Requirements: 4.9
#[tokio::test]
async fn test_update_provider_locked_field() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create a provider
    let created = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    assert!(!created.locked);

    // Update locked to true
    let update_body = json!({
        "locked": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", created.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated: ProviderResponse = serde_json::from_slice(&body).unwrap();

    assert!(updated.locked);
}

// Test update non-existent provider
// Requirements: 4.4
#[tokio::test]
async fn test_update_nonexistent_provider() {
    let app = create_test_app().await.expect("Failed to create test app");

    let update_body = json!({
        "name": "Updated Name"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings/providers/99999")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test delete unlocked provider
// Requirements: 5.1, 5.3, 5.5
#[tokio::test]
async fn test_delete_unlocked_provider() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create a provider
    let created = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    // Delete the provider
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/settings/providers/{}", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

// Test delete locked provider
// Requirements: 5.2
#[tokio::test]
async fn test_delete_locked_provider() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create a provider
    let created = create_test_provider(
        app.clone(),
        "Test Provider",
        "https://gitea.example.com",
        "test_token_12345678",
    )
    .await;

    // Lock the provider
    let update_body = json!({
        "locked": true
    });

    let _lock_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", created.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try to delete the locked provider
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/settings/providers/{}", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

// Test delete non-existent provider
// Requirements: 5.4
#[tokio::test]
async fn test_delete_nonexistent_provider() {
    let app = create_test_app().await.expect("Failed to create test app");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/settings/providers/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test validate with valid token (using test Gitea instance)
// Requirements: 6.1, 6.2, 6.5
#[tokio::test]
#[ignore] // Ignore by default as it requires external Gitea instance
async fn test_validate_provider_with_valid_token() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create a provider with valid test credentials
    let created = create_test_provider(
        app.clone(),
        "Test Gitea",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    // Validate the provider
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/settings/providers/{}/validate", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let validation: ValidationResponse = serde_json::from_slice(&body).unwrap();

    assert!(validation.valid);
    assert!(validation.user_info.is_some());
}

// Test validate with invalid token
// Requirements: 6.3
#[tokio::test]
#[ignore] // Ignore by default as it requires external Gitea instance
async fn test_validate_provider_with_invalid_token() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Create a provider with invalid token
    let created = create_test_provider(
        app.clone(),
        "Test Gitea",
        "https://gitea.devo.top:66",
        "invalid_token_12345678",
    )
    .await;

    // Validate the provider
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/settings/providers/{}/validate", created.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 401 Unauthorized or 200 with valid=false
    assert!(response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::OK);

    if response.status() == StatusCode::OK {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let validation: ValidationResponse = serde_json::from_slice(&body).unwrap();
        assert!(!validation.valid);
    }
}
