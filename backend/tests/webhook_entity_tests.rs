//! WebhookConfig entity tests
//!
//! Tests for the webhook_config entity model and its relationships.

use chrono::Utc;
use gitautodev::{
    entities::{prelude::*, repo_provider, repository, webhook_config},
    test_utils::db::create_test_database,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

/// Test creating a webhook config
#[tokio::test]
async fn test_create_webhook_config() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    // Create test provider and repository
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;

    let webhook = webhook_config::ActiveModel {
        provider_id: Set(provider.id),
        repository_id: Set(repo.id),
        webhook_id: Set("123".to_string()),
        webhook_secret: Set("secret123".to_string()),
        webhook_url: Set("https://example.com/webhook/1".to_string()),
        events: Set(r#"["issue_comment","pull_request_comment"]"#.to_string()),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };

    let result = webhook.insert(&db).await;
    assert!(result.is_ok());

    let saved = result.unwrap();
    assert_eq!(saved.provider_id, provider.id);
    assert_eq!(saved.repository_id, repo.id);
    assert_eq!(saved.webhook_id, "123");
    assert_eq!(saved.webhook_secret, "secret123");
    assert_eq!(
        saved.webhook_url,
        "https://example.com/webhook/1".to_string()
    );
    assert_eq!(
        saved.events,
        r#"["issue_comment","pull_request_comment"]"#.to_string()
    );
    assert!(saved.enabled);
}

/// Test webhook config cascade delete when provider is deleted
#[tokio::test]
async fn test_webhook_config_cascade_delete_with_provider() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    let webhook = create_test_webhook(&db, provider.id, repo.id).await;

    // Delete provider should cascade delete webhook
    RepoProvider::delete_by_id(provider.id)
        .exec(&db)
        .await
        .unwrap();

    let found = WebhookConfig::find_by_id(webhook.id)
        .one(&db)
        .await
        .unwrap();

    assert!(
        found.is_none(),
        "Webhook should be deleted when provider is deleted"
    );
}

/// Test webhook config cascade delete when repository is deleted
#[tokio::test]
async fn test_webhook_config_cascade_delete_with_repository() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    let webhook = create_test_webhook(&db, provider.id, repo.id).await;

    // Delete repository should cascade delete webhook
    Repository::delete_by_id(repo.id).exec(&db).await.unwrap();

    let found = WebhookConfig::find_by_id(webhook.id)
        .one(&db)
        .await
        .unwrap();

    assert!(
        found.is_none(),
        "Webhook should be deleted when repository is deleted"
    );
}

/// Test querying webhook by provider_id
#[tokio::test]
async fn test_find_webhook_by_provider() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    let webhook = create_test_webhook(&db, provider.id, repo.id).await;

    // Find webhooks by provider
    let webhooks = WebhookConfig::find()
        .filter(webhook_config::Column::ProviderId.eq(provider.id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(webhooks.len(), 1);
    assert_eq!(webhooks[0].id, webhook.id);
}

/// Test querying webhook by repository_id
#[tokio::test]
async fn test_find_webhook_by_repository() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    let webhook = create_test_webhook(&db, provider.id, repo.id).await;

    // Find webhooks by repository
    let webhooks = WebhookConfig::find()
        .filter(webhook_config::Column::RepositoryId.eq(repo.id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(webhooks.len(), 1);
    assert_eq!(webhooks[0].id, webhook.id);
}

/// Test updating webhook config
#[tokio::test]
async fn test_update_webhook_config() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    let webhook = create_test_webhook(&db, provider.id, repo.id).await;

    // Update webhook
    let mut webhook_active: webhook_config::ActiveModel = webhook.into();
    webhook_active.enabled = Set(false);
    webhook_active.updated_at = Set(Utc::now());

    let updated = webhook_active.update(&db).await.unwrap();
    assert!(!updated.enabled);
}

// Helper functions

/// Create a test provider
async fn create_test_provider(db: &sea_orm::DatabaseConnection) -> repo_provider::Model {
    repo_provider::ActiveModel {
        name: Set("test-provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test-token".to_string()),
        locked: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test provider")
}

/// Create a test repository
async fn create_test_repository(
    db: &sea_orm::DatabaseConnection,
    provider_id: i32,
) -> repository::Model {
    repository::ActiveModel {
        provider_id: Set(provider_id),
        name: Set("test-repo".to_string()),
        full_name: Set("org/test-repo".to_string()),
        clone_url: Set("https://gitea.example.com/org/test-repo.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Idle),
        has_workspace: Set(false),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        deleted_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test repository")
}

/// Create a test webhook config
async fn create_test_webhook(
    db: &sea_orm::DatabaseConnection,
    provider_id: i32,
    repo_id: i32,
) -> webhook_config::Model {
    webhook_config::ActiveModel {
        provider_id: Set(provider_id),
        repository_id: Set(repo_id),
        webhook_id: Set("test-webhook-123".to_string()),
        webhook_secret: Set("test-secret".to_string()),
        webhook_url: Set("https://example.com/webhook".to_string()),
        events: Set(r#"["issue_comment"]"#.to_string()),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test webhook")
}
