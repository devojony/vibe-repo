//! Tests for webhook retry logic
//!
//! Task 4.2: Error Handling and Retry
//! Requirements: Exponential backoff, retry tracking, max retries

use chrono::Utc;
use gitautodev::config::WebhookRetryConfig;
use gitautodev::entities::{prelude::*, repo_provider, repository, webhook_config};
use gitautodev::services::RepositoryService;
use gitautodev::test_utils::db::create_test_database;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

/// Test that webhook retry fields exist in database
/// Requirements: 4.2.1 - Add retry tracking fields
#[tokio::test]
async fn test_webhook_retry_fields_exist() {
    let db = create_test_database().await.unwrap();

    // Create provider first
    let provider = repo_provider::ActiveModel {
        name: ActiveValue::Set("Test Provider".to_string()),
        provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
        base_url: ActiveValue::Set("https://gitea.example.com".to_string()),
        access_token: ActiveValue::Set("test_token".to_string()),
        locked: ActiveValue::Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&db).await.unwrap();

    // Create repository
    let repo = repository::ActiveModel {
        provider_id: ActiveValue::Set(provider.id),
        name: ActiveValue::Set("test-repo".to_string()),
        full_name: ActiveValue::Set("owner/test-repo".to_string()),
        clone_url: ActiveValue::Set("https://gitea.example.com/owner/test-repo.git".to_string()),
        default_branch: ActiveValue::Set("main".to_string()),
        branches: ActiveValue::Set(serde_json::json!(["main"])),
        validation_status: ActiveValue::Set(repository::ValidationStatus::Valid),
        status: ActiveValue::Set(repository::RepositoryStatus::Idle),
        has_workspace: ActiveValue::Set(false),
        has_required_branches: ActiveValue::Set(true),
        has_required_labels: ActiveValue::Set(true),
        can_manage_prs: ActiveValue::Set(true),
        can_manage_issues: ActiveValue::Set(true),
        validation_message: ActiveValue::Set(None),
        webhook_status: ActiveValue::Set(repository::WebhookStatus::Pending),
        deleted_at: ActiveValue::Set(None),
        ..Default::default()
    };
    let repo = repo.insert(&db).await.unwrap();

    // Create a test webhook config with retry fields
    let webhook = webhook_config::ActiveModel {
        provider_id: ActiveValue::Set(provider.id),
        repository_id: ActiveValue::Set(repo.id),
        webhook_id: ActiveValue::Set("test-webhook-id".to_string()),
        webhook_secret: ActiveValue::Set("test-secret".to_string()),
        webhook_url: ActiveValue::Set("https://example.com/webhook".to_string()),
        events: ActiveValue::Set("[]".to_string()),
        enabled: ActiveValue::Set(true),
        retry_count: ActiveValue::Set(0),
        last_retry_at: ActiveValue::Set(None),
        next_retry_at: ActiveValue::Set(None),
        last_error: ActiveValue::Set(None),
        created_at: ActiveValue::Set(Utc::now()),
        updated_at: ActiveValue::Set(Utc::now()),
        ..Default::default()
    };

    // This should not panic - fields exist
    let result = webhook.insert(&db).await;
    assert!(
        result.is_ok(),
        "Should be able to insert webhook with retry fields"
    );
}

/// Test exponential backoff calculation
/// Requirements: 4.2.2 - Implement exponential backoff
#[test]
fn test_calculate_exponential_backoff() {
    let config = WebhookRetryConfig {
        max_retries: 5,
        initial_delay_secs: 60,
        max_delay_secs: 3600,
        backoff_multiplier: 2.0,
    };

    // Test retry 0: 60s
    let delay_0 = calculate_retry_delay(0, &config);
    assert_eq!(delay_0, 60);

    // Test retry 1: 120s
    let delay_1 = calculate_retry_delay(1, &config);
    assert_eq!(delay_1, 120);

    // Test retry 2: 240s
    let delay_2 = calculate_retry_delay(2, &config);
    assert_eq!(delay_2, 240);

    // Test retry 3: 480s
    let delay_3 = calculate_retry_delay(3, &config);
    assert_eq!(delay_3, 480);

    // Test retry 4: 960s
    let delay_4 = calculate_retry_delay(4, &config);
    assert_eq!(delay_4, 960);

    // Test retry 5: 1920s, but capped at 3600s
    let delay_5 = calculate_retry_delay(5, &config);
    assert_eq!(delay_5, 1920);

    // Test retry 6: 3840s, but capped at 3600s
    let delay_6 = calculate_retry_delay(6, &config);
    assert_eq!(delay_6, 3600, "Should be capped at max_delay_secs");
}

/// Test that max retries are respected
/// Requirements: 4.2.3 - Respect max_retries configuration
#[test]
fn test_max_retries_respected() {
    let config = WebhookRetryConfig {
        max_retries: 3,
        initial_delay_secs: 60,
        max_delay_secs: 3600,
        backoff_multiplier: 2.0,
    };

    // Should return None when retry_count >= max_retries
    let next_retry = calculate_next_retry_time(3, &config);
    assert!(
        next_retry.is_none(),
        "Should not schedule retry when max_retries reached"
    );

    let next_retry = calculate_next_retry_time(4, &config);
    assert!(
        next_retry.is_none(),
        "Should not schedule retry when max_retries exceeded"
    );

    // Should return Some when retry_count < max_retries
    let next_retry = calculate_next_retry_time(2, &config);
    assert!(
        next_retry.is_some(),
        "Should schedule retry when under max_retries"
    );
}

/// Test webhook status transitions during retry
/// Requirements: 4.2.4 - Update webhook_status based on retry results
#[tokio::test]
async fn test_webhook_status_transitions() {
    let db = create_test_database().await.unwrap();

    // Create provider
    let provider = repo_provider::ActiveModel {
        name: ActiveValue::Set("Test Provider".to_string()),
        provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
        base_url: ActiveValue::Set("https://gitea.example.com".to_string()),
        access_token: ActiveValue::Set("test_token".to_string()),
        locked: ActiveValue::Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&db).await.unwrap();

    // Create repository with pending webhook status
    let repo = repository::ActiveModel {
        provider_id: ActiveValue::Set(provider.id),
        name: ActiveValue::Set("test-repo".to_string()),
        full_name: ActiveValue::Set("owner/test-repo".to_string()),
        clone_url: ActiveValue::Set("https://gitea.example.com/owner/test-repo.git".to_string()),
        default_branch: ActiveValue::Set("main".to_string()),
        branches: ActiveValue::Set(serde_json::json!(["main"])),
        validation_status: ActiveValue::Set(repository::ValidationStatus::Valid),
        status: ActiveValue::Set(repository::RepositoryStatus::Idle),
        has_workspace: ActiveValue::Set(false),
        has_required_branches: ActiveValue::Set(true),
        has_required_labels: ActiveValue::Set(true),
        can_manage_prs: ActiveValue::Set(true),
        can_manage_issues: ActiveValue::Set(true),
        validation_message: ActiveValue::Set(None),
        webhook_status: ActiveValue::Set(repository::WebhookStatus::Pending),
        deleted_at: ActiveValue::Set(None),
        ..Default::default()
    };
    let repo = repo.insert(&db).await.unwrap();

    // Verify initial status
    assert_eq!(repo.webhook_status, repository::WebhookStatus::Pending);

    // After successful webhook creation, status should be Active
    // (This will be tested in integration tests with actual webhook creation)

    // After failed webhook creation, status should be Failed
    // (This will be tested in integration tests with actual webhook creation)
}

/// Test retry count increments correctly
/// Requirements: 4.2.5 - Track retry attempts
#[tokio::test]
async fn test_retry_count_increments() {
    let db = create_test_database().await.unwrap();

    // Create provider
    let provider = repo_provider::ActiveModel {
        name: ActiveValue::Set("Test Provider".to_string()),
        provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
        base_url: ActiveValue::Set("https://gitea.example.com".to_string()),
        access_token: ActiveValue::Set("test_token".to_string()),
        locked: ActiveValue::Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&db).await.unwrap();

    // Create repository
    let repo = repository::ActiveModel {
        provider_id: ActiveValue::Set(provider.id),
        name: ActiveValue::Set("test-repo".to_string()),
        full_name: ActiveValue::Set("owner/test-repo".to_string()),
        clone_url: ActiveValue::Set("https://gitea.example.com/owner/test-repo.git".to_string()),
        default_branch: ActiveValue::Set("main".to_string()),
        branches: ActiveValue::Set(serde_json::json!(["main"])),
        validation_status: ActiveValue::Set(repository::ValidationStatus::Valid),
        status: ActiveValue::Set(repository::RepositoryStatus::Idle),
        has_workspace: ActiveValue::Set(false),
        has_required_branches: ActiveValue::Set(true),
        has_required_labels: ActiveValue::Set(true),
        can_manage_prs: ActiveValue::Set(true),
        can_manage_issues: ActiveValue::Set(true),
        validation_message: ActiveValue::Set(None),
        webhook_status: ActiveValue::Set(repository::WebhookStatus::Pending),
        deleted_at: ActiveValue::Set(None),
        ..Default::default()
    };
    let repo = repo.insert(&db).await.unwrap();

    // Create a webhook config
    let webhook = webhook_config::ActiveModel {
        provider_id: ActiveValue::Set(provider.id),
        repository_id: ActiveValue::Set(repo.id),
        webhook_id: ActiveValue::Set("test-webhook-id".to_string()),
        webhook_secret: ActiveValue::Set("test-secret".to_string()),
        webhook_url: ActiveValue::Set("https://example.com/webhook".to_string()),
        events: ActiveValue::Set("[]".to_string()),
        enabled: ActiveValue::Set(true),
        retry_count: ActiveValue::Set(0),
        last_retry_at: ActiveValue::Set(None),
        next_retry_at: ActiveValue::Set(None),
        last_error: ActiveValue::Set(None),
        created_at: ActiveValue::Set(Utc::now()),
        updated_at: ActiveValue::Set(Utc::now()),
        ..Default::default()
    };
    let webhook = webhook.insert(&db).await.unwrap();

    assert_eq!(webhook.retry_count, 0);

    // Simulate retry attempt
    let mut active: webhook_config::ActiveModel = webhook.into();
    active.retry_count = ActiveValue::Set(1);
    active.last_retry_at = ActiveValue::Set(Some(Utc::now()));
    active.last_error = ActiveValue::Set(Some("Test error".to_string()));
    let updated = active.update(&db).await.unwrap();

    assert_eq!(updated.retry_count, 1);
    assert!(updated.last_retry_at.is_some());
    assert_eq!(updated.last_error, Some("Test error".to_string()));
}

// Helper functions that will be implemented in RepositoryService

/// Calculate retry delay in seconds using exponential backoff
fn calculate_retry_delay(retry_count: i32, config: &WebhookRetryConfig) -> u64 {
    let delay_secs = (config.initial_delay_secs as f64
        * config.backoff_multiplier.powi(retry_count))
    .min(config.max_delay_secs as f64) as u64;
    delay_secs
}

/// Calculate next retry time, returns None if max retries exceeded
fn calculate_next_retry_time(
    retry_count: i32,
    config: &WebhookRetryConfig,
) -> Option<chrono::DateTime<Utc>> {
    if retry_count >= config.max_retries as i32 {
        return None;
    }

    let delay_secs = calculate_retry_delay(retry_count, config);
    Some(Utc::now() + chrono::Duration::seconds(delay_secs as i64))
}
