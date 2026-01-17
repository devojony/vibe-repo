//! WebhookConfig entity
//!
//! Represents webhook configuration for repository event monitoring.
//!
//! ## Relationships
//!
//! **Primary Association**: webhook_config → repository (one-to-one)
//! - Each repository has at most one webhook configuration
//! - Webhook URL format: `/api/webhooks/{repository_id}`
//! - The repository_id in the URL enables direct lookup without database queries
//!
//! **Secondary Association**: webhook_config → provider (many-to-one, redundant)
//! - provider_id is redundant (can be obtained via repository.provider_id)
//! - Kept for performance optimization:
//!   - Enables cascade delete when provider is removed
//!   - Allows fast queries: "get all webhooks for provider X"
//!   - Avoids JOIN when getting provider_type for signature verification
//!
//! ## Design Rationale
//!
//! Webhooks are per-repository in Git providers (Gitea/GitHub/GitLab), not per-provider.
//! The URL uses repository_id to make this association explicit and enable direct lookup.
//! While provider_id is technically redundant, it provides significant performance benefits
//! for common operations like cascade deletion and provider-level queries.
//!
//! ## Example Webhook URL
//!
//! ```text
//! https://vibe-repo.example.com/api/webhooks/42
//! ```
//!
//! Where `42` is the repository_id, allowing the handler to:
//! 1. Look up webhook_config by repository_id (indexed, fast)
//! 2. Verify webhook signature using webhook_secret
//! 3. Process events for that specific repository

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub provider_id: i32,
    pub repository_id: i32,
    pub webhook_id: String,
    pub webhook_secret: String,
    pub webhook_url: String,
    pub events: String, // JSON array of event types
    pub enabled: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    // Retry fields
    pub retry_count: i32,
    pub last_retry_at: Option<DateTimeUtc>,
    pub next_retry_at: Option<DateTimeUtc>,
    pub last_error: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::repo_provider::Entity",
        from = "Column::ProviderId",
        to = "super::repo_provider::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    RepoProvider,
    #[sea_orm(
        belongs_to = "super::repository::Entity",
        from = "Column::RepositoryId",
        to = "super::repository::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Repository,
}

impl Related<super::repo_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RepoProvider.def()
    }
}

impl Related<super::repository::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Repository.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
