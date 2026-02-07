//! Repository entity
//!
//! Represents a Git repository with validation status and lifecycle management.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "repositories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub provider_type: String,
    pub provider_base_url: String,
    pub access_token: String,
    pub webhook_secret: Option<String>,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub default_branch: String,
    pub branches: Json,
    pub validation_status: ValidationStatus,
    pub status: RepositoryStatus,
    pub has_workspace: bool,
    pub has_required_branches: bool,
    pub has_required_labels: bool,
    pub can_manage_prs: bool,
    pub can_manage_issues: bool,
    pub validation_message: Option<String>,
    pub webhook_status: WebhookStatus,
    #[sea_orm(column_type = "Text", nullable)]
    pub agent_command: Option<String>,
    pub agent_timeout: i32,
    pub agent_env_vars: Option<Json>,
    pub docker_image: String,
    pub deleted_at: Option<DateTimeUtc>,
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

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum RepositoryStatus {
    #[sea_orm(string_value = "uninitialized")]
    Uninitialized,
    #[sea_orm(string_value = "idle")]
    Idle,
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "unavailable")]
    Unavailable,
    #[sea_orm(string_value = "archived")]
    Archived,
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum WebhookStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "disabled")]
    Disabled,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Check if the repository is soft-deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Check if the repository can be deleted
    /// A repository can only be deleted if it doesn't have a workspace
    pub fn can_delete(&self) -> bool {
        !self.has_workspace
    }

    /// Check if the repository can be archived
    /// A repository can only be archived if it doesn't have a workspace
    pub fn can_archive(&self) -> bool {
        !self.has_workspace && self.status != RepositoryStatus::Archived
    }

    /// Check if the repository can create a workspace
    pub fn can_create_workspace(&self) -> bool {
        self.status == RepositoryStatus::Idle
            && self.validation_status == ValidationStatus::Valid
            && !self.has_workspace
    }
}
