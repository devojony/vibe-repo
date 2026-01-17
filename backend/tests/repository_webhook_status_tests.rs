//! Repository webhook_status field tests
//!
//! Tests for the webhook_status field in the repositories table.

use chrono::Utc;
use gitautodev::{
    entities::{repo_provider, repository},
    test_utils::db::create_test_database,
};
use sea_orm::{ActiveModelTrait, Set};

/// Test that webhook_status column exists in repositories table
#[tokio::test]
async fn test_migration_repository_webhook_status_column_exists() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    // Create a test provider and repository
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;

    // Verify webhook_status field is accessible
    assert!(
        !repo.webhook_status.is_empty(),
        "webhook_status field should exist and be accessible"
    );
}

/// Test that webhook_status defaults to 'pending' when not specified
#[tokio::test]
async fn test_migration_repository_webhook_status_default_value() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    // Create a test provider
    let provider = create_test_provider(&db).await;

    // Create a repository without explicitly setting webhook_status
    let repo = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("test-repo-default".to_string()),
        full_name: Set("org/test-repo-default".to_string()),
        clone_url: Set("https://gitea.example.com/org/test-repo-default.git".to_string()),
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
        saved.webhook_status, "pending",
        "webhook_status should default to 'pending'"
    );
}

/// Test that webhook_status can be set to different valid values
#[tokio::test]
async fn test_repository_webhook_status_valid_values() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let provider = create_test_provider(&db).await;

    // Test 'pending' status
    let repo_pending =
        create_test_repository_with_webhook_status(&db, provider.id, "pending").await;
    assert_eq!(repo_pending.webhook_status, "pending");

    // Test 'active' status
    let repo_active = create_test_repository_with_webhook_status(&db, provider.id, "active").await;
    assert_eq!(repo_active.webhook_status, "active");

    // Test 'failed' status
    let repo_failed = create_test_repository_with_webhook_status(&db, provider.id, "failed").await;
    assert_eq!(repo_failed.webhook_status, "failed");

    // Test 'disabled' status
    let repo_disabled =
        create_test_repository_with_webhook_status(&db, provider.id, "disabled").await;
    assert_eq!(repo_disabled.webhook_status, "disabled");
}

/// Test updating webhook_status
#[tokio::test]
async fn test_update_repository_webhook_status() {
    let db = create_test_database()
        .await
        .expect("Failed to setup test db");

    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;

    // Initial status should be pending
    assert_eq!(repo.webhook_status, "pending");

    // Update to active
    let mut repo_active: repository::ActiveModel = repo.into();
    repo_active.webhook_status = Set("active".to_string());
    repo_active.updated_at = Set(Utc::now());

    let updated = repo_active.update(&db).await.unwrap();
    assert_eq!(updated.webhook_status, "active");

    // Update to failed
    let mut repo_failed: repository::ActiveModel = updated.into();
    repo_failed.webhook_status = Set("failed".to_string());
    repo_failed.updated_at = Set(Utc::now());

    let updated = repo_failed.update(&db).await.unwrap();
    assert_eq!(updated.webhook_status, "failed");
}

// Helper functions

/// Create a test provider
async fn create_test_provider(db: &sea_orm::DatabaseConnection) -> repo_provider::Model {
    repo_provider::ActiveModel {
        name: Set(format!("test-provider-{}", Utc::now().timestamp_millis())),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set(format!("test-token-{}", Utc::now().timestamp_millis())),
        locked: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test provider")
}

/// Create a test repository with default webhook_status
async fn create_test_repository(
    db: &sea_orm::DatabaseConnection,
    provider_id: i32,
) -> repository::Model {
    repository::ActiveModel {
        provider_id: Set(provider_id),
        name: Set(format!("test-repo-{}", Utc::now().timestamp_millis())),
        full_name: Set(format!("org/test-repo-{}", Utc::now().timestamp_millis())),
        clone_url: Set(format!(
            "https://gitea.example.com/org/test-repo-{}.git",
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
    provider_id: i32,
    webhook_status: &str,
) -> repository::Model {
    repository::ActiveModel {
        provider_id: Set(provider_id),
        name: Set(format!(
            "test-repo-{}-{}",
            webhook_status,
            Utc::now().timestamp_millis()
        )),
        full_name: Set(format!(
            "org/test-repo-{}-{}",
            webhook_status,
            Utc::now().timestamp_millis()
        )),
        clone_url: Set(format!(
            "https://gitea.example.com/org/test-repo-{}-{}.git",
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
        webhook_status: Set(webhook_status.to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test repository")
}
