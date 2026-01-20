//! Integration tests for health check endpoint
//!
//! Tests the full HTTP request/response cycle for the health check API.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use vibe_repo::api::health::handlers::HealthResponse;
use vibe_repo::test_utils::state::create_test_app; // for `oneshot`

/// Test GET /health returns 200 when healthy
/// Requirements: 7.1, 7.2
#[tokio::test]
async fn test_health_endpoint_returns_200_when_healthy() {
    // Arrange: Create test application with valid database
    let app = create_test_app().await.expect("Failed to create test app");

    // Act: Send GET request to /health
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Health endpoint should return 200 when database is connected"
    );
}

/// Test response body matches HealthResponse schema
/// Requirements: 7.1, 7.2
#[tokio::test]
async fn test_health_response_body_matches_schema() {
    // Arrange: Create test application
    let app = create_test_app().await.expect("Failed to create test app");

    // Act: Send GET request to /health
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Response body should match HealthResponse schema
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Parse JSON response
    let health_response: HealthResponse =
        serde_json::from_str(&body_str).expect("Response body should be valid HealthResponse JSON");

    // Verify schema fields
    assert!(
        !health_response.status.is_empty(),
        "status field should not be empty"
    );
    assert!(
        !health_response.database.is_empty(),
        "database field should not be empty"
    );

    // Verify expected values based on status code
    if status == StatusCode::OK {
        assert_eq!(health_response.status, "healthy");
        assert_eq!(health_response.database, "connected");
    } else if status == StatusCode::SERVICE_UNAVAILABLE {
        assert_eq!(health_response.status, "unhealthy");
        assert_eq!(health_response.database, "disconnected");
    }
}

/// Test health endpoint returns correct content-type header
/// Requirements: 7.1, 7.2
#[tokio::test]
async fn test_health_endpoint_returns_json_content_type() {
    // Arrange: Create test application
    let app = create_test_app().await.expect("Failed to create test app");

    // Act: Send GET request to /health
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Content-Type should be application/json
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Response should have content-type header");

    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "Content-Type should be application/json"
    );
}

/// Test health endpoint is accessible via the router
/// Requirements: 7.1
#[tokio::test]
async fn test_health_endpoint_accessible_via_router() {
    // Arrange: Create test application
    let app = create_test_app().await.expect("Failed to create test app");

    // Act: Send GET request to /health
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should not return 404 (endpoint exists)
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Health endpoint should be accessible and not return 404"
    );
}

/// Test health endpoint handles multiple requests
/// Requirements: 7.1, 7.2
#[tokio::test]
async fn test_health_endpoint_handles_multiple_requests() {
    // Arrange: Create test application
    let app = create_test_app().await.expect("Failed to create test app");

    // Act: Send multiple requests
    for i in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert: Each request should succeed
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE,
            "Request {} should return valid status code",
            i
        );
    }
}
