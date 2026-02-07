//! Repository webhook_status field tests
//!
//! Tests for the webhook_status field in the repositories table.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use vibe_repo::{entities::repository, test_utils::db::create_test_database};

/// Test that webhook_status column exists in repositories table
#[tokio::test]
async fn test_migration_repository_webhook_status_column_exists() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    // Create a test repository
    let repo = create_test_repository(&db).await;

    // Verify webhook_status field is accessible
    assert_eq!(
        repo.webhook_status,
        repository::WebhookStatus::Pending,
        "webhook_status field should exist and be accessible"
    );
}

/// Test that webhook_status defaults to 'pending' when not specified
#[tokio::test]
async fn test_migration_repository_webhook_status_default_value() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    // Create a repository without explicitly setting webhook_status
    let repo = repository::ActiveModel {
        name: Set("test-repo-default".to_string()),
        full_name: Set("org/test-repo-default".to_string()),
        provider_type: Set("github".to_string()),
        provider_base_url: Set("https://api.github.com".to_string()),
        access_token: Set("test_token".to_string()),
        webhook_secret: Set(Some("test_secret".to_string())),
        clone_url: Set("https://api.github.com/org/test-repo-default.git".to_string()),
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
        // Don't set webhook_status - should default to 'pending'
        ..Default::default()
    };

    let saved = repo.insert(&db).await.unwrap();
    assert_eq!(
        saved.webhook_status,
        repository::WebhookStatus::Pending,
        "webhook_status should default to 'pending'"
    );
}

/// Test that webhook_status can be set to different valid values
#[tokio::test]
async fn test_repository_webhook_status_valid_values() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    // Test 'pending' status
    let repo_pending =
        create_test_repository_with_webhook_status(&db, repository::WebhookStatus::Pending).await;
    assert_eq!(
        repo_pending.webhook_status,
        repository::WebhookStatus::Pending
    );

    // Test 'active' status
    let repo_active =
        create_test_repository_with_webhook_status(&db, repository::WebhookStatus::Active).await;
    assert_eq!(
        repo_active.webhook_status,
        repository::WebhookStatus::Active
    );

    // Test 'failed' status
    let repo_failed =
        create_test_repository_with_webhook_status(&db, repository::WebhookStatus::Failed).await;
    assert_eq!(
        repo_failed.webhook_status,
        repository::WebhookStatus::Failed
    );

    // Test 'disabled' status
    let repo_disabled =
        create_test_repository_with_webhook_status(&db, repository::WebhookStatus::Disabled).await;
    assert_eq!(
        repo_disabled.webhook_status,
        repository::WebhookStatus::Disabled
    );
}

/// Test updating webhook_status
#[tokio::test]
async fn test_update_repository_webhook_status() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let repo = create_test_repository(&db).await;

    // Initial status should be pending
    assert_eq!(repo.webhook_status, repository::WebhookStatus::Pending);

    // Update to active
    let mut repo_active: repository::ActiveModel = repo.into();
    repo_active.webhook_status = Set(repository::WebhookStatus::Active);
    repo_active.updated_at = Set(Utc::now());

    let updated = repo_active.update(&db).await.unwrap();
    assert_eq!(updated.webhook_status, repository::WebhookStatus::Active);

    // Update to failed
    let mut repo_failed: repository::ActiveModel = updated.into();
    repo_failed.webhook_status = Set(repository::WebhookStatus::Failed);
    repo_failed.updated_at = Set(Utc::now());

    let updated = repo_failed.update(&db).await.unwrap();
    assert_eq!(updated.webhook_status, repository::WebhookStatus::Failed);
}

// Helper functions

/// Create a test repository with default webhook_status
async fn create_test_repository(db: &sea_orm::DatabaseConnection) -> repository::Model {
    repository::ActiveModel {
        name: Set(format!("test-repo-{}", Utc::now().timestamp_millis())),
        full_name: Set(format!("org/test-repo-{}", Utc::now().timestamp_millis())),
        provider_type: Set("github".to_string()),
        provider_base_url: Set("https://api.github.com".to_string()),
        access_token: Set(format!("test-token-{}", Utc::now().timestamp_millis())),
        webhook_secret: Set(Some(format!(
            "test-secret-{}",
            Utc::now().timestamp_millis()
        ))),
        clone_url: Set(format!(
            "https://api.github.com/org/test-repo-{}.git",
            Utc::now().timestamp_millis()
        )),
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

/// Create a test repository with specific webhook_status
async fn create_test_repository_with_webhook_status(
    db: &sea_orm::DatabaseConnection,
    webhook_status: repository::WebhookStatus,
) -> repository::Model {
    repository::ActiveModel {
        name: Set(format!(
            "test-repo-{:?}-{}",
            webhook_status,
            Utc::now().timestamp_millis()
        )),
        full_name: Set(format!(
            "org/test-repo-{:?}-{}",
            webhook_status,
            Utc::now().timestamp_millis()
        )),
        provider_type: Set("github".to_string()),
        provider_base_url: Set("https://api.github.com".to_string()),
        access_token: Set(format!(
            "test-token-{:?}-{}",
            webhook_status,
            Utc::now().timestamp_millis()
        )),
        webhook_secret: Set(Some(format!(
            "test-secret-{:?}-{}",
            webhook_status,
            Utc::now().timestamp_millis()
        ))),
        clone_url: Set(format!(
            "https://api.github.com/org/test-repo-{:?}-{}.git",
            webhook_status,
            Utc::now().timestamp_millis()
        )),
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
        webhook_status: Set(webhook_status),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test repository")
}
