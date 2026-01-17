//! WebhookConfig entity
//!
//! Represents webhook configuration for repository event monitoring.

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
