//! Webhook Config entity (simplified MVP version)
//!
//! Minimal webhook configuration stored in database.
//! In simplified MVP, most webhook config comes from env vars,
//! but we still store the webhook_id and secret per repository.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub repository_id: i32,
    pub webhook_id: String,
    pub webhook_secret: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::repository::Entity",
        from = "Column::RepositoryId",
        to = "super::repository::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Repository,
}

impl Related<super::repository::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Repository.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
