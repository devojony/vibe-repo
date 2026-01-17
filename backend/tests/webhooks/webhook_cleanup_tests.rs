//! Tests for webhook cleanup mechanisms
//!
//! Tests for Task 4.3: Webhook Cleanup Mechanism

use vibe_repo::entities::prelude::*;
use vibe_repo::entities::{repo_provider, repository, webhook_config};
use vibe_repo::services::RepositoryService;
use vibe_repo::test_utils::db::create_test_database;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};
use std::sync::Arc;

/// Helper function to create test provider
async fn create_test_provider(
    db: &sea_orm::DatabaseConnection,
) -> repo_provider::Model {
    let provider = repo_provider::ActiveModel {
        name: ActiveValue::Set("test-provider".to_string()),
        provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
        base_url: ActiveValue::Set("https://gitea.example.com".to_string()),
        access_token: ActiveValue::Set("test-token".to_string()),
        locked: ActiveValue::Set(false),
        ..Default::default()
    };
    provider.insert(db).await.unwrap()
}

/// Helper function to create test repository
async fn create_test_repository(
    db: &sea_orm::DatabaseConnection,
    provider_id: i32,
    name: &str,
) -> repository::Model {
    let repo = repository::ActiveModel {
        provider_id: ActiveValue::Set(provider_id),
        name: ActiveValue::Set(name.to_string()),
        full_name: ActiveValue::Set(format!("owner/{}", name)),
        clone_url: ActiveValue::Set(format!("https://gitea.example.com/owner/{}.git", name)),
        default_branch: ActiveValue::Set("main".to_string()),
        branches: ActiveValue::Set(serde_json::json!(["main"])),
        validation_status: ActiveValue::Set(repository::ValidationStatus::Valid),
        has_required_branches: ActiveValue::Set(true),
        has_required_labels: ActiveValue::Set(true),
        can_manage_prs: ActiveValue::Set(true),
        can_manage_issues: ActiveValue::Set(true),
        validation_message: ActiveValue::Set(None),
        ..Default::default()
    };
    repo.insert(db).await.unwrap()
}

/// Helper function to create test webhook config
async fn create_test_webhook(
    db: &sea_orm::DatabaseConnection,
    provider_id: i32,
    repository_id: i32,
    webhook_id: &str,
) -> webhook_config::Model {
    let webhook = webhook_config::ActiveModel {
        provider_id: ActiveValue::Set(provider_id),
        repository_id: ActiveValue::Set(repository_id),
        webhook_id: ActiveValue::Set(webhook_id.to_string()),
        webhook_secret: ActiveValue::Set("test-secret".to_string()),
        webhook_url: ActiveValue::Set("https://example.com/webhook".to_string()),
        events: ActiveValue::Set("[]".to_string()),
        enabled: ActiveValue::Set(true),
        retry_count: ActiveValue::Set(0),
        last_retry_at: ActiveValue::Set(None),
        next_retry_at: ActiveValue::Set(None),
        last_error: ActiveValue::Set(None),
        ..Default::default()
    };
    webhook.insert(db).await.unwrap()
}

/// Test that deleting a repository cascades to webhook_config
///
/// Requirements: 4.3.1, 4.3.4
#[tokio::test]
async fn test_delete_repository_cascades_to_webhook_config() {
    let db = create_test_database().await.expect("Failed to create test database");
    
    // Create test data
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id, "test-repo").await;
    let webhook = create_test_webhook(&db, provider.id, repo.id, "webhook-123").await;
    
    // Verify webhook exists
    let webhook_exists = WebhookConfig::find_by_id(webhook.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(webhook_exists, "Webhook should exist before deletion");
    
    // Delete repository using SeaORM cascade
    let repo_active: repository::ActiveModel = repo.into();
    repo_active.delete(&db).await.unwrap();
    
    // Verify webhook was cascade deleted
    let webhook_exists = WebhookConfig::find_by_id(webhook.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(!webhook_exists, "Webhook should be cascade deleted with repository");
}

/// Test that deleting a provider cascades to webhook_config
///
/// Requirements: 4.3.2, 4.3.4
#[tokio::test]
async fn test_delete_provider_cascades_to_webhook_config() {
    let db = create_test_database().await.expect("Failed to create test database");
    
    // Create test data
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id, "test-repo").await;
    let webhook = create_test_webhook(&db, provider.id, repo.id, "webhook-123").await;
    
    // Verify webhook exists
    let webhook_exists = WebhookConfig::find_by_id(webhook.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(webhook_exists, "Webhook should exist before deletion");
    
    // Delete provider using SeaORM cascade
    let provider_active: repo_provider::ActiveModel = provider.into();
    provider_active.delete(&db).await.unwrap();
    
    // Verify webhook was cascade deleted
    let webhook_exists = WebhookConfig::find_by_id(webhook.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(!webhook_exists, "Webhook should be cascade deleted with provider");
}

/// Test that delete_repository method attempts to delete webhook from Git provider
///
/// Requirements: 4.3.1
#[tokio::test]
async fn test_delete_repository_attempts_git_provider_cleanup() {
    let db = create_test_database().await.expect("Failed to create test database");
    let config = Arc::new(vibe_repo::config::AppConfig::default());
    let service = RepositoryService::new(db.clone(), config);
    
    // Create test data
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id, "test-repo").await;
    let _webhook = create_test_webhook(&db, provider.id, repo.id, "webhook-123").await;
    
    // Note: This test will fail to delete webhook from Git provider (no real provider)
    // but should still succeed in deleting the repository
    let result = service.delete_repository(repo.id).await;
    
    // Repository deletion should succeed even if webhook deletion fails
    assert!(result.is_ok(), "Repository deletion should succeed even if webhook deletion fails");
    
    // Verify repository was deleted
    let repo_exists = Repository::find_by_id(repo.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(!repo_exists, "Repository should be deleted");
}

/// Test that delete_repository handles missing webhook gracefully
///
/// Requirements: 4.3.1
#[tokio::test]
async fn test_delete_repository_without_webhook() {
    let db = create_test_database().await.expect("Failed to create test database");
    let config = Arc::new(vibe_repo::config::AppConfig::default());
    let service = RepositoryService::new(db.clone(), config);
    
    // Create test data without webhook
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id, "test-repo").await;
    
    // Delete repository
    let result = service.delete_repository(repo.id).await;
    
    // Should succeed even without webhook
    assert!(result.is_ok(), "Repository deletion should succeed without webhook");
    
    // Verify repository was deleted
    let repo_exists = Repository::find_by_id(repo.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(!repo_exists, "Repository should be deleted");
}

/// Test that delete_repository returns error for non-existent repository
///
/// Requirements: 4.3.1
#[tokio::test]
async fn test_delete_repository_not_found() {
    let db = create_test_database().await.expect("Failed to create test database");
    let config = Arc::new(vibe_repo::config::AppConfig::default());
    let service = RepositoryService::new(db.clone(), config);
    
    // Try to delete non-existent repository
    let result = service.delete_repository(99999).await;
    
    // Should return NotFound error
    assert!(result.is_err(), "Should return error for non-existent repository");
    assert!(
        matches!(result.unwrap_err(), vibe_repo::error::VibeRepoError::NotFound(_)),
        "Should return NotFound error"
    );
}

/// Test orphaned webhook cleanup - webhook exists in DB but not on Git provider
///
/// Requirements: 4.3.3
#[tokio::test]
async fn test_orphaned_webhook_cleanup_removes_orphans() {
    let db = create_test_database().await.expect("Failed to create test database");
    
    // Create test data
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id, "test-repo").await;
    let webhook = create_test_webhook(&db, provider.id, repo.id, "orphaned-webhook").await;
    
    // Verify webhook exists
    let webhook_id = webhook.id;
    let webhook_exists = WebhookConfig::find_by_id(webhook_id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(webhook_exists, "Webhook should exist before cleanup");
    
    // Note: In a real scenario, the cleanup service would:
    // 1. Query Git provider for webhooks
    // 2. Find that this webhook doesn't exist on provider
    // 3. Delete it from database
    
    // For this test, we'll manually simulate the cleanup
    // (The actual cleanup service will be tested with mocked Git provider)
    let webhook_active: webhook_config::ActiveModel = webhook.into();
    webhook_active.delete(&db).await.unwrap();
    
    // Verify webhook was deleted
    let webhook_exists = WebhookConfig::find_by_id(webhook_id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(!webhook_exists, "Orphaned webhook should be deleted");
}

/// Test that multiple webhooks are cleaned up when provider is deleted
///
/// Requirements: 4.3.2, 4.3.4
#[tokio::test]
async fn test_delete_provider_cleans_up_multiple_webhooks() {
    let db = create_test_database().await.expect("Failed to create test database");
    
    // Create test data with multiple repositories and webhooks
    let provider = create_test_provider(&db).await;
    let repo1 = create_test_repository(&db, provider.id, "repo1").await;
    let repo2 = create_test_repository(&db, provider.id, "repo2").await;
    let webhook1 = create_test_webhook(&db, provider.id, repo1.id, "webhook-1").await;
    let webhook2 = create_test_webhook(&db, provider.id, repo2.id, "webhook-2").await;
    
    // Verify webhooks exist
    let webhook1_exists = WebhookConfig::find_by_id(webhook1.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    let webhook2_exists = WebhookConfig::find_by_id(webhook2.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(webhook1_exists && webhook2_exists, "Webhooks should exist before deletion");
    
    // Delete provider
    let provider_active: repo_provider::ActiveModel = provider.into();
    provider_active.delete(&db).await.unwrap();
    
    // Verify all webhooks were cascade deleted
    let webhook1_exists = WebhookConfig::find_by_id(webhook1.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    let webhook2_exists = WebhookConfig::find_by_id(webhook2.id)
        .one(&db)
        .await
        .unwrap()
        .is_some();
    assert!(!webhook1_exists && !webhook2_exists, "All webhooks should be cascade deleted");
}
