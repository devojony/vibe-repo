//! Property-based tests for Static Dispatch Git Client
//!
//! Feature: enum-dispatch-git-client
//! Tests universal properties of the GitClient enum dispatch system using proptest.

use gitautodev::git_provider::{
    GitClient, GitClientFactory, GitHubClient, GitLabClient, GitProvider, GitProviderError,
};
use gitautodev::git_provider::gitea::GiteaClient;
use proptest::prelude::*;
use std::sync::Arc;

// ============================================
// Generators for property tests
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

// ============================================
// Property 1: Dispatch Correctness
// Validates: Requirements 2.3, 3.4
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: enum-dispatch-git-client, Property 1: Dispatch Correctness
    /// For any GitClient variant created from a specific provider type,
    /// calling provider_type() on the GitClient SHALL return the corresponding provider type string.
    /// **Validates: Requirements 2.3, 3.4**
    #[test]
    fn prop_dispatch_correctness_provider_type(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let result = GitClientFactory::create(provider_type, &base_url, &access_token);

        prop_assert!(result.is_ok(), "Factory should succeed for supported provider type");

        let client = result.unwrap();

        // The dispatch should correctly forward provider_type() to the underlying implementation
        prop_assert_eq!(
            client.provider_type(),
            provider_type,
            "Dispatch should correctly forward provider_type() call to underlying implementation"
        );
    }

    /// Feature: enum-dispatch-git-client, Property 1: Dispatch Correctness
    /// For any GitClient variant, calling base_url() should dispatch to the correct implementation.
    /// **Validates: Requirements 2.3, 3.4**
    #[test]
    fn prop_dispatch_correctness_base_url(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let result = GitClientFactory::create(provider_type, &base_url, &access_token);

        prop_assert!(result.is_ok(), "Factory should succeed for supported provider type");

        let client = result.unwrap();
        let expected_url = base_url.trim_end_matches('/');

        // The dispatch should correctly forward base_url() to the underlying implementation
        prop_assert_eq!(
            client.base_url(),
            expected_url,
            "Dispatch should correctly forward base_url() call to underlying implementation"
        );
    }

    /// Feature: enum-dispatch-git-client, Property 1: Dispatch Correctness
    /// For any directly constructed GitClient variant, dispatch should work correctly.
    /// **Validates: Requirements 2.3, 3.4**
    #[test]
    fn prop_dispatch_correctness_direct_construction(
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        // Test Gitea variant
        let gitea_client = GitClient::Gitea(GiteaClient::new(&base_url, &access_token));
        prop_assert_eq!(
            gitea_client.provider_type(),
            "gitea",
            "Gitea variant should dispatch to gitea provider_type"
        );

        // Test GitHub variant
        let github_client = GitClient::GitHub(GitHubClient::new(&base_url, &access_token));
        prop_assert_eq!(
            github_client.provider_type(),
            "github",
            "GitHub variant should dispatch to github provider_type"
        );

        // Test GitLab variant
        let gitlab_client = GitClient::GitLab(GitLabClient::new(&base_url, &access_token));
        prop_assert_eq!(
            gitlab_client.provider_type(),
            "gitlab",
            "GitLab variant should dispatch to gitlab provider_type"
        );
    }

    /// Feature: enum-dispatch-git-client, Property 1: Dispatch Correctness
    /// For any GitClient variant, the factory-created and directly-constructed clients
    /// should have identical dispatch behavior.
    /// **Validates: Requirements 2.3, 3.4**
    #[test]
    fn prop_dispatch_correctness_factory_vs_direct(
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        // Create via factory
        let factory_gitea = GitClientFactory::create("gitea", &base_url, &access_token).unwrap();
        let factory_github = GitClientFactory::create("github", &base_url, &access_token).unwrap();
        let factory_gitlab = GitClientFactory::create("gitlab", &base_url, &access_token).unwrap();

        // Create directly
        let direct_gitea = GitClient::Gitea(GiteaClient::new(&base_url, &access_token));
        let direct_github = GitClient::GitHub(GitHubClient::new(&base_url, &access_token));
        let direct_gitlab = GitClient::GitLab(GitLabClient::new(&base_url, &access_token));

        // Verify dispatch behavior is identical
        prop_assert_eq!(
            factory_gitea.provider_type(),
            direct_gitea.provider_type(),
            "Factory and direct Gitea clients should have same provider_type"
        );
        prop_assert_eq!(
            factory_github.provider_type(),
            direct_github.provider_type(),
            "Factory and direct GitHub clients should have same provider_type"
        );
        prop_assert_eq!(
            factory_gitlab.provider_type(),
            direct_gitlab.provider_type(),
            "Factory and direct GitLab clients should have same provider_type"
        );

        // Verify base_url dispatch is identical
        prop_assert_eq!(
            factory_gitea.base_url(),
            direct_gitea.base_url(),
            "Factory and direct Gitea clients should have same base_url"
        );
        prop_assert_eq!(
            factory_github.base_url(),
            direct_github.base_url(),
            "Factory and direct GitHub clients should have same base_url"
        );
        prop_assert_eq!(
            factory_gitlab.base_url(),
            direct_gitlab.base_url(),
            "Factory and direct GitLab clients should have same base_url"
        );
    }
}

// ============================================
// Property 2: Factory Error Handling
// Validates: Requirements 3.3
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: enum-dispatch-git-client, Property 2: Factory Error Handling
    /// For any string that is not a supported provider type ("gitea", "github", "gitlab"),
    /// the GitClientFactory::create method SHALL return Err(GitProviderError::UnsupportedProvider(_)).
    /// **Validates: Requirements 3.3**
    #[test]
    fn prop_factory_error_handling_unsupported_provider(
        provider_type in arb_unsupported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let result = GitClientFactory::create(&provider_type, &base_url, &access_token);

        prop_assert!(
            result.is_err(),
            "Factory should return error for unsupported provider type: {}",
            provider_type
        );

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
                    return Err(proptest::test_runner::TestCaseError::fail(format!(
                        "Expected UnsupportedProvider error, got: {:?}",
                        other
                    )));
                }
            }
        }
    }

    /// Feature: enum-dispatch-git-client, Property 2: Factory Error Handling
    /// For any arbitrary string, the factory should either succeed (for supported types)
    /// or return UnsupportedProvider error (for unsupported types), never panic.
    /// **Validates: Requirements 3.3**
    #[test]
    fn prop_factory_error_handling_never_panics(
        provider_type in "[a-zA-Z0-9_-]{0,20}",
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let result = GitClientFactory::create(&provider_type, &base_url, &access_token);

        // Should either succeed or return UnsupportedProvider error
        match result {
            Ok(client) => {
                // If it succeeded, it must be a supported provider
                let actual_type = client.provider_type();
                prop_assert!(
                    actual_type == "gitea" || actual_type == "github" || actual_type == "gitlab",
                    "Successful creation should only happen for supported providers"
                );
            }
            Err(GitProviderError::UnsupportedProvider(_)) => {
                // Expected error for unsupported providers
            }
            Err(other) => {
                return Err(proptest::test_runner::TestCaseError::fail(format!(
                    "Unexpected error type: {:?}",
                    other
                )));
            }
        }
    }

    /// Feature: enum-dispatch-git-client, Property 2: Factory Error Handling
    /// For case-sensitive provider types, only exact matches should succeed.
    /// **Validates: Requirements 3.3**
    #[test]
    fn prop_factory_error_handling_case_sensitive(
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        // Test uppercase variants - should fail
        let uppercase_variants = ["GITEA", "GITHUB", "GITLAB", "Gitea", "GitHub", "GitLab"];

        for variant in uppercase_variants {
            let result = GitClientFactory::create(variant, &base_url, &access_token);
            prop_assert!(
                result.is_err(),
                "Factory should reject case-variant provider type: {}",
                variant
            );

            if let Err(GitProviderError::UnsupportedProvider(msg)) = result {
                prop_assert_eq!(
                    msg, variant,
                    "Error should contain the exact input string"
                );
            }
        }
    }
}

// ============================================
// Property 3: Thread Safety (Send + Sync)
// Validates: Requirements 6.1, 6.2, 6.4
// ============================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: enum-dispatch-git-client, Property 3: Thread Safety
    /// For any GitClient instance, it SHALL be usable across thread boundaries (Send + Sync).
    /// **Validates: Requirements 6.1, 6.2, 6.4**
    #[test]
    fn prop_thread_safety_send_sync(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let client = GitClientFactory::create(provider_type, &base_url, &access_token).unwrap();

        // Verify Send + Sync at compile time
        fn assert_send<T: Send>(_: &T) {}
        fn assert_sync<T: Sync>(_: &T) {}

        assert_send(&client);
        assert_sync(&client);

        // Test that we can move the client to another thread
        let handle = std::thread::spawn(move || {
            // Access the client from another thread
            let _ = client.provider_type();
            let _ = client.base_url();
            true
        });

        let result = handle.join();
        prop_assert!(result.is_ok(), "Thread should complete successfully");
        prop_assert!(result.unwrap(), "Thread should return true");
    }

    /// Feature: enum-dispatch-git-client, Property 3: Thread Safety
    /// For any GitClient instance, it SHALL be usable in Arc for shared ownership.
    /// **Validates: Requirements 6.1, 6.2, 6.4**
    #[test]
    fn prop_thread_safety_arc_shared_ownership(
        provider_type in arb_supported_provider_type(),
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        let client = GitClientFactory::create(provider_type, &base_url, &access_token).unwrap();
        let arc_client = Arc::new(client);

        // Clone Arc for multiple threads
        let arc1 = Arc::clone(&arc_client);
        let arc2 = Arc::clone(&arc_client);

        // Spawn multiple threads accessing the same client
        let handle1 = std::thread::spawn(move || {
            arc1.provider_type().to_string()
        });

        let handle2 = std::thread::spawn(move || {
            arc2.base_url().to_string()
        });

        let result1 = handle1.join();
        let result2 = handle2.join();

        prop_assert!(result1.is_ok(), "First thread should complete successfully");
        prop_assert!(result2.is_ok(), "Second thread should complete successfully");

        prop_assert_eq!(
            result1.unwrap(),
            provider_type,
            "First thread should get correct provider_type"
        );

        let expected_url = base_url.trim_end_matches('/');
        prop_assert_eq!(
            result2.unwrap(),
            expected_url,
            "Second thread should get correct base_url"
        );
    }

    /// Feature: enum-dispatch-git-client, Property 3: Thread Safety
    /// For any directly constructed GitClient variant, Send + Sync should hold.
    /// **Validates: Requirements 6.1, 6.2, 6.4**
    #[test]
    fn prop_thread_safety_all_variants(
        base_url in arb_base_url(),
        access_token in arb_access_token()
    ) {
        // Test all variants are Send + Sync
        fn assert_send_sync<T: Send + Sync>(_: &T) {}

        let gitea = GitClient::Gitea(GiteaClient::new(&base_url, &access_token));
        let github = GitClient::GitHub(GitHubClient::new(&base_url, &access_token));
        let gitlab = GitClient::GitLab(GitLabClient::new(&base_url, &access_token));

        assert_send_sync(&gitea);
        assert_send_sync(&github);
        assert_send_sync(&gitlab);

        // Test Arc wrapping for all variants
        let arc_gitea = Arc::new(gitea);
        let arc_github = Arc::new(github);
        let arc_gitlab = Arc::new(gitlab);

        // Verify Arc<GitClient> is also Send + Sync
        assert_send_sync(&arc_gitea);
        assert_send_sync(&arc_github);
        assert_send_sync(&arc_gitlab);
    }
}

// ============================================
// Compile-time assertions for Send + Sync
// ============================================

/// Static assertion that GitClient implements Send
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<GitClient>();
};

/// Static assertion that GitClient implements Sync
const _: () = {
    const fn assert_sync<T: Sync>() {}
    assert_sync::<GitClient>();
};

/// Static assertion that Arc<GitClient> implements Send + Sync
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Arc<GitClient>>();
};
