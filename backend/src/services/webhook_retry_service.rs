//! Webhook Retry Service
//!
//! Background service that periodically retries failed webhook creations.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::sync::Arc;
use std::time::Duration;

use crate::config::AppConfig;
use crate::entities::prelude::*;
use crate::entities::{repository, webhook_config};
use crate::error::Result;
use crate::git_provider::traits::GitProvider;
use crate::git_provider::{CreateWebhookRequest, GitClientFactory, WebhookEvent};
use crate::services::{BackgroundService, RepositoryService};
use crate::state::AppState;

pub struct WebhookRetryService {
    db: DatabaseConnection,
    config: Arc<AppConfig>,
}

impl WebhookRetryService {
    pub fn new(db: DatabaseConnection, config: Arc<AppConfig>) -> Self {
        Self { db, config }
    }

    /// Retry failed webhooks that are due for retry
    async fn retry_failed_webhooks(&self) -> Result<()> {
        let now = Utc::now();

        // Find webhooks ready for retry
        let webhooks = WebhookConfig::find()
            .filter(webhook_config::Column::NextRetryAt.lte(now))
            .filter(
                webhook_config::Column::RetryCount.lt(self.config.webhook.retry.max_retries as i32),
            )
            .filter(webhook_config::Column::Enabled.eq(false)) // Only retry disabled webhooks
            .all(&self.db)
            .await?;

        tracing::info!(count = webhooks.len(), "Found webhooks ready for retry");

        for webhook in webhooks {
            if let Err(e) = self.retry_single_webhook(&webhook).await {
                tracing::error!(
                    webhook_id = webhook.id,
                    repository_id = webhook.repository_id,
                    error = %e,
                    "Failed to retry webhook"
                );
            }
        }

        Ok(())
    }

    /// Check if polling should be enabled as a fallback for failed webhooks
    async fn check_and_enable_polling(&self, webhook: &webhook_config::Model) -> Result<()> {
        // Check if retry count exceeds the fallback threshold
        if webhook.retry_count >= self.config.webhook.retry.polling_fallback_threshold as i32 {
            let repo = Repository::find_by_id(webhook.repository_id)
                .one(&self.db)
                .await?
                .ok_or_else(|| {
                    crate::error::VibeRepoError::NotFound(format!(
                        "Repository {} not found",
                        webhook.repository_id
                    ))
                })?;

            // Only enable polling if it's not already enabled
            if !repo.polling_enabled {
                tracing::warn!(
                    repository_id = repo.id,
                    webhook_retry_count = webhook.retry_count,
                    fallback_threshold = self.config.webhook.retry.polling_fallback_threshold,
                    "Webhook failed multiple times, enabling polling as fallback"
                );

                let mut repo_active: repository::ActiveModel = repo.into();
                repo_active.polling_enabled = ActiveValue::Set(true);
                repo_active.updated_at = ActiveValue::Set(Utc::now());
                repo_active.update(&self.db).await?;

                tracing::info!(
                    repository_id = webhook.repository_id,
                    "Polling enabled as fallback for failed webhook"
                );
            }
        }

        Ok(())
    }

    /// Retry a single webhook
    async fn retry_single_webhook(&self, webhook: &webhook_config::Model) -> Result<()> {
        tracing::info!(
            webhook_id = webhook.id,
            repository_id = webhook.repository_id,
            retry_count = webhook.retry_count,
            "Attempting webhook retry"
        );

        // Check if we should enable polling as a fallback
        if let Err(e) = self.check_and_enable_polling(webhook).await {
            tracing::error!(
                webhook_id = webhook.id,
                error = %e,
                "Failed to enable polling fallback"
            );
        }

        // Get repository and provider
        let repo = Repository::find_by_id(webhook.repository_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                crate::error::VibeRepoError::NotFound(format!(
                    "Repository {} not found",
                    webhook.repository_id
                ))
            })?;

        let provider = RepoProvider::find_by_id(webhook.provider_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                crate::error::VibeRepoError::NotFound(format!(
                    "Provider {} not found",
                    webhook.provider_id
                ))
            })?;

        // Create Git client
        let client = GitClientFactory::from_provider(&provider).map_err(|e| {
            crate::error::VibeRepoError::Internal(format!("Failed to create git client: {}", e))
        })?;

        // Parse repository owner and name
        let parts: Vec<&str> = repo.full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(crate::error::VibeRepoError::Validation(format!(
                "Invalid repository full_name format: {}",
                repo.full_name
            )));
        }
        let (owner, repo_name) = (parts[0], parts[1]);

        // Attempt to create webhook
        // Note: Uses the webhook URL stored in the database (format: /api/webhooks/{repository_id})
        // The URL is generated during webhook creation in RepositoryService
        let webhook_request = CreateWebhookRequest {
            url: webhook.webhook_url.clone(),
            secret: webhook.webhook_secret.clone(),
            events: vec![WebhookEvent::IssueComment, WebhookEvent::PullRequestComment],
            active: true,
        };

        match client
            .create_webhook(owner, repo_name, webhook_request)
            .await
        {
            Ok(git_webhook) => {
                // Success! Update webhook config
                let mut webhook_active: webhook_config::ActiveModel = webhook.clone().into();
                webhook_active.webhook_id = ActiveValue::Set(git_webhook.id.clone());
                webhook_active.enabled = ActiveValue::Set(true);
                webhook_active.retry_count = ActiveValue::Set(0); // Reset retry count
                webhook_active.last_retry_at = ActiveValue::Set(Some(Utc::now()));
                webhook_active.next_retry_at = ActiveValue::Set(None); // Clear next retry
                webhook_active.last_error = ActiveValue::Set(None); // Clear error
                webhook_active.updated_at = ActiveValue::Set(Utc::now());
                webhook_active.update(&self.db).await?;

                // Update repository webhook_status to Active
                let mut repo_active: repository::ActiveModel = repo.into();
                repo_active.webhook_status = ActiveValue::Set(repository::WebhookStatus::Active);
                repo_active.updated_at = ActiveValue::Set(Utc::now());
                repo_active.update(&self.db).await?;

                tracing::info!(
                    webhook_id = webhook.id,
                    repository_id = webhook.repository_id,
                    git_webhook_id = %git_webhook.id,
                    "Webhook retry successful"
                );

                Ok(())
            }
            Err(e) => {
                // Failure - record for next retry
                let retry_count = webhook.retry_count + 1;
                let error_message = e.to_string();
                let next_retry = if retry_count < self.config.webhook.retry.max_retries as i32 {
                    Some(RepositoryService::calculate_next_retry_time(
                        retry_count,
                        &self.config.webhook.retry,
                    ))
                } else {
                    None // Max retries exceeded
                };

                let mut webhook_active: webhook_config::ActiveModel = webhook.clone().into();
                webhook_active.retry_count = ActiveValue::Set(retry_count);
                webhook_active.last_retry_at = ActiveValue::Set(Some(Utc::now()));
                webhook_active.next_retry_at = ActiveValue::Set(next_retry.flatten());
                webhook_active.last_error = ActiveValue::Set(Some(error_message.clone()));
                webhook_active.updated_at = ActiveValue::Set(Utc::now());
                webhook_active.update(&self.db).await?;

                tracing::warn!(
                    webhook_id = webhook.id,
                    repository_id = webhook.repository_id,
                    retry_count = retry_count,
                    next_retry_at = ?next_retry,
                    error = %error_message,
                    "Webhook retry failed"
                );

                Err(crate::error::VibeRepoError::Internal(format!(
                    "Webhook retry failed: {}",
                    error_message
                )))
            }
        }
    }
}

#[async_trait]
impl BackgroundService for WebhookRetryService {
    fn name(&self) -> &'static str {
        "webhook_retry_service"
    }

    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        tracing::info!("Starting webhook retry service");

        let db = self.db.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let service = WebhookRetryService::new(db, config);
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            loop {
                interval.tick().await;

                if let Err(e) = service.retry_failed_webhooks().await {
                    tracing::error!(error = %e, "Error in webhook retry service");
                }
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("Webhook retry service stopped");
        Ok(())
    }

    async fn health_check(&self) -> bool {
        // Check database connection
        self.db.ping().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, WebhookRetryConfig};
    use crate::entities::{repo_provider, repository, webhook_config};
    use crate::test_utils::db::TestDatabase;
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    /// Helper function to create a test webhook retry config
    fn create_test_retry_config(fallback_threshold: u32) -> WebhookRetryConfig {
        WebhookRetryConfig {
            max_retries: 10,
            initial_delay_secs: 60,
            max_delay_secs: 3600,
            backoff_multiplier: 2.0,
            polling_fallback_threshold: fallback_threshold,
        }
    }

    /// Test that polling is enabled when retry count exceeds threshold
    #[tokio::test]
    async fn test_check_and_enable_polling_when_threshold_exceeded() {
        // Arrange: Create test database and service
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let mut config = AppConfig::default();
        config.webhook.retry = create_test_retry_config(5);
        let service = WebhookRetryService::new(db.clone(), Arc::new(config));

        // Create a test provider
        let provider = repo_provider::ActiveModel {
            name: Set("test-provider".to_string()),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://gitea.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = provider.insert(db).await.unwrap();

        // Create a test repository with polling disabled
        let repo = repository::ActiveModel {
            provider_id: Set(provider.id),
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://gitea.example.com/owner/test-repo.git".to_string()),
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
            webhook_status: Set(repository::WebhookStatus::Failed),
            polling_enabled: Set(false), // Initially disabled
            polling_interval_seconds: Set(Some(300)), // Default 5 minutes
            last_issue_poll_at: Set(None),
            deleted_at: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        let repo = repo.insert(db).await.unwrap();

        // Create a webhook config with retry count exceeding threshold
        let webhook = webhook_config::ActiveModel {
            provider_id: Set(provider.id),
            repository_id: Set(repo.id),
            webhook_id: Set("webhook-123".to_string()),
            webhook_secret: Set("secret".to_string()),
            webhook_url: Set(format!("/api/webhooks/{}", repo.id)),
            events: Set("[]".to_string()),
            enabled: Set(false),
            retry_count: Set(6), // Exceeds threshold of 5
            last_retry_at: Set(Some(Utc::now())),
            next_retry_at: Set(None),
            last_error: Set(Some("Connection failed".to_string())),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        let webhook = webhook.insert(db).await.unwrap();

        // Act: Check and enable polling
        let result = service.check_and_enable_polling(&webhook).await;

        // Assert: Polling should be enabled
        assert!(result.is_ok(), "check_and_enable_polling should succeed");

        let updated_repo = Repository::find_by_id(repo.id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        assert!(
            updated_repo.polling_enabled,
            "Polling should be enabled after threshold exceeded"
        );
    }

    /// Test that polling is NOT enabled when retry count is below threshold
    #[tokio::test]
    async fn test_check_and_enable_polling_when_below_threshold() {
        // Arrange: Create test database and service
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let mut config = AppConfig::default();
        config.webhook.retry = create_test_retry_config(5);
        let service = WebhookRetryService::new(db.clone(), Arc::new(config));

        // Create a test provider
        let provider = repo_provider::ActiveModel {
            name: Set("test-provider".to_string()),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://gitea.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = provider.insert(db).await.unwrap();

        // Create a test repository with polling disabled
        let repo = repository::ActiveModel {
            provider_id: Set(provider.id),
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://gitea.example.com/owner/test-repo.git".to_string()),
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
            webhook_status: Set(repository::WebhookStatus::Failed),
            polling_enabled: Set(false), // Initially disabled
            polling_interval_seconds: Set(Some(300)), // Default 5 minutes
            last_issue_poll_at: Set(None),
            deleted_at: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        let repo = repo.insert(db).await.unwrap();

        // Create a webhook config with retry count below threshold
        let webhook = webhook_config::ActiveModel {
            provider_id: Set(provider.id),
            repository_id: Set(repo.id),
            webhook_id: Set("webhook-123".to_string()),
            webhook_secret: Set("secret".to_string()),
            webhook_url: Set(format!("/api/webhooks/{}", repo.id)),
            events: Set("[]".to_string()),
            enabled: Set(false),
            retry_count: Set(3), // Below threshold of 5
            last_retry_at: Set(Some(Utc::now())),
            next_retry_at: Set(None),
            last_error: Set(Some("Connection failed".to_string())),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        let webhook = webhook.insert(db).await.unwrap();

        // Act: Check and enable polling
        let result = service.check_and_enable_polling(&webhook).await;

        // Assert: Polling should NOT be enabled
        assert!(result.is_ok(), "check_and_enable_polling should succeed");

        let updated_repo = Repository::find_by_id(repo.id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        assert!(
            !updated_repo.polling_enabled,
            "Polling should NOT be enabled when below threshold"
        );
    }

    /// Test that polling is not re-enabled if already enabled
    #[tokio::test]
    async fn test_check_and_enable_polling_when_already_enabled() {
        // Arrange: Create test database and service
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let mut config = AppConfig::default();
        config.webhook.retry = create_test_retry_config(5);
        let service = WebhookRetryService::new(db.clone(), Arc::new(config));

        // Create a test provider
        let provider = repo_provider::ActiveModel {
            name: Set("test-provider".to_string()),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://gitea.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = provider.insert(db).await.unwrap();

        // Create a test repository with polling already enabled
        let repo = repository::ActiveModel {
            provider_id: Set(provider.id),
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://gitea.example.com/owner/test-repo.git".to_string()),
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
            webhook_status: Set(repository::WebhookStatus::Failed),
            polling_enabled: Set(true), // Already enabled
            polling_interval_seconds: Set(Some(300)), // Default 5 minutes
            last_issue_poll_at: Set(None),
            deleted_at: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        let repo = repo.insert(db).await.unwrap();
        let original_updated_at = repo.updated_at;

        // Create a webhook config with retry count exceeding threshold
        let webhook = webhook_config::ActiveModel {
            provider_id: Set(provider.id),
            repository_id: Set(repo.id),
            webhook_id: Set("webhook-123".to_string()),
            webhook_secret: Set("secret".to_string()),
            webhook_url: Set(format!("/api/webhooks/{}", repo.id)),
            events: Set("[]".to_string()),
            enabled: Set(false),
            retry_count: Set(6), // Exceeds threshold of 5
            last_retry_at: Set(Some(Utc::now())),
            next_retry_at: Set(None),
            last_error: Set(Some("Connection failed".to_string())),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        let webhook = webhook.insert(db).await.unwrap();

        // Act: Check and enable polling
        let result = service.check_and_enable_polling(&webhook).await;

        // Assert: Polling should remain enabled, but updated_at should not change
        assert!(result.is_ok(), "check_and_enable_polling should succeed");

        let updated_repo = Repository::find_by_id(repo.id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        assert!(
            updated_repo.polling_enabled,
            "Polling should remain enabled"
        );
        assert_eq!(
            updated_repo.updated_at, original_updated_at,
            "updated_at should not change when polling is already enabled"
        );
    }
}
