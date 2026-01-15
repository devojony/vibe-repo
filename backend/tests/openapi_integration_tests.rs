//! OpenAPI documentation integration tests
//!
//! Tests for OpenAPI specification and Swagger UI endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use gitautodev::api::create_router;
use gitautodev::test_utils::state::create_test_state;
use tower::ServiceExt; // for `oneshot`

/// Test /api-docs/openapi.json returns valid JSON
/// Requirements: 8.1
#[tokio::test]
async fn test_openapi_json_returns_valid_json() {
    // Arrange: Create test state and router
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = create_router(state);

    // Act: Send request to OpenAPI spec endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "OpenAPI spec endpoint should return 200"
    );

    // Assert: Content-Type should be application/json
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present");
    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "Content-Type should be application/json"
    );

    // Assert: Body should be valid JSON
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let json_value: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Response body should be valid JSON");

    // Assert: JSON should contain OpenAPI required fields
    assert!(
        json_value.get("openapi").is_some(),
        "OpenAPI spec should contain 'openapi' field"
    );
    assert!(
        json_value.get("info").is_some(),
        "OpenAPI spec should contain 'info' field"
    );
    assert!(
        json_value.get("paths").is_some(),
        "OpenAPI spec should contain 'paths' field"
    );
}

/// Test /swagger-ui returns HTML
/// Requirements: 8.2
#[tokio::test]
async fn test_swagger_ui_returns_html() {
    // Arrange: Create test state and router
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = create_router(state);

    // Act: Send request to Swagger UI endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Swagger UI endpoint should return 200"
    );

    // Assert: Content-Type should be text/html
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present");
    assert!(
        content_type.to_str().unwrap().contains("text/html"),
        "Content-Type should be text/html, got: {}",
        content_type.to_str().unwrap()
    );

    // Assert: Body should contain HTML content
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body_str = String::from_utf8(body_bytes.to_vec()).expect("Body should be valid UTF-8");

    // Assert: HTML should contain Swagger UI elements
    assert!(
        body_str.contains("<!DOCTYPE html>") || body_str.contains("<html"),
        "Response should contain HTML doctype or html tag"
    );
}

/// Test OpenAPI spec contains API info
/// Requirements: 8.3
#[tokio::test]
async fn test_openapi_spec_contains_api_info() {
    // Arrange: Create test state and router
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = create_router(state);

    // Act: Send request to OpenAPI spec endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Parse JSON response
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let json_value: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Response body should be valid JSON");

    // Assert: Info section should contain title, version, and description
    let info = json_value
        .get("info")
        .expect("OpenAPI spec should contain 'info' field");

    assert!(
        info.get("title").is_some(),
        "Info should contain 'title' field"
    );
    assert!(
        info.get("version").is_some(),
        "Info should contain 'version' field"
    );
    assert!(
        info.get("description").is_some(),
        "Info should contain 'description' field"
    );

    // Assert: Verify specific values
    assert_eq!(
        info.get("title").unwrap().as_str().unwrap(),
        "GitAutoDev API",
        "API title should be 'GitAutoDev API'"
    );
    assert_eq!(
        info.get("version").unwrap().as_str().unwrap(),
        "0.1.0",
        "API version should be '0.1.0'"
    );
}

/// Test OpenAPI spec documents health endpoint
/// Requirements: 8.4
#[tokio::test]
async fn test_openapi_spec_documents_health_endpoint() {
    // Arrange: Create test state and router
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = create_router(state);

    // Act: Send request to OpenAPI spec endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Parse JSON response
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let json_value: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Response body should be valid JSON");

    // Assert: Paths should contain /health endpoint
    let paths = json_value
        .get("paths")
        .expect("OpenAPI spec should contain 'paths' field");

    assert!(
        paths.get("/health").is_some(),
        "Paths should contain '/health' endpoint"
    );

    // Assert: Health endpoint should have GET method
    let health_path = paths.get("/health").unwrap();
    assert!(
        health_path.get("get").is_some(),
        "Health endpoint should have GET method"
    );

    // Assert: Components should contain HealthResponse schema
    let components = json_value
        .get("components")
        .expect("OpenAPI spec should contain 'components' field");
    let schemas = components
        .get("schemas")
        .expect("Components should contain 'schemas' field");

    assert!(
        schemas.get("HealthResponse").is_some(),
        "Schemas should contain 'HealthResponse'"
    );
}

/// Test OpenAPI spec documents repository endpoints
/// Requirements: Task 12 - OpenAPI Documentation
#[tokio::test]
async fn test_openapi_spec_documents_repository_endpoints() {
    // Arrange: Create test state and router
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = create_router(state);

    // Act: Send request to OpenAPI spec endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Parse JSON response
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let json_value: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Response body should be valid JSON");

    // Assert: Paths should contain repository endpoints
    let paths = json_value
        .get("paths")
        .expect("OpenAPI spec should contain 'paths' field");

    // Check /api/repositories endpoint
    assert!(
        paths.get("/api/repositories").is_some(),
        "Paths should contain '/api/repositories' endpoint"
    );
    let list_endpoint = paths.get("/api/repositories").unwrap();
    assert!(
        list_endpoint.get("get").is_some(),
        "Repository list endpoint should have GET method"
    );

    // Check /api/repositories/{id} endpoint
    assert!(
        paths.get("/api/repositories/{id}").is_some(),
        "Paths should contain '/api/repositories/{{id}}' endpoint"
    );
    let get_endpoint = paths.get("/api/repositories/{id}").unwrap();
    assert!(
        get_endpoint.get("get").is_some(),
        "Repository get endpoint should have GET method"
    );

    // Check /api/repositories/{id}/refresh endpoint
    assert!(
        paths.get("/api/repositories/{id}/refresh").is_some(),
        "Paths should contain '/api/repositories/{{id}}/refresh' endpoint"
    );
    let refresh_endpoint = paths.get("/api/repositories/{id}/refresh").unwrap();
    assert!(
        refresh_endpoint.get("post").is_some(),
        "Repository refresh endpoint should have POST method"
    );

    // Assert: Components should contain RepositoryResponse schema
    let components = json_value
        .get("components")
        .expect("OpenAPI spec should contain 'components' field");
    let schemas = components
        .get("schemas")
        .expect("Components should contain 'schemas' field");

    assert!(
        schemas.get("RepositoryResponse").is_some(),
        "Schemas should contain 'RepositoryResponse'"
    );

    // Assert: Components should contain ValidationStatus schema
    assert!(
        schemas.get("ValidationStatus").is_some(),
        "Schemas should contain 'ValidationStatus'"
    );

    // Verify ValidationStatus enum values
    let validation_status = schemas.get("ValidationStatus").unwrap();
    let enum_values = validation_status
        .get("enum")
        .expect("ValidationStatus should have enum values");
    assert!(
        enum_values
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("Valid")),
        "ValidationStatus should contain 'Valid' value"
    );
    assert!(
        enum_values
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("Invalid")),
        "ValidationStatus should contain 'Invalid' value"
    );
    assert!(
        enum_values
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("Pending")),
        "ValidationStatus should contain 'Pending' value"
    );
}

/// Test OpenAPI spec documents 409 Conflict responses for provider endpoints
/// Requirements: Unique constraint error handling
#[tokio::test]
async fn test_openapi_spec_documents_provider_conflict_responses() {
    // Arrange: Create test state and router
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = create_router(state);

    // Act: Send request to OpenAPI spec endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Parse JSON response
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let json_value: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("Response body should be valid JSON");

    // Assert: Paths should contain provider endpoints
    let paths = json_value
        .get("paths")
        .expect("OpenAPI spec should contain 'paths' field");

    // Check POST /api/settings/providers has 409 response
    let create_endpoint = paths
        .get("/api/settings/providers")
        .expect("Paths should contain '/api/settings/providers' endpoint");
    let post_method = create_endpoint
        .get("post")
        .expect("Provider endpoint should have POST method");
    let post_responses = post_method
        .get("responses")
        .expect("POST method should have responses");

    assert!(
        post_responses.get("409").is_some(),
        "POST /api/settings/providers should document 409 Conflict response"
    );

    let conflict_response = post_responses.get("409").unwrap();
    let description = conflict_response
        .get("description")
        .expect("409 response should have description")
        .as_str()
        .unwrap();

    assert!(
        description.contains("name") && description.contains("base_url") && description.contains("access_token"),
        "409 description should mention the unique constraint fields (name, base_url, access_token), got: {}",
        description
    );

    // Check PUT /api/settings/providers/{id} has 409 response
    let update_endpoint = paths
        .get("/api/settings/providers/{id}")
        .expect("Paths should contain '/api/settings/providers/{{id}}' endpoint");
    let put_method = update_endpoint
        .get("put")
        .expect("Provider endpoint should have PUT method");
    let put_responses = put_method
        .get("responses")
        .expect("PUT method should have responses");

    assert!(
        put_responses.get("409").is_some(),
        "PUT /api/settings/providers/{{id}} should document 409 Conflict response"
    );

    let update_conflict_response = put_responses.get("409").unwrap();
    let update_description = update_conflict_response
        .get("description")
        .expect("409 response should have description")
        .as_str()
        .unwrap();

    assert!(
        update_description.contains("duplicate") || update_description.contains("name"),
        "409 description should mention duplicate or constraint fields, got: {}",
        update_description
    );
}
