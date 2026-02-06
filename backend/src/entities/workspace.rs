//! Workspace entity (simplified MVP version)
//!
//! Minimal workspace entity that links a repository to its container.
//! In the simplified MVP, workspace is just a thin layer that manages the container lifecycle.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workspaces")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub repository_id: i32,
    pub container_id: Option<String>,
    pub workspace_status: String,
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
    #[sea_orm(has_many = "super::agent::Entity")]
    Agent,
    #[sea_orm(has_many = "super::task::Entity")]
    Task,
}

impl Related<super::repository::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Repository.def()
    }
}

impl Related<super::agent::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Agent.def()
    }
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
