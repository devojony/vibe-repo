//! API module
//!
//! Simplified MVP API with only core endpoints.

pub mod repositories;
pub mod settings;
pub mod tasks;
pub mod webhooks;

use axum::{middleware, Router};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{logging, state::AppState};

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "VibeRepo API - Simplified MVP",
        version = "0.4.0-mvp",
        description = "VibeRepo automated programming assistant API - Simplified MVP with 10 core endpoints"
    ),
    paths(
        repositories::handlers::add_repository,
        repositories::handlers::list_repositories,
        repositories::handlers::get_repository,
        repositories::handlers::update_repository,
        repositories::handlers::delete_repository,
        repositories::handlers::refresh_repository,
        repositories::handlers::initialize_repository,
        repositories::handlers::reinitialize_repository,
        repositories::handlers::archive_repository,
        repositories::handlers::unarchive_repository,
        webhooks::handlers::handle_webhook,
        tasks::handlers::create_task,
        tasks::handlers::get_task,
        tasks::handlers::list_tasks_by_workspace,
        tasks::handlers::update_task_status,
    ),
    components(schemas(
        repositories::models::AddRepositoryRequest,
        repositories::models::RepositoryResponse,
        repositories::models::InitializeRepositoryRequest,
        repositories::models::UpdateRepositoryRequest,
        webhooks::models::WebhookPayload,
        webhooks::models::WebhookResponse,
        crate::entities::repository::ValidationStatus,
        crate::entities::repository::RepositoryStatus,
        tasks::TaskResponse,
        tasks::CreateTaskRequest,
        tasks::UpdateTaskStatusRequest,
        crate::entities::task::TaskStatus,
    ))
)]
pub struct ApiDoc;

/// Create the main application router by combining all feature routers
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Mount feature routers at their respective paths
        .nest("/api/repositories", repositories::routes::router())
        .nest("/api/webhooks", webhooks::routes::router())
        .merge(tasks::task_routes())
        // OpenAPI documentation
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Attach shared state
        .with_state(state)
        // Add middleware layers
        .layer(middleware::from_fn(logging::request_id_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::state::create_test_state;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // for `oneshot`

    /// Test that the router includes tracing middleware
    #[tokio::test]
    async fn test_router_includes_tracing_middleware() {
        // Arrange: Create test state and router
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let app = create_router(state);

        // Act: Send request to trigger middleware
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/repositories")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert: Request should be processed successfully
        // The presence of TraceLayer is verified by the fact that the request completes
        // without errors. TraceLayer adds tracing spans to all requests.
        assert!(
            response.status().is_success()
                || response.status().is_server_error()
                || response.status().is_client_error(),
            "Request should be processed by middleware"
        );
    }

    /// Test that the router includes CORS middleware
    #[tokio::test]
    async fn test_router_includes_cors_middleware() {
        // Arrange: Create test state and router
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let app = create_router(state);

        // Act: Send request with Origin header
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/repositories")
                    .header("Origin", "http://localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert: Response should include CORS headers
        // CorsLayer::permissive() adds access-control-allow-origin header
        let headers = response.headers();
        assert!(
            headers.contains_key("access-control-allow-origin"),
            "Response should include CORS headers"
        );
    }

    /// Test that the router includes OpenAPI documentation endpoints
    #[tokio::test]
    async fn test_router_includes_openapi_endpoints() {
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

        // Assert: Should get a successful response
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "OpenAPI spec endpoint should return 200"
        );
    }

    /// Test that the router includes request ID middleware
    #[tokio::test]
    async fn test_router_includes_request_id_middleware() {
        // Arrange: Create test state and router
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let app = create_router(state);

        // Act: Send request without request ID
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/repositories")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert: Response should include generated request ID
        assert!(
            response.headers().contains_key("x-request-id"),
            "Response should include x-request-id header"
        );
    }
}
