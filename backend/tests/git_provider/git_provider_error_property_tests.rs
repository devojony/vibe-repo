//! Property-based tests for GitProviderError
//!
//! Tests universal properties of the error handling system using proptest.

use vibe_repo::git_provider::GitProviderError;
use proptest::prelude::*;

// ============================================
// Property 1: HTTP Status Code to Error Mapping
// Validates: Requirements 9.2, 9.3, 9.4, 9.5, 9.6
// ============================================

/// Generate arbitrary HTTP status codes
fn arb_http_status() -> impl Strategy<Value = u16> {
    prop_oneof![
        // Success codes (should map to Internal)
        Just(200u16),
        Just(201u16),
        Just(204u16),
        // Client error codes (specific mappings)
        Just(401u16), // Unauthorized
        Just(403u16), // Forbidden
        Just(404u16), // NotFound
        Just(409u16), // Conflict
        Just(422u16), // ValidationError
        // Other client errors (should map to Internal)
        Just(400u16),
        Just(405u16),
        Just(410u16),
        Just(429u16),
        // Server error codes (should map to Internal)
        Just(500u16),
        Just(502u16),
        Just(503u16),
        Just(504u16),
        // Random status codes
        400u16..600u16,
    ]
}

/// Generate arbitrary error messages
fn arb_error_message() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Error occurred".to_string()),
        Just("Invalid request".to_string()),
        Just("Resource not found".to_string()),
        Just("Access denied".to_string()),
        "[A-Z][a-z]{5,20} [a-z]{3,10}",
        "Error: [a-z]{3,15}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any HTTP status code 401, from_status should return Unauthorized variant
    #[test]
    fn prop_status_401_maps_to_unauthorized(message in arb_error_message()) {
        let error = GitProviderError::from_status(401, message.clone());

        match error {
            GitProviderError::Unauthorized(msg) => {
                prop_assert_eq!(msg, message, "Message should be preserved");
            }
            _ => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    "Status 401 should map to Unauthorized"
                ));
            }
        }
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any HTTP status code 403, from_status should return Forbidden variant
    #[test]
    fn prop_status_403_maps_to_forbidden(message in arb_error_message()) {
        let error = GitProviderError::from_status(403, message.clone());

        match error {
            GitProviderError::Forbidden(msg) => {
                prop_assert_eq!(msg, message, "Message should be preserved");
            }
            _ => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    "Status 403 should map to Forbidden"
                ));
            }
        }
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any HTTP status code 404, from_status should return NotFound variant
    #[test]
    fn prop_status_404_maps_to_not_found(message in arb_error_message()) {
        let error = GitProviderError::from_status(404, message.clone());

        match error {
            GitProviderError::NotFound(msg) => {
                prop_assert_eq!(msg, message, "Message should be preserved");
            }
            _ => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    "Status 404 should map to NotFound"
                ));
            }
        }
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any HTTP status code 409, from_status should return Conflict variant
    #[test]
    fn prop_status_409_maps_to_conflict(message in arb_error_message()) {
        let error = GitProviderError::from_status(409, message.clone());

        match error {
            GitProviderError::Conflict(msg) => {
                prop_assert_eq!(msg, message, "Message should be preserved");
            }
            _ => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    "Status 409 should map to Conflict"
                ));
            }
        }
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any HTTP status code 422, from_status should return ValidationError variant
    #[test]
    fn prop_status_422_maps_to_validation_error(message in arb_error_message()) {
        let error = GitProviderError::from_status(422, message.clone());

        match error {
            GitProviderError::ValidationError(msg) => {
                prop_assert_eq!(msg, message, "Message should be preserved");
            }
            _ => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    "Status 422 should map to ValidationError"
                ));
            }
        }
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any HTTP status code not in [401, 403, 404, 409, 422], from_status should return Internal variant
    #[test]
    fn prop_other_status_maps_to_internal(
        status in prop::sample::select(vec![200u16, 400, 405, 500, 502, 503]),
        message in arb_error_message()
    ) {
        let error = GitProviderError::from_status(status, message.clone());

        match error {
            GitProviderError::Internal(msg) => {
                // Should contain both status code and message
                prop_assert!(msg.contains(&status.to_string()), "Should contain status code");
                prop_assert!(msg.contains(&message), "Should contain original message");
            }
            _ => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    format!("Status {} should map to Internal", status)
                ));
            }
        }
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any status code and message, from_status should preserve the message content
    #[test]
    fn prop_from_status_preserves_message(
        status in arb_http_status(),
        message in arb_error_message()
    ) {
        let error = GitProviderError::from_status(status, message.clone());
        let error_string = error.to_string();

        // The error message should contain the original message
        prop_assert!(
            error_string.contains(&message),
            "Error string should contain original message: {} not in {}",
            message,
            error_string
        );
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any status code, from_status should return a valid error variant
    #[test]
    fn prop_from_status_returns_valid_error(
        status in arb_http_status(),
        message in arb_error_message()
    ) {
        let error = GitProviderError::from_status(status, message.clone());

        // Verify the error can be displayed (implements Display trait)
        let error_string = error.to_string();
        prop_assert!(!error_string.is_empty(), "Error string should not be empty");

        // Verify the error implements std::error::Error trait
        let _: &dyn std::error::Error = &error;
    }

    /// Feature: git-provider-abstraction, Property 1: HTTP Status Code to Error Mapping
    /// For any mapped status code (401, 403, 404, 409, 422), the error variant should match
    #[test]
    fn prop_mapped_status_codes_consistency(
        message in arb_error_message()
    ) {
        let test_cases = vec![
            (401u16, "Unauthorized"),
            (403u16, "Forbidden"),
            (404u16, "NotFound"),
            (409u16, "Conflict"),
            (422u16, "ValidationError"),
        ];

        for (status, expected_variant) in test_cases {
            let error = GitProviderError::from_status(status, message.clone());
            let error_string = format!("{:?}", error);

            prop_assert!(
                error_string.contains(expected_variant),
                "Status {} should map to {} variant, got: {}",
                status,
                expected_variant,
                error_string
            );
        }
    }
}
