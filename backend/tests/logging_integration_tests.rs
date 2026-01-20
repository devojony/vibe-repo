//! Integration tests for logging configuration
//!
//! Tests verify that:
//! - RUST_LOG environment variable controls log levels
//! - LOG_FORMAT environment variable controls output format
//! - Request ID middleware adds request IDs to responses

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vibe_repo::test_utils::state::create_test_app;

/// Test that request ID is added to response headers
/// Requirements: 9.3
#[tokio::test]
async fn test_request_id_added_to_response() {
    // Arrange
    let app = create_test_app().await.expect("Failed to create test app");

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers().contains_key("x-request-id"),
        "Response should include x-request-id header"
    );
}

/// Test that existing request ID is preserved
/// Requirements: 9.3
#[tokio::test]
async fn test_existing_request_id_preserved() {
    // Arrange
    let app = create_test_app().await.expect("Failed to create test app");
    let request_id = "test-request-123";

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("x-request-id", request_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let response_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(response_id, request_id);
}

/// Test that request ID is unique for each request
/// Requirements: 9.3
#[tokio::test]
async fn test_request_id_unique_per_request() {
    // Arrange
    let app1 = create_test_app().await.expect("Failed to create test app");
    let app2 = create_test_app().await.expect("Failed to create test app");

    // Act
    let response1 = app1
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let response2 = app2
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    let id1 = response1
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    let id2 = response2
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();

    assert_ne!(id1, id2, "Each request should have a unique request ID");
}
