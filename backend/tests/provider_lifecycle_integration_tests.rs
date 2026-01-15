//! Comprehensive integration tests for RepoProvider lifecycle
//!
//! Tests the complete lifecycle of providers including:
//! - Create → List → Verify presence
//! - Create → Get by ID → Verify data
//! - Create → Update → Verify changes
//! - Create → Delete → Verify removal
//! - Create → Validate token → Verify result
//!
//! Requirements: All provider requirements (1.1-9.5)

use axum::body::Body;
use axum::http::{Request, StatusCode};
use gitautodev::api::settings::providers::models::{ProviderResponse, ValidationResponse};
use gitautodev::entities::repo_provider::ProviderType;
use gitautodev::test_utils::state::create_test_app;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

/// Helper function to create a test provider and return the response
async fn create_provider(
    app: axum::Router,
    name: &str,
    base_url: &str,
    token: &str,
) -> (StatusCode, ProviderResponse) {
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

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

    (status, provider)
}

/// Test: Create provider → List → Verify presence
/// Requirements: 1.1-1.8, 2.1-2.4
#[tokio::test]
async fn test_lifecycle_create_then_list() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Lifecycle Test Provider",
        "https://gitea.lifecycle.test",
        "lifecycle_token_12345678",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);
    assert_eq!(created_provider.name, "Lifecycle Test Provider");
    assert_eq!(created_provider.base_url, "https://gitea.lifecycle.test");
    assert_eq!(created_provider.access_token, "lifecycl***"); // Masked
    assert!(!created_provider.locked);

    // Step 2: List all providers
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

    // Step 3: Verify the created provider is in the list
    let found = providers.iter().find(|p| p.id == created_provider.id);

    assert!(found.is_some(), "Created provider should be in the list");
    let found_provider = found.unwrap();
    assert_eq!(found_provider.name, "Lifecycle Test Provider");
    assert_eq!(found_provider.access_token, "lifecycl***"); // Token should be masked
}

/// Test: Create provider → Get by ID → Verify data
/// Requirements: 1.1-1.8, 3.1-3.4
#[tokio::test]
async fn test_lifecycle_create_then_get() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Get Test Provider",
        "https://gitea.get.test",
        "get_test_token_12345678",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);

    // Step 2: Get the provider by ID
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let retrieved_provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

    // Step 3: Verify all data matches
    assert_eq!(retrieved_provider.id, created_provider.id);
    assert_eq!(retrieved_provider.name, "Get Test Provider");
    assert_eq!(retrieved_provider.provider_type, ProviderType::Gitea);
    assert_eq!(retrieved_provider.base_url, "https://gitea.get.test");
    assert_eq!(retrieved_provider.access_token, "get_test***"); // Masked
    assert!(!retrieved_provider.locked);
    assert_eq!(retrieved_provider.created_at, created_provider.created_at);
    assert_eq!(retrieved_provider.updated_at, created_provider.updated_at);
}

/// Test: Create provider → Update → Verify changes
/// Requirements: 1.1-1.8, 4.1-4.9
#[tokio::test]
async fn test_lifecycle_create_then_update() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Original Name",
        "https://gitea.original.test",
        "original_token_12345678",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);
    assert_eq!(created_provider.name, "Original Name");
    assert!(!created_provider.locked);

    // Step 2: Update the provider (name and locked status)
    let update_body = json!({
        "name": "Updated Name",
        "locked": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated_provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

    // Step 3: Verify changes
    assert_eq!(updated_provider.id, created_provider.id);
    assert_eq!(updated_provider.name, "Updated Name"); // Changed
    assert!(updated_provider.locked); // Changed
    assert_eq!(updated_provider.base_url, created_provider.base_url); // Unchanged
    assert_eq!(
        updated_provider.provider_type,
        created_provider.provider_type
    ); // Unchanged
       // Note: updated_at may not change if update happens within same second

    // Step 4: Get the provider again to verify persistence
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let retrieved_provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(retrieved_provider.name, "Updated Name");
    assert!(retrieved_provider.locked);
}

/// Test: Create provider → Delete → Verify removal
/// Requirements: 1.1-1.8, 5.1-5.5
#[tokio::test]
async fn test_lifecycle_create_then_delete() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Delete Test Provider",
        "https://gitea.delete.test",
        "delete_token_12345678",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);
    assert!(!created_provider.locked); // Should be unlocked by default

    // Step 2: Verify the provider exists
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Step 3: Delete the provider
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Step 4: Verify the provider is removed (GET should return 404)
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test: Create locked provider → Attempt delete → Verify failure
/// Requirements: 5.1-5.5
#[tokio::test]
async fn test_lifecycle_create_locked_then_delete_fails() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Locked Provider",
        "https://gitea.locked.test",
        "locked_token_12345678",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);

    // Step 2: Lock the provider
    let update_body = json!({
        "locked": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Step 3: Attempt to delete the locked provider
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 409 Conflict
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // Step 4: Verify the provider still exists
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/settings/providers/{}", created_provider.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test: Create provider → Validate token → Verify result (with valid token)
/// Requirements: 6.1-6.5
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_lifecycle_create_then_validate_valid_token() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider with valid test credentials
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Valid Token Provider",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);

    // Step 2: Validate the provider token
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!(
                    "/api/settings/providers/{}/validate",
                    created_provider.id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Step 3: Verify validation result
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let validation: ValidationResponse = serde_json::from_slice(&body).unwrap();

    assert!(validation.valid, "Token should be valid");
    assert!(validation.user_info.is_some(), "Should return user info");
}

/// Test: Create provider → Validate token → Verify result (with invalid token)
/// Requirements: 6.1-6.5
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_lifecycle_create_then_validate_invalid_token() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider with invalid token
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Invalid Token Provider",
        "https://gitea.devo.top:66",
        "invalid_token_12345678",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);

    // Step 2: Validate the provider token
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!(
                    "/api/settings/providers/{}/validate",
                    created_provider.id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Step 3: Verify validation result
    // Should return 401 Unauthorized or 200 with valid=false
    assert!(response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::OK);

    if response.status() == StatusCode::OK {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let validation: ValidationResponse = serde_json::from_slice(&body).unwrap();
        assert!(!validation.valid, "Token should be invalid");
    }
}

/// Test: Complete lifecycle with multiple operations
/// Requirements: All provider requirements
#[tokio::test]
async fn test_complete_provider_lifecycle() {
    let app = create_test_app().await.expect("Failed to create test app");

    // Step 1: Create a provider
    let (create_status, created_provider) = create_provider(
        app.clone(),
        "Complete Lifecycle Provider",
        "https://gitea.complete.test",
        "complete_token_12345678",
    )
    .await;

    assert_eq!(create_status, StatusCode::CREATED);
    let provider_id = created_provider.id;

    // Step 2: List and verify presence
    let response = app
        .clone()
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
    assert!(providers.iter().any(|p| p.id == provider_id));

    // Step 3: Get by ID
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/settings/providers/{}", provider_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Step 4: Update name
    let update_body = json!({
        "name": "Updated Complete Provider"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", provider_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated: ProviderResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.name, "Updated Complete Provider");

    // Step 5: Update token
    let update_body = json!({
        "access_token": "new_token_87654321"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", provider_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated: ProviderResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.access_token, "new_toke***"); // New token masked

    // Step 6: Delete
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/settings/providers/{}", provider_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
