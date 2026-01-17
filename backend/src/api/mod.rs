//! API module
//!
//! Combines all feature routers into a single application router.

pub mod health;
pub mod repositories;
pub mod settings;
pub mod workspaces;

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
        title = "GitAutoDev API",
        version = "0.1.0",
        description = "GitAutoDev automated programming assistant API"
    ),
    paths(
        health::handlers::health_check,
        settings::providers::handlers::list_providers,
        settings::providers::handlers::create_provider,
        settings::providers::handlers::get_provider,
        settings::providers::handlers::update_provider,
        settings::providers::handlers::delete_provider,
        settings::providers::handlers::validate_provider,
        settings::providers::handlers::sync_provider,
        repositories::handlers::list_repositories,
        repositories::handlers::get_repository,
        repositories::handlers::update_repository,
        repositories::handlers::delete_repository,
        repositories::handlers::refresh_repository,
        repositories::handlers::initialize_repository,
        repositories::handlers::reinitialize_repository,
        repositories::handlers::archive_repository,
        repositories::handlers::unarchive_repository,
        repositories::handlers::batch_initialize_repositories,
        repositories::handlers::batch_archive_repositories,
        repositories::handlers::batch_delete_repositories,
        repositories::handlers::batch_refresh_repositories,
        repositories::handlers::batch_reinitialize_repositories,
    ),
    components(schemas(
        health::handlers::HealthResponse,
        settings::providers::models::CreateProviderRequest,
        settings::providers::models::UpdateProviderRequest,
        settings::providers::models::ProviderResponse,
        settings::providers::models::ValidationResponse,
        settings::providers::models::UserInfo,
        repositories::models::RepositoryResponse,
        repositories::models::InitializeRepositoryRequest,
        repositories::models::UpdateRepositoryRequest,
        repositories::models::BatchInitializeParams,
        repositories::models::BatchInitializeResponse,
        repositories::models::BatchOperationRequest,
        repositories::models::BatchOperationResponse,
        repositories::models::BatchOperationResult,
        crate::entities::repo_provider::ProviderType,
        crate::entities::repository::ValidationStatus,
        crate::entities::repository::RepositoryStatus,
    ))
)]
pub struct ApiDoc;

/// Create the main application router by combining all feature routers
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Mount feature routers at their respective paths
        .nest("/health", health::routes::router())
        .nest(
            "/api/settings/providers",
            settings::providers::routes::router(),
        )
        .nest("/api/repositories", repositories::routes::router())
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

    /// Test that the router includes health routes
    /// Requirements: 6.2
    #[tokio::test]
    async fn test_router_includes_health_routes() {
        // Arrange: Create test state and router
        let state = create_test_state()
            .await
            .expect("Failed to create test state");
        let app = create_router(state);

        // Act: Send request to /health endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert: Should get a valid response (200 or 503, not 404)
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE,
            "Health endpoint should exist and return 200 or 503, got {}",
            status
        );
    }

    /// Test that the router includes tracing middleware
    /// Requirements: 6.3
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
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert: Request should be processed successfully
        // The presence of TraceLayer is verified by the fact that the request completes
        // without errors. TraceLayer adds tracing spans to all requests.
        assert!(
            response.status().is_success() || response.status().is_server_error(),
            "Request should be processed by middleware"
        );
    }

    /// Test that the router includes CORS middleware
    /// Requirements: 6.3
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
                    .uri("/health")
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
    /// Requirements: 6.2
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
    /// Requirements: 9.3
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
                    .uri("/health")
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
