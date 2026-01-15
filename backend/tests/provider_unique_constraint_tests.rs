//! Integration tests for provider unique constraint
//!
//! Tests that the (name, base_url, access_token) unique constraint works correctly.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use gitautodev::test_utils::state::create_test_app;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_duplicate_provider_returns_409() {
    let app = create_test_app().await.unwrap();

    // Create first provider
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Test Provider",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "test_token_12345678"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Try to create duplicate provider (same name, base_url, access_token)
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Test Provider",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "test_token_12345678"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // Verify error message
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(body["error"].as_str().unwrap().contains("already exists"));
}

#[tokio::test]
async fn test_different_name_allows_duplicate() {
    let app = create_test_app().await.unwrap();

    // Create first provider
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Provider 1",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "test_token_12345678"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Create provider with different name (should succeed)
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Provider 2",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "test_token_12345678"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_different_base_url_allows_duplicate() {
    let app = create_test_app().await.unwrap();

    // Create first provider
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Test Provider",
                "provider_type": "gitea",
                "base_url": "https://gitea1.example.com",
                "access_token": "test_token_12345678"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Create provider with different base_url (should succeed)
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Test Provider",
                "provider_type": "gitea",
                "base_url": "https://gitea2.example.com",
                "access_token": "test_token_12345678"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_different_token_allows_duplicate() {
    let app = create_test_app().await.unwrap();

    // Create first provider
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Test Provider",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "token_aaaaaaaa"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Create provider with different token (should succeed)
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Test Provider",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "token_bbbbbbbb"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_update_to_duplicate_returns_409() {
    let app = create_test_app().await.unwrap();

    // Create first provider
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Provider 1",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "token_aaaaaaaa"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let provider1: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let provider1_id = provider1["id"].as_i64().unwrap();

    // Create second provider
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Provider 2",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "token_bbbbbbbb"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Try to update second provider to match first provider (should fail)
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/settings/providers/{}", provider1_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Provider 2",
                "access_token": "token_bbbbbbbb"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_update_same_provider_succeeds() {
    let app = create_test_app().await.unwrap();

    // Create provider
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/providers")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Test Provider",
                "provider_type": "gitea",
                "base_url": "https://gitea.example.com",
                "access_token": "test_token_12345678"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let provider: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let provider_id = provider["id"].as_i64().unwrap();

    // Update locked field only (should succeed even though name/url/token unchanged)
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/settings/providers/{}", provider_id))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "locked": true
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
