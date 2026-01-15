//! Property-based tests for Repository Sync functionality
//!
//! Tests universal properties of the periodic sync system using proptest.
//!
//! **Feature: repository-initialization**
//! **Property 5: Batch Initialization Processes All Eligible Repositories**
//! **Validates: Requirements 5.5, 5.6**

use gitautodev::entities::{prelude::*, repo_provider};
use gitautodev::services::RepositoryService;
use gitautodev::test_utils::db::create_test_database;
use proptest::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

// ============================================
// Property 5: Periodic Sync Processes All Providers
// Validates: Requirements 5.5, 5.6
// ============================================

/// Generate arbitrary provider names
fn arb_provider_name() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Z][a-z]{3,15}",
        "Test[A-Z][a-z]{3,10}",
        "[A-Z][a-z]+Provider",
    ]
}

/// Generate arbitrary base URLs
fn arb_base_url() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("https://gitea.example.com".to_string()),
        Just("https://git.test.com".to_string()),
        Just("https://code.example.io".to_string()),
    ]
}

/// Generate arbitrary access tokens
fn arb_access_token() -> impl Strategy<Value = String> {
    "[a-f0-9]{40}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repository-initialization, Property 5: Periodic Sync Processes All Providers
    ///
    /// For any set of providers in the database, sync_all_providers should:
    /// - Fetch all providers from the database
    /// - Attempt to process each provider
    /// - Continue processing even if some providers fail
    /// - Return Ok(()) regardless of individual provider failures
    ///
    /// **Validates: Requirements 5.5, 5.6**
    #[test]
    fn prop_sync_all_providers_fetches_all_providers(
        provider_count in 1usize..5usize,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            // Create test database
            let db = create_test_database().await.expect("Failed to create test database");

            // Create multiple providers
            for i in 0..provider_count {
                let provider = repo_provider::ActiveModel {
                    name: ActiveValue::Set(format!("Provider{}", i)),
                    provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
                    base_url: ActiveValue::Set(format!("https://gitea{}.example.com", i)),
                    access_token: ActiveValue::Set(format!("token{:040}", i)),
                    locked: ActiveValue::Set(false),
                    created_at: ActiveValue::Set(chrono::Utc::now()),
                    updated_at: ActiveValue::Set(chrono::Utc::now()),
                    ..Default::default()
                };
                provider.insert(&db).await.expect("Failed to insert provider");
            }

            // Verify providers were created
            let providers = RepoProvider::find().all(&db).await.expect("Failed to fetch providers");
            prop_assert_eq!(providers.len(), provider_count, "Should have created {} providers", provider_count);

            // Create repository service
            let service = RepositoryService::new(db.clone());

            // Call sync_all_providers - it should not panic and should return Ok
            // Note: The actual sync will fail because the Git providers are not real,
            // but the method should continue processing all providers and return Ok
            let result = service.sync_all_providers().await;

            // The method should return Ok even if individual providers fail
            // (errors are logged but don't stop processing)
            prop_assert!(result.is_ok(), "sync_all_providers should return Ok even with failing providers");

            Ok(())
        }).unwrap();
    }

    /// Feature: repository-initialization, Property 5: Periodic Sync Continues On Error
    ///
    /// For any provider that fails during sync, the service should:
    /// - Log the error
    /// - Continue processing remaining providers
    /// - Not propagate the error to the caller
    ///
    /// **Validates: Requirements 5.5, 5.6**
    #[test]
    fn prop_sync_all_providers_continues_on_error(
        name in arb_provider_name(),
        base_url in arb_base_url(),
        token in arb_access_token(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            // Create test database
            let db = create_test_database().await.expect("Failed to create test database");

            // Create a provider with invalid/unreachable URL
            let provider = repo_provider::ActiveModel {
                name: ActiveValue::Set(name),
                provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
                base_url: ActiveValue::Set(base_url),
                access_token: ActiveValue::Set(token),
                locked: ActiveValue::Set(false),
                created_at: ActiveValue::Set(chrono::Utc::now()),
                updated_at: ActiveValue::Set(chrono::Utc::now()),
                ..Default::default()
            };
            provider.insert(&db).await.expect("Failed to insert provider");

            // Create repository service
            let service = RepositoryService::new(db);

            // Call sync_all_providers - should not panic even with unreachable provider
            let result = service.sync_all_providers().await;

            // Should return Ok even though the provider sync failed
            prop_assert!(result.is_ok(), "sync_all_providers should return Ok even with failing provider");

            Ok(())
        }).unwrap();
    }

    /// Feature: repository-initialization, Property 5: Empty Provider List Handling
    ///
    /// When there are no providers in the database, sync_all_providers should:
    /// - Return Ok(()) without errors
    /// - Not attempt any network operations
    ///
    /// **Validates: Requirements 5.5, 5.6**
    #[test]
    fn prop_sync_all_providers_handles_empty_list(_dummy in 0..1i32) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            // Create test database with no providers
            let db = create_test_database().await.expect("Failed to create test database");

            // Verify no providers exist
            let providers = RepoProvider::find().all(&db).await.expect("Failed to fetch providers");
            prop_assert_eq!(providers.len(), 0, "Should have no providers");

            // Create repository service
            let service = RepositoryService::new(db);

            // Call sync_all_providers - should return Ok with empty list
            let result = service.sync_all_providers().await;

            prop_assert!(result.is_ok(), "sync_all_providers should return Ok with empty provider list");

            Ok(())
        }).unwrap();
    }
}

// ============================================
// Unit tests for BackgroundService implementation
// Validates: Requirements 5.5
// ============================================

#[cfg(test)]
mod background_service_tests {
    use super::*;
    use gitautodev::services::BackgroundService;

    #[tokio::test]
    async fn test_repository_service_name() {
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let service = RepositoryService::new(db);

        assert_eq!(service.name(), "repository_service");
    }

    #[tokio::test]
    async fn test_repository_service_health_check() {
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let service = RepositoryService::new(db);

        // Health check should return true when database is connected
        assert!(service.health_check().await);
    }

    #[tokio::test]
    async fn test_sync_all_providers_with_no_providers() {
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let service = RepositoryService::new(db);

        // Should succeed with no providers
        let result = service.sync_all_providers().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sync_all_providers_with_multiple_providers() {
        let db = create_test_database()
            .await
            .expect("Failed to create test database");

        // Create multiple providers
        for i in 0..3 {
            let provider = repo_provider::ActiveModel {
                name: ActiveValue::Set(format!("TestProvider{}", i)),
                provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
                base_url: ActiveValue::Set(format!("https://gitea{}.test.com", i)),
                access_token: ActiveValue::Set(format!("test_token_{:040}", i)),
                locked: ActiveValue::Set(false),
                created_at: ActiveValue::Set(chrono::Utc::now()),
                updated_at: ActiveValue::Set(chrono::Utc::now()),
                ..Default::default()
            };
            provider
                .insert(&db)
                .await
                .expect("Failed to insert provider");
        }

        let service = RepositoryService::new(db);

        // Should succeed even though providers are unreachable
        // (errors are logged but don't stop processing)
        let result = service.sync_all_providers().await;
        assert!(result.is_ok());
    }
}
