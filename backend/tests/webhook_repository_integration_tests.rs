//! Integration tests for webhook creation during repository initialization
//!
//! Note: These tests verify error handling behavior when Git provider is unavailable.
//! Full webhook creation testing requires mocking the Git provider (future work).

use gitautodev::config::AppConfig;
use gitautodev::entities::prelude::*;
use gitautodev::entities::{repo_provider, repository, webhook_config};
use gitautodev::services::RepositoryService;
use gitautodev::test_utils::db::create_test_database;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

/// Helper: Create test provider
async fn create_test_provider(db: &sea_orm::DatabaseConnection) -> repo_provider::Model {
    let provider = repo_provider::ActiveModel {
        name: Set("test-provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test-token".to_string()),
        locked: Set(false),
        ..Default::default()
    };
    provider.insert(db).await.unwrap()
}

/// Helper: Create test repository
async fn create_test_repository(
    db: &sea_orm::DatabaseConnection,
    provider_id: i32,
) -> repository::Model {
    let repo = repository::ActiveModel {
        provider_id: Set(provider_id),
        name: Set("test-repo".to_string()),
        full_name: Set("owner/test-repo".to_string()),
        clone_url: Set("https://gitea.example.com/owner/test-repo.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        webhook_status: Set(repository::WebhookStatus::Pending),
        ..Default::default()
    };
    repo.insert(db).await.unwrap()
}

/// Test that initialization fails gracefully when Git provider is unreachable
/// 
/// Without a mocked Git provider, the initialization will fail at the branch creation step.
/// This test verifies that the error is handled properly and returns ServiceUnavailable.
#[tokio::test]
async fn test_initialize_repository_fails_when_git_provider_unreachable() {
    // Arrange
    let db = create_test_database().await.unwrap();
    let service = RepositoryService::new(db.clone());
    let config = AppConfig::default();
    
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    
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
    assert!(result.is_err(), "Initialization should fail when Git provider is unreachable");
    
    match result {
        Err(gitautodev::error::GitAutoDevError::ServiceUnavailable(msg)) => {
            assert_eq!(msg, "Git provider unreachable", "Error message should indicate Git provider is unreachable");
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
#[tokio::test]
async fn test_initialize_repository_without_webhook_config_fails_gracefully() {
    // Arrange
    let db = create_test_database().await.unwrap();
    let service = RepositoryService::new(db.clone());
    
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    
    // Act - Initialize without webhook config (None, None)
    let result = service
        .initialize_repository(repo.id, "vibe-dev", None, None)
        .await;
    
    // Assert - Should fail with ServiceUnavailable error
    assert!(result.is_err(), "Initialization should fail when Git provider is unreachable");
    
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
    
    // Verify no webhook was created in database
    let webhooks = WebhookConfig::find()
        .filter(webhook_config::Column::RepositoryId.eq(repo.id))
        .all(&db)
        .await
        .unwrap();
    
    assert_eq!(webhooks.len(), 0, "No webhook should be created when config is not provided");
}

/// Test that webhook_status field exists and can be queried
/// 
/// This test verifies the database schema includes the webhook_status field
/// and that it can be properly queried and updated.
#[tokio::test]
async fn test_webhook_status_field_exists_and_queryable() {
    // Arrange
    let db = create_test_database().await.unwrap();
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    
    // Act - Query the repository
    let fetched_repo = Repository::find_by_id(repo.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    
    // Assert - webhook_status should be Pending (default)
    assert_eq!(
        fetched_repo.webhook_status,
        repository::WebhookStatus::Pending,
        "New repository should have Pending webhook status"
    );
    
    // Act - Update webhook_status to Active
    let mut active_repo: repository::ActiveModel = fetched_repo.into();
    active_repo.webhook_status = Set(repository::WebhookStatus::Active);
    let updated_repo = active_repo.update(&db).await.unwrap();
    
    // Assert - webhook_status should be Active
    assert_eq!(
        updated_repo.webhook_status,
        repository::WebhookStatus::Active,
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
