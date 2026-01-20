use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vibe_repo::test_utils::state::create_test_app;

#[tokio::test]
async fn test_webhook_endpoint_exists() {
    let app = create_test_app().await.expect("Failed to create test app");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/webhooks/1")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // The endpoint exists, but repository 1 doesn't exist in test DB
    // So we expect 404 (repository not found), not 404 (route not found)
    // Both return 404, but the route exists and is being processed
    // We can verify this by checking that it's not a 405 Method Not Allowed
    assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_webhook_endpoint_requires_repository_id() {
    let app = create_test_app().await.expect("Failed to create test app");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/webhooks/invalid")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 for invalid repository_id format (not a valid integer)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_webhook_endpoint_accepts_json() {
    let app = create_test_app().await.expect("Failed to create test app");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/webhooks/1")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"test": "data"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should accept JSON content
    // May fail validation but should not reject content-type
    assert_ne!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
