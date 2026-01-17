//! Property-based tests for RepoProvider API
//!
//! Tests universal properties of the provider system using proptest.

use vibe_repo::api::settings::providers::models::mask_token;
use proptest::prelude::*;

// ============================================
// Property 1: Token masking consistency
// Validates: Requirements 1.7, 2.2, 3.2, 4.6, 7.1, 7.2, 7.3, 7.4
// ============================================

/// Generate arbitrary tokens of various lengths
fn arb_token() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty token
        Just("".to_string()),
        // Very short tokens (1-7 chars)
        "[a-zA-Z0-9]{1,7}",
        // Exactly 8 chars
        "[a-zA-Z0-9]{8}",
        // 9-20 chars
        "[a-zA-Z0-9]{9,20}",
        // Long tokens (21-100 chars)
        "[a-zA-Z0-9]{21,100}",
        // Tokens with special characters
        "[a-zA-Z0-9!@#$%^&*()_+\\-=\\[\\]{}|;:',.<>?/]{9,50}",
        // Tokens with unicode
        prop::string::string_regex("token[🔑🔐🔒🔓]{0,5}[a-z]{0,10}").unwrap(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 1: Token masking consistency
    /// For any token of any length, the mask_token function should:
    /// - Return "***" if token length <= 8 characters
    /// - Return first 8 characters + "***" if token length > 8 characters
    #[test]
    fn prop_token_masking_consistency(token in arb_token()) {
        let masked = mask_token(&token);
        let char_count = token.chars().count();

        if char_count <= 8 {
            prop_assert_eq!(masked, "***", "Tokens with <= 8 chars should mask to '***'");
        } else {
            let prefix: String = token.chars().take(8).collect();
            let expected = format!("{}***", prefix);
            prop_assert_eq!(masked, expected, "Tokens with > 8 chars should show first 8 + '***'");
        }
    }

    /// Feature: repo-provider-api, Property 1: Token masking consistency
    /// For any token, the masked version should always end with "***"
    #[test]
    fn prop_masked_token_always_ends_with_stars(token in arb_token()) {
        let masked = mask_token(&token);
        prop_assert!(masked.ends_with("***"), "Masked token should always end with '***'");
    }

    /// Feature: repo-provider-api, Property 1: Token masking consistency
    /// For any token longer than 8 characters, the masked version should be exactly 11 characters longer
    /// than the original prefix (8 chars + "***" = 11 chars total minimum)
    #[test]
    fn prop_masked_token_length_consistency(token in "[a-zA-Z0-9]{9,100}") {
        let masked = mask_token(&token);
        // Masked should be exactly 11 characters (8 prefix + 3 stars)
        prop_assert_eq!(masked.chars().count(), 11, "Masked token should be 11 characters (8 + ***)");
    }
}

// ============================================
// Property 2: CRUD round-trip consistency
// Validates: Requirements 1.1, 3.1, 3.4
// ============================================

use axum::body::Body;
use axum::http::Request;
use vibe_repo::api::settings::providers::models::ProviderResponse;
use vibe_repo::test_utils::state::create_test_app;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

/// Generate arbitrary provider names
fn arb_provider_name() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Z][a-z]{3,15}",
        "Test [A-Z][a-z]{3,10}",
        "[A-Z][a-z]+ [A-Z][a-z]+ Provider",
    ]
}

/// Generate arbitrary base URLs
fn arb_base_url() -> impl Strategy<Value = String> {
    prop_oneof![
        "https://gitea\\.example\\.com",
        "https://git\\.[a-z]{3,10}\\.com",
        "https://[a-z]{3,10}\\.gitea\\.io",
        "http://localhost:[0-9]{4}",
    ]
}

/// Generate arbitrary access tokens
fn arb_access_token() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-f0-9]{40}",
        "ghp_[a-zA-Z0-9]{36}",
        "glpat-[a-zA-Z0-9_\\-]{20}",
        "[a-zA-Z0-9]{32,64}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 2: CRUD round-trip consistency
    /// For any valid provider data, creating then retrieving the provider should return
    /// equivalent data (except for masked token and auto-generated fields).
    #[test]
    fn prop_crud_round_trip_consistency(
        name in arb_provider_name(),
        base_url in arb_base_url(),
        access_token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            // Create provider
            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": access_token
            });

            let create_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(create_response.status(), 201, "Create should return 201");

            let body = create_response.into_body().collect().await.unwrap().to_bytes();
            let created: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Retrieve provider
            let get_response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(&format!("/api/settings/providers/{}", created.id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(get_response.status(), 200, "Get should return 200");

            let body = get_response.into_body().collect().await.unwrap().to_bytes();
            let retrieved: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Verify equivalence (except auto-generated fields)
            prop_assert_eq!(retrieved.id, created.id, "IDs should match");
            prop_assert_eq!(retrieved.name, name, "Names should match");
            prop_assert_eq!(retrieved.base_url, base_url, "Base URLs should match");
            prop_assert_eq!(retrieved.provider_type, created.provider_type, "Provider types should match");
            prop_assert_eq!(retrieved.locked, false, "Locked should default to false");

            // Verify token is masked consistently
            let expected_masked = mask_token(&access_token);
            prop_assert_eq!(retrieved.access_token, expected_masked.clone(), "Token should be masked consistently");
            prop_assert_eq!(created.access_token, expected_masked, "Created token should be masked");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 3: Partial update correctness
// Validates: Requirements 4.1, 4.2, 4.5, 4.9
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 3: Partial update correctness
    /// For any provider and any subset of updateable fields, updating only those fields
    /// should leave all other fields unchanged (except updated_at).
    #[test]
    fn prop_partial_update_correctness(
        original_name in arb_provider_name(),
        original_base_url in arb_base_url(),
        original_token in arb_access_token(),
        new_name in arb_provider_name(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            // Create provider
            let request_body = json!({
                "name": original_name,
                "provider_type": "gitea",
                "base_url": original_base_url,
                "access_token": original_token
            });

            let create_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = create_response.into_body().collect().await.unwrap().to_bytes();
            let created: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Update only the name
            let update_body = json!({
                "name": new_name
            });

            let update_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(&format!("/api/settings/providers/{}", created.id))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(update_response.status(), 200, "Update should return 200");

            let body = update_response.into_body().collect().await.unwrap().to_bytes();
            let updated: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Verify name was updated
            prop_assert_eq!(updated.name, new_name, "Name should be updated");

            // Verify other fields unchanged
            prop_assert_eq!(updated.id, created.id, "ID should not change");
            prop_assert_eq!(updated.base_url, original_base_url, "Base URL should not change");
            prop_assert_eq!(updated.provider_type, created.provider_type, "Provider type should not change");
            prop_assert_eq!(updated.locked, created.locked, "Locked should not change");
            prop_assert_eq!(updated.access_token, created.access_token, "Token should not change");
            prop_assert_eq!(updated.created_at, created.created_at, "Created_at should not change");

            Ok(())
        }).unwrap();
    }

    /// Feature: repo-provider-api, Property 3: Partial update correctness
    /// For any provider, updating the locked field should not affect other fields.
    #[test]
    fn prop_partial_update_locked_field(
        name in arb_provider_name(),
        base_url in arb_base_url(),
        token in arb_access_token(),
        locked_value in prop::bool::ANY,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            // Create provider
            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": token
            });

            let create_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = create_response.into_body().collect().await.unwrap().to_bytes();
            let created: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Update only the locked field
            let update_body = json!({
                "locked": locked_value
            });

            let update_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(&format!("/api/settings/providers/{}", created.id))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(update_response.status(), 200, "Update should return 200");

            let body = update_response.into_body().collect().await.unwrap().to_bytes();
            let updated: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Verify locked was updated
            prop_assert_eq!(updated.locked, locked_value, "Locked should be updated");

            // Verify other fields unchanged
            prop_assert_eq!(updated.id, created.id, "ID should not change");
            prop_assert_eq!(updated.name, name, "Name should not change");
            prop_assert_eq!(updated.base_url, base_url, "Base URL should not change");
            prop_assert_eq!(updated.provider_type, created.provider_type, "Provider type should not change");
            prop_assert_eq!(updated.access_token, created.access_token, "Token should not change");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 4: Input validation consistency
// Validates: Requirements 1.2, 1.3, 1.4, 1.5, 4.3, 4.8, 9.2, 9.3, 9.5
// ============================================

use axum::http::StatusCode;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 4: Input validation consistency
    /// For any provider with empty name, the API should return 400 Bad Request.
    #[test]
    fn prop_input_validation_empty_name(
        base_url in arb_base_url(),
        token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            let request_body = json!({
                "name": "",
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": token
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status(), StatusCode::BAD_REQUEST, "Empty name should return 400");

            Ok(())
        }).unwrap();
    }

    /// Feature: repo-provider-api, Property 4: Input validation consistency
    /// For any provider with empty access_token, the API should return 400 Bad Request.
    #[test]
    fn prop_input_validation_empty_token(
        name in arb_provider_name(),
        base_url in arb_base_url(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": ""
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status(), StatusCode::BAD_REQUEST, "Empty token should return 400");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 5: Not found error consistency
// Validates: Requirements 3.3, 4.4, 5.4
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 5: Not found error consistency
    /// For any non-existent provider ID, GET should return 404.
    #[test]
    fn prop_not_found_get(non_existent_id in 10000i32..99999i32) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(&format!("/api/settings/providers/{}", non_existent_id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status(), StatusCode::NOT_FOUND, "GET non-existent should return 404");

            Ok(())
        }).unwrap();
    }

    /// Feature: repo-provider-api, Property 5: Not found error consistency
    /// For any non-existent provider ID, PUT should return 404.
    #[test]
    fn prop_not_found_put(
        non_existent_id in 10000i32..99999i32,
        name in arb_provider_name(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            let update_body = json!({
                "name": name
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(&format!("/api/settings/providers/{}", non_existent_id))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status(), StatusCode::NOT_FOUND, "PUT non-existent should return 404");

            Ok(())
        }).unwrap();
    }

    /// Feature: repo-provider-api, Property 5: Not found error consistency
    /// For any non-existent provider ID, DELETE should return 404.
    #[test]
    fn prop_not_found_delete(non_existent_id in 10000i32..99999i32) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            let response = app
                .oneshot(
                    Request::builder()
                        .method("DELETE")
                        .uri(&format!("/api/settings/providers/{}", non_existent_id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status(), StatusCode::NOT_FOUND, "DELETE non-existent should return 404");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 6: Creation returns 201
// Validates: Requirements 1.1, 1.6, 1.7, 1.8
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 6: Creation returns 201
    /// For any valid provider data, creation should return 201 with correct data.
    #[test]
    fn prop_creation_returns_201(
        name in arb_provider_name(),
        base_url in arb_base_url(),
        token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": token
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(response.status(), StatusCode::CREATED, "Creation should return 201");

            let body = response.into_body().collect().await.unwrap().to_bytes();
            let provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Verify response data
            prop_assert_eq!(provider.name, name, "Name should match");
            prop_assert_eq!(provider.base_url, base_url, "Base URL should match");
            prop_assert!(!provider.locked, "Locked should default to false");
            prop_assert!(provider.access_token.ends_with("***"), "Token should be masked");
            prop_assert!(!provider.created_at.is_empty(), "Created_at should not be empty");
            prop_assert!(!provider.updated_at.is_empty(), "Updated_at should not be empty");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 7: Deletion returns 204
// Validates: Requirements 5.3, 5.5
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 7: Deletion returns 204
    /// For any unlocked provider, deletion should return 204.
    #[test]
    fn prop_deletion_returns_204(
        name in arb_provider_name(),
        base_url in arb_base_url(),
        token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            // Create provider
            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": token
            });

            let create_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = create_response.into_body().collect().await.unwrap().to_bytes();
            let created: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Delete provider
            let delete_response = app
                .oneshot(
                    Request::builder()
                        .method("DELETE")
                        .uri(&format!("/api/settings/providers/{}", created.id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(delete_response.status(), StatusCode::NO_CONTENT, "Deletion should return 204");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 7.1: Locked provider cannot be deleted
// Validates: Requirements 5.2
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 7.1: Locked provider cannot be deleted
    /// For any locked provider, deletion should return 409.
    #[test]
    fn prop_locked_provider_cannot_be_deleted(
        name in arb_provider_name(),
        base_url in arb_base_url(),
        token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            // Create provider
            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": token
            });

            let create_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = create_response.into_body().collect().await.unwrap().to_bytes();
            let created: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Lock the provider
            let lock_body = json!({
                "locked": true
            });

            let _lock_response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(&format!("/api/settings/providers/{}", created.id))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&lock_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            // Try to delete the locked provider
            let delete_response = app
                .oneshot(
                    Request::builder()
                        .method("DELETE")
                        .uri(&format!("/api/settings/providers/{}", created.id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(delete_response.status(), StatusCode::CONFLICT, "Locked provider deletion should return 409");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 8: Response schema completeness
// Validates: Requirements 2.4, 3.4, 9.1, 9.4
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 8: Response schema completeness
    /// For any provider, the response should include all required fields.
    #[test]
    fn prop_response_schema_completeness(
        name in arb_provider_name(),
        base_url in arb_base_url(),
        token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            // Create provider
            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "base_url": base_url,
                "access_token": token
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = response.into_body().collect().await.unwrap().to_bytes();
            let provider: ProviderResponse = serde_json::from_slice(&body).unwrap();

            // Verify all required fields are present and non-empty
            prop_assert!(provider.id > 0, "ID should be positive");
            prop_assert!(!provider.name.is_empty(), "Name should not be empty");
            prop_assert!(!provider.base_url.is_empty(), "Base URL should not be empty");
            prop_assert!(!provider.access_token.is_empty(), "Access token should not be empty");
            prop_assert!(provider.access_token.ends_with("***"), "Token should be masked");
            prop_assert!(!provider.created_at.is_empty(), "Created_at should not be empty");
            prop_assert!(!provider.updated_at.is_empty(), "Updated_at should not be empty");

            // Verify timestamps are in ISO 8601 format (basic check)
            prop_assert!(provider.created_at.contains('T'), "Created_at should be ISO 8601");
            prop_assert!(provider.updated_at.contains('T'), "Updated_at should be ISO 8601");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 9: Gitea base URL required
// Validates: Requirements 1.5
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repo-provider-api, Property 9: Gitea base URL required
    /// For any gitea provider without base_url, validation should fail.
    #[test]
    fn prop_gitea_base_url_required(
        name in arb_provider_name(),
        token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");

            // Try to create provider without base_url
            let request_body = json!({
                "name": name,
                "provider_type": "gitea",
                "access_token": token
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/settings/providers")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();

            // Should return 400 or 422 for missing base_url
            prop_assert!(
                response.status() == StatusCode::BAD_REQUEST ||
                response.status() == StatusCode::UNPROCESSABLE_ENTITY,
                "Missing base_url should return 400 or 422"
            );

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Property 10: List returns all providers
// Validates: Requirements 2.1, 2.2
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]  // Reduced cases for performance

    /// Feature: repo-provider-api, Property 10: List returns all providers
    /// For any number of providers, list should return all of them.
    #[test]
    fn prop_list_returns_all_providers(
        provider_count in 1usize..5usize,  // Test with 1-4 providers
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let app = create_test_app().await.expect("Failed to create test app");
            let mut created_ids = Vec::new();

            // Create multiple providers
            for i in 0..provider_count {
                let request_body = json!({
                    "name": format!("Provider {}", i),
                    "provider_type": "gitea",
                    "base_url": format!("https://gitea{}.example.com", i),
                    "access_token": format!("token_{}_12345678", i)
                });

                let create_response = app.clone()
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri("/api/settings/providers")
                            .header("content-type", "application/json")
                            .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                let body = create_response.into_body().collect().await.unwrap().to_bytes();
                let created: ProviderResponse = serde_json::from_slice(&body).unwrap();
                created_ids.push(created.id);
            }

            // List all providers
            let list_response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/settings/providers")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            prop_assert_eq!(list_response.status(), StatusCode::OK, "List should return 200");

            let body = list_response.into_body().collect().await.unwrap().to_bytes();
            let providers: Vec<ProviderResponse> = serde_json::from_slice(&body).unwrap();

            // Verify all created providers are in the list
            prop_assert!(providers.len() >= provider_count, "List should contain at least {} providers", provider_count);

            for created_id in created_ids {
                prop_assert!(
                    providers.iter().any(|p| p.id == created_id),
                    "List should contain provider with ID {}",
                    created_id
                );
            }

            // Verify all tokens are masked
            for provider in providers {
                prop_assert!(provider.access_token.ends_with("***"), "All tokens should be masked");
            }

            Ok(())
        }).unwrap();
    }
}
