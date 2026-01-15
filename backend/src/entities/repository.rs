//! Repository entity
//!
//! Represents a Git repository with validation status.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "repositories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub provider_id: i32,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub default_branch: String,
    pub branches: Json,
    pub validation_status: ValidationStatus,
    pub has_required_branches: bool,
    pub has_required_labels: bool,
    pub can_manage_prs: bool,
    pub can_manage_issues: bool,
    pub validation_message: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ValidationStatus {
    #[sea_orm(string_value = "valid")]
    Valid,
    #[sea_orm(string_value = "invalid")]
    Invalid,
    #[sea_orm(string_value = "pending")]
    Pending,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::repo_provider::Entity",
        from = "Column::ProviderId",
        to = "super::repo_provider::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    RepoProvider,
}

impl Related<super::repo_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RepoProvider.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
