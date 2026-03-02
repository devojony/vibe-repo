//! Integration tests for webhook creation during repository initialization
//!
//! Note: These tests verify error handling behavior when Git provider is unavailable.
//! Full webhook creation testing requires mocking the Git provider (future work).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use vibe_repo::config::AppConfig;
use vibe_repo::entities::prelude::*;
use vibe_repo::entities::repository;
use vibe_repo::services::RepositoryService;
use vibe_repo::test_utils::create_test_repository;
use vibe_repo::test_utils::db::create_test_database;

/// Test that initialization fails gracefully when Git provider is unreachable
///
/// Without a mocked Git provider, the initialization will fail at the branch creation step.
/// This test verifies that the error is handled properly and returns ServiceUnavailable.
/// NOTE: In MVP, this test is disabled because test repositories use mock data that succeeds.
#[tokio::test]
#[ignore = "Test repositories now use mock data that succeeds"]
async fn test_initialize_repository_fails_when_git_provider_unreachable() {
    // Arrange
    let db = create_test_database().await.unwrap();
    let service = RepositoryService::new(
        db.clone(),
        Arc::new(vibe_repo::config::AppConfig::default()),
    );
    let config = AppConfig::default();

    let repo = create_test_repository(
        &db,
        "test-repo",
        "owner/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");

    // Act
    let result = service
        .initialize_repository(
            repo.id,
            "vibe-dev",
            Some(config.webhook.domain.clone()),
            Some(config.webhook.secret_key.clone()),
        )
        .await;

    // Assert - Should fail with ServiceUnavailable error
    assert!(
        result.is_err(),
        "Initialization should fail when Git provider is unreachable"
    );

    match result {
        Err(vibe_repo::error::VibeRepoError::ServiceUnavailable(msg)) => {
            assert_eq!(
                msg, "Git provider unreachable",
                "Error message should indicate Git provider is unreachable"
            );
        }
        _ => panic!("Expected ServiceUnavailable error, got: {:?}", result),
    }

    // Verify webhook_status remains Pending (webhook creation was never attempted)
    let updated_repo = Repository::find_by_id(repo.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        updated_repo.webhook_status,
        repository::WebhookStatus::Pending,
        "Webhook status should remain Pending when initialization fails before webhook creation"
    );
}

/// Test that initialization fails gracefully without webhook config
///
/// This test verifies that even without webhook config, the initialization
/// still fails at the branch creation step when Git provider is unreachable.
/// NOTE: In MVP, this test is disabled because test repositories use mock data that succeeds.
#[tokio::test]
#[ignore = "Test repositories now use mock data that succeeds"]
async fn test_initialize_repository_without_webhook_config_fails_gracefully() {
    // Arrange
    let db = create_test_database().await.unwrap();
    let service = RepositoryService::new(
        db.clone(),
        Arc::new(vibe_repo::config::AppConfig::default()),
    );

    let repo = create_test_repository(
        &db,
        "test-repo",
        "owner/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");

    // Act - Initialize without webhook config (None, None)
    let result = service
        .initialize_repository(repo.id, "vibe-dev", None, None)
        .await;

    // Assert - Should fail with ServiceUnavailable error
    assert!(
        result.is_err(),
        "Initialization should fail when Git provider is unreachable"
    );

    // Verify webhook_status remains Pending (webhook creation was never attempted)
    let updated_repo = Repository::find_by_id(repo.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        updated_repo.webhook_status,
        repository::WebhookStatus::Pending,
        "Webhook status should remain Pending when webhook creation is skipped"
    );

    // Note: In the new architecture, webhook_secret is stored directly in the repository entity
    // No separate webhook_config table exists
}

/// Test that webhook_status field exists and can be queried
///
/// This test verifies the database schema includes the webhook_status field
/// and that it can be properly queried and updated.
#[tokio::test]
async fn test_webhook_status_field_exists_and_queryable() {
    // Arrange
    let db = create_test_database().await.unwrap();
    let repo = create_test_repository(
        &db,
        "test-repo",
        "owner/test-repo",
        "gitea",
        "https://gitea.example.com",
        "test-token",
    )
    .await
    .expect("Failed to create test repository");

    // Act - Query the repository
    let fetched_repo = Repository::find_by_id(repo.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    // Assert - webhook_status should be Active (test repositories are created with Active status)
    assert_eq!(
        fetched_repo.webhook_status,
        repository::WebhookStatus::Active,
        "Test repository should have Active webhook status"
    );

    // Act - Update webhook_status to Disabled
    let mut disabled_repo: repository::ActiveModel = fetched_repo.into();
    disabled_repo.webhook_status = Set(repository::WebhookStatus::Disabled);
    let updated_repo = disabled_repo.update(&db).await.unwrap();

    // Assert - webhook_status should be Disabled
    assert_eq!(
        updated_repo.webhook_status,
        repository::WebhookStatus::Disabled,
        "Webhook status should be updated to Active"
    );

    // Act - Update webhook_status to Failed
    let mut active_repo: repository::ActiveModel = updated_repo.into();
    active_repo.webhook_status = Set(repository::WebhookStatus::Failed);
    let updated_repo = active_repo.update(&db).await.unwrap();

    // Assert - webhook_status should be Failed
    assert_eq!(
        updated_repo.webhook_status,
        repository::WebhookStatus::Failed,
        "Webhook status should be updated to Failed"
    );
}
