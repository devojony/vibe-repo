//! Property-based tests for GitClientFactory
//!
//! Tests universal properties of the factory system using proptest.

// GitClient, GitHubClient, and GitLabClient are exported from git_provider module
// and can be imported when needed for direct enum variant construction
#[allow(unused_imports)]
use gitautodev::git_provider::{GitClient, GitHubClient, GitLabClient};
use gitautodev::git_provider::{GitClientFactory, GitProvider, GitProviderError};
use proptest::prelude::*;

// ============================================
// Property 3: Factory Creates Correct Provider Type
// Validates: Requirements 10.2
// ============================================

/// Generate arbitrary supported provider types
fn arb_supported_provider_type() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("gitea"), Just("github"), Just("gitlab"),]
}

/// Generate arbitrary unsupported provider types
fn arb_unsupported_provider_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("bitbucket".to_string()),
        Just("unknown".to_string()),
        Just("invalid".to_string()),
        Just("".to_string()),
        "[a-z]{3,10}".prop_filter("Must not be a supported provider", |s| {
            s != "gitea" && s != "github" && s != "gitlab"
        }),
        "provider-[0-9]{1,3}",
    ]
}

/// Generate arbitrary base URLs
fn arb_base_url() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("https://gitea.example.com".to_string()),
        Just("http://localhost:3000".to_string()),
        Just("https://git.company.com".to_string()),
        Just("http://192.168.1.100:8080".to_string()),
        "https://[a-z]{5,10}\\.com",
        "http://[a-z]{3,8}\\.[a-z]{3,8}\\.[a-z]{2,3}",
    ]
}

/// Generate arbitrary access tokens
fn arb_access_token() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("token123456".to_string()),
        Just("ghp_1234567890abcdef".to_string()),
        Just("glpat-abcdefghijklmnop".to_string()),
        "[a-zA-Z0-9]{20,40}",
        "[a-z]{3,5}_[a-zA-Z0-9]{16,32}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: git-provider-abstraction, Property 3: Factory Creates Correct Provider Type
    /// For any supported provider_type, the factory should return a GitProvider instance
    /// where provider_type() returns the same string
    #[test]
    fn prop_factory_creates_correct_provider_type(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let result = GitClientFactory::create(provider_type, &base_url, &access_token);

        prop_assert!(result.is_ok(), "Factory should succeed for supported provider type");

        let client = result.unwrap();
        prop_assert_eq!(
            client.provider_type(),
            provider_type,
            "Provider type should match the requested type"
        );
    }

    /// Feature: git-provider-abstraction, Property 3: Factory Creates Correct Provider Type
    /// For any supported provider_type, the factory should return a GitProvider instance
    /// where base_url() returns the normalized base URL (without trailing slash)
    #[test]
    fn prop_factory_preserves_base_url(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let result = GitClientFactory::create(provider_type, &base_url, &access_token);

        prop_assert!(result.is_ok(), "Factory should succeed for supported provider type");

        let client = result.unwrap();
        let returned_url = client.base_url();
        let expected_url = base_url.trim_end_matches('/');

        prop_assert_eq!(
            returned_url,
            expected_url,
            "Base URL should be preserved (without trailing slash)"
        );
    }

    /// Feature: git-provider-abstraction, Property 3: Factory Creates Correct Provider Type
    /// For any unsupported provider_type, the factory should return UnsupportedProvider error
    #[test]
    fn prop_factory_rejects_unsupported_provider_type(
        provider_type in arb_unsupported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let result = GitClientFactory::create(&provider_type, &base_url, &access_token);

        prop_assert!(result.is_err(), "Factory should fail for unsupported provider type");

        if let Err(error) = result {
            match error {
                GitProviderError::UnsupportedProvider(msg) => {
                    prop_assert_eq!(
                        msg,
                        provider_type,
                        "Error message should contain the unsupported provider type"
                    );
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::fail(
                        format!("Expected UnsupportedProvider error, got: {:?}", other)
                    ));
                }
            }
        }
    }

    /// Feature: git-provider-abstraction, Property 3: Factory Creates Correct Provider Type
    /// For any supported provider_type, creating multiple instances should work consistently
    #[test]
    fn prop_factory_creates_consistent_instances(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        // Create two instances with the same parameters
        let result1 = GitClientFactory::create(provider_type, &base_url, &access_token);
        let result2 = GitClientFactory::create(provider_type, &base_url, &access_token);

        prop_assert!(result1.is_ok(), "First factory call should succeed");
        prop_assert!(result2.is_ok(), "Second factory call should succeed");

        let client1 = result1.unwrap();
        let client2 = result2.unwrap();

        // Both should have the same provider type and base URL
        prop_assert_eq!(
            client1.provider_type(),
            client2.provider_type(),
            "Both instances should have the same provider type"
        );
        prop_assert_eq!(
            client1.base_url(),
            client2.base_url(),
            "Both instances should have the same base URL"
        );
    }

    /// Feature: git-provider-abstraction, Property 3: Factory Creates Correct Provider Type
    /// For any provider_type (supported or not), the factory should never panic
    #[test]
    fn prop_factory_never_panics(
        provider_type in "[a-z]{3,15}",
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        // This should never panic, only return Ok or Err
        let _result = GitClientFactory::create(&provider_type, &base_url, &access_token);
        // If we reach here, the test passes (no panic occurred)
    }

    /// Feature: git-provider-abstraction, Property 3: Factory Creates Correct Provider Type
    /// For any supported provider_type, the factory should handle base URLs with trailing slashes
    #[test]
    fn prop_factory_handles_trailing_slash(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token(),
        add_slash in prop::bool::ANY
    ) {
        let url_with_maybe_slash = if add_slash {
            format!("{}/", base_url)
        } else {
            base_url.clone()
        };

        let result = GitClientFactory::create(provider_type, &url_with_maybe_slash, &access_token);

        prop_assert!(result.is_ok(), "Factory should succeed regardless of trailing slash");

        let client = result.unwrap();
        let returned_url = client.base_url();

        // Should always return URL without trailing slash
        prop_assert!(
            !returned_url.ends_with('/'),
            "Base URL should not have trailing slash: {}",
            returned_url
        );
    }

    /// Feature: git-provider-abstraction, Property 3: Factory Creates Correct Provider Type
    /// For any supported provider_type, the factory should handle empty strings gracefully
    #[test]
    fn prop_factory_handles_empty_strings(
        provider_type in arb_supported_provider_type()
    ) {
        // Test with empty base_url
        let result1 = GitClientFactory::create(provider_type, "", "token");
        prop_assert!(result1.is_ok(), "Factory should handle empty base_url");

        // Test with empty access_token
        let result2 = GitClientFactory::create(provider_type, "https://example.com", "");
        prop_assert!(result2.is_ok(), "Factory should handle empty access_token");

        // Test with both empty
        let result3 = GitClientFactory::create(provider_type, "", "");
        prop_assert!(result3.is_ok(), "Factory should handle both empty");
    }
}
