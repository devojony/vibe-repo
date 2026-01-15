//! Error handling module
//!
//! Provides unified error types and HTTP response conversion.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Unified error type for the application
#[derive(Debug, thiserror::Error)]
pub enum GitAutoDevError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

/// Error response format for API
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// Optional error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for GitAutoDevError {
    fn into_response(self) -> Response {
        let (status, code, error) = match &self {
            GitAutoDevError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                e.to_string(),
            ),
            GitAutoDevError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            GitAutoDevError::Validation(msg) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone())
            }
            GitAutoDevError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT_ERROR", msg.clone()),
            GitAutoDevError::Config(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_ERROR",
                msg.clone(),
            ),
            GitAutoDevError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
            GitAutoDevError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            GitAutoDevError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                msg.clone(),
            ),
        };

        let body = ErrorResponse {
            error,
            code: Some(code.to_string()),
            details: None,
        };

        (status, Json(body)).into_response()
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, GitAutoDevError>;

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // Task 4.1: Tests for error types
    // Requirements: 4.1, 4.3
    // ============================================

    #[test]
    fn test_database_error_variant_exists() {
        let db_err = sea_orm::DbErr::Custom("test error".to_string());
        let error = GitAutoDevError::Database(db_err);
        assert!(matches!(error, GitAutoDevError::Database(_)));
    }

    #[test]
    fn test_not_found_error_variant_exists() {
        let error = GitAutoDevError::NotFound("resource".to_string());
        assert!(matches!(error, GitAutoDevError::NotFound(_)));
    }

    #[test]
    fn test_validation_error_variant_exists() {
        let error = GitAutoDevError::Validation("invalid input".to_string());
        assert!(matches!(error, GitAutoDevError::Validation(_)));
    }

    #[test]
    fn test_config_error_variant_exists() {
        let error = GitAutoDevError::Config("missing config".to_string());
        assert!(matches!(error, GitAutoDevError::Config(_)));
    }

    #[test]
    fn test_internal_error_variant_exists() {
        let error = GitAutoDevError::Internal("unexpected error".to_string());
        assert!(matches!(error, GitAutoDevError::Internal(_)));
    }

    #[test]
    fn test_database_error_message_is_descriptive() {
        let db_err = sea_orm::DbErr::Custom("connection failed".to_string());
        let error = GitAutoDevError::Database(db_err);
        let message = error.to_string();
        assert!(message.contains("Database error"));
        assert!(message.contains("connection failed"));
    }

    #[test]
    fn test_not_found_error_message_is_descriptive() {
        let error = GitAutoDevError::NotFound("User with id 123".to_string());
        let message = error.to_string();
        assert!(message.contains("Resource not found"));
        assert!(message.contains("User with id 123"));
    }

    #[test]
    fn test_validation_error_message_is_descriptive() {
        let error = GitAutoDevError::Validation("email format invalid".to_string());
        let message = error.to_string();
        assert!(message.contains("Validation error"));
        assert!(message.contains("email format invalid"));
    }

    #[test]
    fn test_config_error_message_is_descriptive() {
        let error = GitAutoDevError::Config("DATABASE_URL not set".to_string());
        let message = error.to_string();
        assert!(message.contains("Configuration error"));
        assert!(message.contains("DATABASE_URL not set"));
    }

    #[test]
    fn test_internal_error_message_is_descriptive() {
        let error = GitAutoDevError::Internal("unexpected state".to_string());
        let message = error.to_string();
        assert!(message.contains("Internal error"));
        assert!(message.contains("unexpected state"));
    }

    // ============================================
    // Task 1.2: Tests for Forbidden and ServiceUnavailable error types
    // Requirements: 4.2, 4.3
    // ============================================

    #[test]
    fn test_forbidden_error_variant_exists() {
        let error = GitAutoDevError::Forbidden("access denied".to_string());
        assert!(matches!(error, GitAutoDevError::Forbidden(_)));
    }

    #[test]
    fn test_forbidden_error_message_is_descriptive() {
        let error =
            GitAutoDevError::Forbidden("Insufficient permissions to create branch".to_string());
        let message = error.to_string();
        assert!(message.contains("Forbidden"));
        assert!(message.contains("Insufficient permissions to create branch"));
    }

    #[test]
    fn test_service_unavailable_error_variant_exists() {
        let error = GitAutoDevError::ServiceUnavailable("service down".to_string());
        assert!(matches!(error, GitAutoDevError::ServiceUnavailable(_)));
    }

    #[test]
    fn test_service_unavailable_error_message_is_descriptive() {
        let error = GitAutoDevError::ServiceUnavailable("Git provider unreachable".to_string());
        let message = error.to_string();
        assert!(message.contains("Service unavailable"));
        assert!(message.contains("Git provider unreachable"));
    }

    #[test]
    fn test_database_error_from_sea_orm_db_err() {
        let db_err = sea_orm::DbErr::Custom("test".to_string());
        let error: GitAutoDevError = db_err.into();
        assert!(matches!(error, GitAutoDevError::Database(_)));
    }

    // ============================================
    // Task 4.3: Tests for error to HTTP response conversion
    // Requirements: 4.2, 4.4
    // ============================================

    fn extract_status_and_body(response: Response) -> (StatusCode, ErrorResponse) {
        use http_body_util::BodyExt;

        let (parts, body) = response.into_parts();
        let status = parts.status;

        // Use tokio runtime to collect body bytes
        let rt = tokio::runtime::Runtime::new().unwrap();
        let bytes = rt.block_on(async { body.collect().await.unwrap().to_bytes() });

        let body: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
        (status, body)
    }

    #[test]
    fn test_not_found_error_returns_404() {
        let error = GitAutoDevError::NotFound("User not found".to_string());
        let response = error.into_response();
        let (status, body) = extract_status_and_body(response);

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, Some("NOT_FOUND".to_string()));
        assert!(body.error.contains("User not found"));
    }

    #[test]
    fn test_validation_error_returns_400() {
        let error = GitAutoDevError::Validation("Invalid email".to_string());
        let response = error.into_response();
        let (status, body) = extract_status_and_body(response);

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, Some("VALIDATION_ERROR".to_string()));
        assert!(body.error.contains("Invalid email"));
    }

    #[test]
    fn test_database_error_returns_500() {
        let db_err = sea_orm::DbErr::Custom("connection failed".to_string());
        let error = GitAutoDevError::Database(db_err);
        let response = error.into_response();
        let (status, body) = extract_status_and_body(response);

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, Some("DATABASE_ERROR".to_string()));
    }

    #[test]
    fn test_config_error_returns_500() {
        let error = GitAutoDevError::Config("Missing DATABASE_URL".to_string());
        let response = error.into_response();
        let (status, body) = extract_status_and_body(response);

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, Some("CONFIG_ERROR".to_string()));
        assert!(body.error.contains("Missing DATABASE_URL"));
    }

    #[test]
    fn test_internal_error_returns_500() {
        let error = GitAutoDevError::Internal("Unexpected state".to_string());
        let response = error.into_response();
        let (status, body) = extract_status_and_body(response);

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, Some("INTERNAL_ERROR".to_string()));
        assert!(body.error.contains("Unexpected state"));
    }

    // ============================================
    // Task 1.2: Tests for Forbidden and ServiceUnavailable HTTP responses
    // Requirements: 4.2, 4.3
    // ============================================

    #[test]
    fn test_forbidden_error_returns_403() {
        let error =
            GitAutoDevError::Forbidden("Insufficient permissions to create branch".to_string());
        let response = error.into_response();
        let (status, body) = extract_status_and_body(response);

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.code, Some("FORBIDDEN".to_string()));
        assert!(body
            .error
            .contains("Insufficient permissions to create branch"));
    }

    #[test]
    fn test_service_unavailable_error_returns_503() {
        let error = GitAutoDevError::ServiceUnavailable("Git provider unreachable".to_string());
        let response = error.into_response();
        let (status, body) = extract_status_and_body(response);

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.code, Some("SERVICE_UNAVAILABLE".to_string()));
        assert!(body.error.contains("Git provider unreachable"));
    }

    #[test]
    fn test_error_response_has_required_error_field() {
        let error = GitAutoDevError::NotFound("test".to_string());
        let response = error.into_response();
        let (_, body) = extract_status_and_body(response);

        // error field must be non-empty
        assert!(!body.error.is_empty());
    }

    #[test]
    fn test_error_response_has_code_field() {
        let error = GitAutoDevError::Validation("test".to_string());
        let response = error.into_response();
        let (_, body) = extract_status_and_body(response);

        // code field should be present
        assert!(body.code.is_some());
    }

    #[test]
    fn test_error_response_schema_matches() {
        // Test that response body can be deserialized to ErrorResponse
        let error = GitAutoDevError::Internal("test error".to_string());
        let response = error.into_response();
        let (_, body) = extract_status_and_body(response);

        // Verify ErrorResponse schema: error (required), code (optional), details (optional)
        assert!(!body.error.is_empty());
        // code and details are optional but code should be set by our implementation
        assert!(body.code.is_some());
        // details is None by default
        assert!(body.details.is_none());
    }

    // ============================================
    // Task 4.5: Property test for error conversion consistency
    // Property 3: Error conversion consistency
    // Validates: Requirements 4.2, 4.4
    // ============================================

    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        /// Strategy to generate arbitrary error messages
        fn error_message_strategy() -> impl Strategy<Value = String> {
            // Generate non-empty strings with printable characters
            "[a-zA-Z0-9 _-]{1,100}".prop_map(|s| s.to_string())
        }

        /// Strategy to generate NotFound errors
        fn not_found_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            error_message_strategy().prop_map(GitAutoDevError::NotFound)
        }

        /// Strategy to generate Validation errors
        fn validation_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            error_message_strategy().prop_map(GitAutoDevError::Validation)
        }

        /// Strategy to generate Config errors
        fn config_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            error_message_strategy().prop_map(GitAutoDevError::Config)
        }

        /// Strategy to generate Internal errors
        fn internal_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            error_message_strategy().prop_map(GitAutoDevError::Internal)
        }

        /// Strategy to generate Database errors
        fn database_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            error_message_strategy()
                .prop_map(|msg| GitAutoDevError::Database(sea_orm::DbErr::Custom(msg)))
        }

        /// Strategy to generate Forbidden errors
        fn forbidden_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            error_message_strategy().prop_map(GitAutoDevError::Forbidden)
        }

        /// Strategy to generate ServiceUnavailable errors
        fn service_unavailable_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            error_message_strategy().prop_map(GitAutoDevError::ServiceUnavailable)
        }

        /// Strategy to generate any GitAutoDevError variant
        fn any_error_strategy() -> impl Strategy<Value = GitAutoDevError> {
            prop_oneof![
                not_found_error_strategy(),
                validation_error_strategy(),
                config_error_strategy(),
                internal_error_strategy(),
                database_error_strategy(),
                forbidden_error_strategy(),
                service_unavailable_error_strategy(),
            ]
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Feature: backend-init, Property 3: Error conversion consistency
            /// For any GitAutoDevError, converting to HTTP response produces valid status and body
            #[test]
            fn prop_error_conversion_produces_valid_http_status(error in any_error_strategy()) {
                let response = error.into_response();
                let (status, _) = extract_status_and_body(response);

                // Status must be 4xx or 5xx (client or server error)
                prop_assert!(
                    status.is_client_error() || status.is_server_error(),
                    "Status {} is not a client or server error", status
                );
            }

            /// Feature: backend-init, Property 3: Error conversion consistency
            /// For any GitAutoDevError, the response body deserializes to ErrorResponse with non-empty error
            #[test]
            fn prop_error_conversion_produces_valid_error_response(error in any_error_strategy()) {
                let response = error.into_response();
                let (_, body) = extract_status_and_body(response);

                // error field must be non-empty
                prop_assert!(!body.error.is_empty(), "Error message should not be empty");
            }

            /// Feature: backend-init, Property 3: Error conversion consistency
            /// NotFound errors always map to 404
            #[test]
            fn prop_not_found_error_returns_404(error in not_found_error_strategy()) {
                let response = error.into_response();
                let (status, body) = extract_status_and_body(response);

                prop_assert_eq!(status, StatusCode::NOT_FOUND);
                prop_assert_eq!(body.code, Some("NOT_FOUND".to_string()));
            }

            /// Feature: backend-init, Property 3: Error conversion consistency
            /// Validation errors always map to 400
            #[test]
            fn prop_validation_error_returns_400(error in validation_error_strategy()) {
                let response = error.into_response();
                let (status, body) = extract_status_and_body(response);

                prop_assert_eq!(status, StatusCode::BAD_REQUEST);
                prop_assert_eq!(body.code, Some("VALIDATION_ERROR".to_string()));
            }

            /// Feature: backend-init, Property 3: Error conversion consistency
            /// Database errors always map to 500
            #[test]
            fn prop_database_error_returns_500(error in database_error_strategy()) {
                let response = error.into_response();
                let (status, body) = extract_status_and_body(response);

                prop_assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                prop_assert_eq!(body.code, Some("DATABASE_ERROR".to_string()));
            }

            /// Feature: backend-init, Property 3: Error conversion consistency
            /// Config errors always map to 500
            #[test]
            fn prop_config_error_returns_500(error in config_error_strategy()) {
                let response = error.into_response();
                let (status, body) = extract_status_and_body(response);

                prop_assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                prop_assert_eq!(body.code, Some("CONFIG_ERROR".to_string()));
            }

            /// Feature: backend-init, Property 3: Error conversion consistency
            /// Internal errors always map to 500
            #[test]
            fn prop_internal_error_returns_500(error in internal_error_strategy()) {
                let response = error.into_response();
                let (status, body) = extract_status_and_body(response);

                prop_assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                prop_assert_eq!(body.code, Some("INTERNAL_ERROR".to_string()));
            }

            /// Feature: repository-initialization, Property: Forbidden error mapping
            /// Forbidden errors always map to 403
            /// Validates: Requirements 4.2
            #[test]
            fn prop_forbidden_error_returns_403(error in forbidden_error_strategy()) {
                let response = error.into_response();
                let (status, body) = extract_status_and_body(response);

                prop_assert_eq!(status, StatusCode::FORBIDDEN);
                prop_assert_eq!(body.code, Some("FORBIDDEN".to_string()));
            }

            /// Feature: repository-initialization, Property: ServiceUnavailable error mapping
            /// ServiceUnavailable errors always map to 503
            /// Validates: Requirements 4.3
            #[test]
            fn prop_service_unavailable_error_returns_503(error in service_unavailable_error_strategy()) {
                let response = error.into_response();
                let (status, body) = extract_status_and_body(response);

                prop_assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
                prop_assert_eq!(body.code, Some("SERVICE_UNAVAILABLE".to_string()));
            }
        }
    }
}
