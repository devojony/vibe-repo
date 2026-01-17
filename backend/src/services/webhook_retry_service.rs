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

        tracing::info!(
            count = webhooks.len(),
            "Found webhooks ready for retry"
        );

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

    /// Retry a single webhook
    async fn retry_single_webhook(&self, webhook: &webhook_config::Model) -> Result<()> {
        tracing::info!(
            webhook_id = webhook.id,
            repository_id = webhook.repository_id,
            retry_count = webhook.retry_count,
            "Attempting webhook retry"
        );

        // Get repository and provider
        let repo = Repository::find_by_id(webhook.repository_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                crate::error::GitAutoDevError::NotFound(format!(
                    "Repository {} not found",
                    webhook.repository_id
                ))
            })?;

        let provider = RepoProvider::find_by_id(webhook.provider_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                crate::error::GitAutoDevError::NotFound(format!(
                    "Provider {} not found",
                    webhook.provider_id
                ))
            })?;

        // Create Git client
        let client = GitClientFactory::from_provider(&provider).map_err(|e| {
            crate::error::GitAutoDevError::Internal(format!("Failed to create git client: {}", e))
        })?;

        // Parse repository owner and name
        let parts: Vec<&str> = repo.full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(crate::error::GitAutoDevError::Validation(format!(
                "Invalid repository full_name format: {}",
                repo.full_name
            )));
        }
        let (owner, repo_name) = (parts[0], parts[1]);

        // Attempt to create webhook
        let webhook_request = CreateWebhookRequest {
            url: webhook.webhook_url.clone(),
            secret: webhook.webhook_secret.clone(),
            events: vec![
                WebhookEvent::IssueComment,
                WebhookEvent::PullRequestComment,
            ],
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

                Err(crate::error::GitAutoDevError::Internal(format!(
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
