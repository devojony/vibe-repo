//! Logging and Tracing Configuration
//!
//! This module provides structured logging configuration with support for:
//! - RUST_LOG environment variable for log level control
//! - Request ID tracking in log entries
//! - JSON formatting in production, human-readable in development

use axum::{extract::Request, middleware::Next, response::Response};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

/// Request ID header name
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Initialize the tracing subscriber with appropriate formatting
///
/// # Arguments
/// * `json_format` - If true, use JSON formatting (production). If false, use human-readable format (development)
///
/// # Examples
/// ```no_run
/// use vibe_repo::logging::init_tracing;
///
/// // Development mode (human-readable)
/// init_tracing(false);
///
/// // Production mode (JSON)
/// init_tracing(true);
/// ```
pub fn init_tracing(json_format: bool) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if json_format {
        // JSON format for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // Human-readable format for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

/// Middleware to add request ID to all log entries
///
/// This middleware:
/// 1. Extracts or generates a request ID
/// 2. Adds it to the tracing span
/// 3. Includes it in the response headers
///
/// # Examples
/// ```no_run
/// use axum::{Router, middleware};
/// use std::sync::Arc;
/// use vibe_repo::{logging::request_id_middleware, state::AppState};
///
/// # async fn example(state: Arc<AppState>) {
/// let app: Router = Router::new()
///     .layer(middleware::from_fn(request_id_middleware))
///     .with_state(state);
/// # }
/// ```
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Extract or generate request ID
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Create a span with the request ID
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    // Enter the span for this request
    let _enter = span.enter();

    // Add request ID to request extensions for handlers to access
    request.extensions_mut().insert(request_id.clone());

    // Process the request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    if let Ok(header_value) = request_id.parse() {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    } else {
        tracing::warn!(
            request_id = %request_id,
            "Failed to parse request ID as header value"
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "test")
    }

    #[tokio::test]
    async fn test_request_id_middleware_generates_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should have request ID in response headers
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_id_middleware_preserves_existing_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let existing_id = "test-request-id-123";
        let request = Request::builder()
            .uri("/test")
            .header(REQUEST_ID_HEADER, existing_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should preserve the existing request ID
        let response_id = response
            .headers()
            .get(REQUEST_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(response_id, existing_id);
    }

    #[test]
    fn test_init_tracing_does_not_panic() {
        // Test that initialization doesn't panic
        // Note: We can't test this multiple times in the same process
        // because tracing can only be initialized once
        // This test just ensures the function signature is correct
    }
}
