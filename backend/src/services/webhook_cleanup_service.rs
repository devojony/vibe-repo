//! Webhook Cleanup Service
//!
//! Background service that cleans up orphaned webhook configurations.
//!
//! This service runs periodically to find and remove webhook configs where
//! the webhook no longer exists on the Git provider.

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::sync::Arc;
use std::time::Duration;

use crate::config::AppConfig;
use crate::entities::prelude::*;
use crate::entities::webhook_config;
use crate::error::Result;
use crate::git_provider::{GitClientFactory, GitProvider};
use crate::services::BackgroundService;
use crate::state::AppState;

/// Webhook cleanup service
///
/// Periodically scans for orphaned webhooks and removes them from the database.
pub struct WebhookCleanupService {
    db: DatabaseConnection,
    config: Arc<AppConfig>,
}

impl WebhookCleanupService {
    /// Create a new webhook cleanup service
    pub fn new(db: DatabaseConnection, config: Arc<AppConfig>) -> Self {
        Self { db, config }
    }
    
    /// Clean up orphaned webhooks
    ///
    /// Finds webhook configs where the webhook no longer exists on the Git provider
    /// and removes them from the database.
    async fn cleanup_orphaned_webhooks(&self) -> Result<()> {
        tracing::info!("Starting orphaned webhook cleanup");
        
        // Get all webhook configs
        let webhooks = WebhookConfig::find().all(&self.db).await?;
        
        let mut cleaned = 0;
        let mut errors = 0;
        
        for webhook in webhooks {
            match self.verify_webhook_exists(&webhook).await {
                Ok(exists) => {
                    if !exists {
                        tracing::info!(
                            webhook_id = webhook.id,
                            repository_id = webhook.repository_id,
                            "Found orphaned webhook, deleting from database"
                        );
                        
                        if let Err(e) = webhook_config::Entity::delete_by_id(webhook.id)
                            .exec(&self.db)
                            .await
                        {
                            tracing::error!(
                                webhook_id = webhook.id,
                                error = %e,
                                "Failed to delete orphaned webhook"
                            );
                            errors += 1;
                        } else {
                            cleaned += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        webhook_id = webhook.id,
                        error = %e,
                        "Failed to verify webhook existence"
                    );
                    errors += 1;
                }
            }
        }
        
        tracing::info!(
            cleaned = cleaned,
            errors = errors,
            "Orphaned webhook cleanup complete"
        );
        
        Ok(())
    }
    
    /// Verify if webhook exists on Git provider
    ///
    /// # Arguments
    /// * `webhook` - The webhook config model
    ///
    /// # Returns
    /// Ok(true) if webhook exists, Ok(false) if orphaned, Err on verification failure
    async fn verify_webhook_exists(&self, webhook: &webhook_config::Model) -> Result<bool> {
        // Get repository
        let repo = Repository::find_by_id(webhook.repository_id)
            .one(&self.db)
            .await?;
        
        if repo.is_none() {
            // Repository doesn't exist, webhook is orphaned
            return Ok(false);
        }
        let repo = repo.unwrap();
        
        // Get provider
        let provider = RepoProvider::find_by_id(webhook.provider_id)
            .one(&self.db)
            .await?;
        
        if provider.is_none() {
            // Provider doesn't exist, webhook is orphaned
            return Ok(false);
        }
        let provider = provider.unwrap();
        
        // Create Git client
        let client = GitClientFactory::from_provider(&provider).map_err(|e| {
            crate::error::GitAutoDevError::Internal(format!("Failed to create git client: {}", e))
        })?;
        
        // Parse repository owner and name
        let parts: Vec<&str> = repo.full_name.split('/').collect();
        if parts.len() != 2 {
            return Ok(false);
        }
        let (owner, repo_name) = (parts[0], parts[1]);
        
        // List webhooks from Git provider
        match client.list_webhooks(owner, repo_name).await {
            Ok(webhooks) => {
                // Check if our webhook ID exists in the list
                Ok(webhooks.iter().any(|w| w.id == webhook.webhook_id))
            }
            Err(e) => {
                tracing::warn!(
                    webhook_id = webhook.id,
                    error = %e,
                    "Failed to list webhooks from Git provider"
                );
                // Assume webhook exists if we can't verify (avoid false positives)
                Ok(true)
            }
        }
    }
}

#[async_trait]
impl BackgroundService for WebhookCleanupService {
    fn name(&self) -> &'static str {
        "webhook_cleanup_service"
    }
    
    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        tracing::info!("Starting webhook cleanup service");
        
        let db = self.db.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let service = WebhookCleanupService::new(db, config);
            let mut interval = tokio::time::interval(Duration::from_secs(86400)); // 24 hours
            
            loop {
                interval.tick().await;
                
                if let Err(e) = service.cleanup_orphaned_webhooks().await {
                    tracing::error!(error = %e, "Error in webhook cleanup service");
                }
            }
        });
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        tracing::info!("Webhook cleanup service stopped");
        Ok(())
    }
    
    async fn health_check(&self) -> bool {
        // Check database connection
        self.db.ping().await.is_ok()
    }
}
