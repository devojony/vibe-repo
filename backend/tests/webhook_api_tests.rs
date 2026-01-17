use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use gitautodev::test_utils::state::create_test_app;
use tower::ServiceExt;

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
    
    // Should not return 404 (endpoint exists)
    // May return 400 or 401 (validation errors) but not 404
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_webhook_endpoint_requires_provider_id() {
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
    
    // Should return 400 for invalid provider_id format (not a valid integer)
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
