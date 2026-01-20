//! Integration tests for webhook retry service
//!
//! Tests the background service that retries failed webhook creations.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use vibe_repo::entities::prelude::*;
use vibe_repo::entities::{repo_provider, repository, webhook_config};
use vibe_repo::services::{BackgroundService, WebhookRetryService};
use vibe_repo::test_utils::db::create_test_database;

/// Helper function to create test provider
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

/// Helper function to create test repository
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
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        webhook_status: Set(repository::WebhookStatus::Failed),
        ..Default::default()
    };
    repo.insert(db).await.unwrap()
}

/// Test that WebhookRetryService can be created
#[tokio::test]
async fn test_webhook_retry_service_creation() {
    let db = create_test_database().await.unwrap();
    let config = Arc::new(vibe_repo::config::AppConfig::default());

    let service = WebhookRetryService::new(db.clone(), config);

    assert_eq!(service.name(), "webhook_retry_service");
}

/// Test that service finds webhooks ready for retry
#[tokio::test]
async fn test_webhook_retry_service_finds_due_webhooks() {
    let db = create_test_database().await.unwrap();
    let config = Arc::new(vibe_repo::config::AppConfig::default());

    // Create test provider and repository
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;

    // Create webhook config with next_retry_at in the past
    let past_time = Utc::now() - chrono::Duration::seconds(60);
    let webhook = webhook_config::ActiveModel {
        provider_id: Set(provider.id),
        repository_id: Set(repo.id),
        webhook_id: Set(String::new()),
        webhook_secret: Set("test-secret".to_string()),
        webhook_url: Set("https://example.com/webhook".to_string()),
        events: Set("[]".to_string()),
        enabled: Set(false),
        retry_count: Set(1),
        last_retry_at: Set(Some(past_time)),
        next_retry_at: Set(Some(past_time)),
        last_error: Set(Some("Test error".to_string())),
        ..Default::default()
    };
    webhook.insert(&db).await.unwrap();

    // Query webhooks ready for retry
    let now = Utc::now();
    let webhooks = WebhookConfig::find()
        .filter(webhook_config::Column::NextRetryAt.lte(now))
        .filter(webhook_config::Column::RetryCount.lt(config.webhook.retry.max_retries as i32))
        .filter(webhook_config::Column::Enabled.eq(false))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(webhooks.len(), 1, "Should find one webhook ready for retry");
    assert_eq!(webhooks[0].repository_id, repo.id);
}

/// Test that service respects max retries
#[tokio::test]
async fn test_webhook_retry_service_respects_max_retries() {
    let db = create_test_database().await.unwrap();
    let config = Arc::new(vibe_repo::config::AppConfig::default());

    // Create test provider and repository
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;

    // Create webhook config with retry_count >= max_retries
    let past_time = Utc::now() - chrono::Duration::seconds(60);
    let webhook = webhook_config::ActiveModel {
        provider_id: Set(provider.id),
        repository_id: Set(repo.id),
        webhook_id: Set(String::new()),
        webhook_secret: Set("test-secret".to_string()),
        webhook_url: Set("https://example.com/webhook".to_string()),
        events: Set("[]".to_string()),
        enabled: Set(false),
        retry_count: Set(config.webhook.retry.max_retries as i32), // At max retries
        last_retry_at: Set(Some(past_time)),
        next_retry_at: Set(Some(past_time)),
        last_error: Set(Some("Test error".to_string())),
        ..Default::default()
    };
    webhook.insert(&db).await.unwrap();

    // Query webhooks ready for retry
    let now = Utc::now();
    let webhooks = WebhookConfig::find()
        .filter(webhook_config::Column::NextRetryAt.lte(now))
        .filter(webhook_config::Column::RetryCount.lt(config.webhook.retry.max_retries as i32))
        .filter(webhook_config::Column::Enabled.eq(false))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(webhooks.len(), 0, "Should not find webhooks at max retries");
}

/// Test that service only retries disabled webhooks
#[tokio::test]
async fn test_webhook_retry_service_only_retries_disabled_webhooks() {
    let db = create_test_database().await.unwrap();
    let config = Arc::new(vibe_repo::config::AppConfig::default());

    // Create test provider and repository
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;

    // Create enabled webhook config with next_retry_at in the past
    let past_time = Utc::now() - chrono::Duration::seconds(60);
    let webhook = webhook_config::ActiveModel {
        provider_id: Set(provider.id),
        repository_id: Set(repo.id),
        webhook_id: Set("webhook-123".to_string()),
        webhook_secret: Set("test-secret".to_string()),
        webhook_url: Set("https://example.com/webhook".to_string()),
        events: Set("[]".to_string()),
        enabled: Set(true), // Enabled webhook
        retry_count: Set(1),
        last_retry_at: Set(Some(past_time)),
        next_retry_at: Set(Some(past_time)),
        last_error: Set(Some("Test error".to_string())),
        ..Default::default()
    };
    webhook.insert(&db).await.unwrap();

    // Query webhooks ready for retry
    let now = Utc::now();
    let webhooks = WebhookConfig::find()
        .filter(webhook_config::Column::NextRetryAt.lte(now))
        .filter(webhook_config::Column::RetryCount.lt(config.webhook.retry.max_retries as i32))
        .filter(webhook_config::Column::Enabled.eq(false))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(webhooks.len(), 0, "Should not find enabled webhooks");
}

/// Test that service skips webhooks with future retry times
#[tokio::test]
async fn test_webhook_retry_service_skips_future_retries() {
    let db = create_test_database().await.unwrap();
    let config = Arc::new(vibe_repo::config::AppConfig::default());

    // Create test provider and repository
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;

    // Create webhook config with next_retry_at in the future
    let future_time = Utc::now() + chrono::Duration::seconds(3600);
    let webhook = webhook_config::ActiveModel {
        provider_id: Set(provider.id),
        repository_id: Set(repo.id),
        webhook_id: Set(String::new()),
        webhook_secret: Set("test-secret".to_string()),
        webhook_url: Set("https://example.com/webhook".to_string()),
        events: Set("[]".to_string()),
        enabled: Set(false),
        retry_count: Set(1),
        last_retry_at: Set(Some(Utc::now())),
        next_retry_at: Set(Some(future_time)),
        last_error: Set(Some("Test error".to_string())),
        ..Default::default()
    };
    webhook.insert(&db).await.unwrap();

    // Query webhooks ready for retry
    let now = Utc::now();
    let webhooks = WebhookConfig::find()
        .filter(webhook_config::Column::NextRetryAt.lte(now))
        .filter(webhook_config::Column::RetryCount.lt(config.webhook.retry.max_retries as i32))
        .filter(webhook_config::Column::Enabled.eq(false))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(
        webhooks.len(),
        0,
        "Should not find webhooks with future retry times"
    );
}
