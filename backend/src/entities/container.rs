//! Container Entity
//!
//! Represents Docker container instances for workspaces.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "containers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub workspace_id: i32,
    #[sea_orm(unique)]
    pub container_id: String,
    pub container_name: String,
    pub image_name: String,
    pub image_id: Option<String>,
    pub status: String,
    pub health_status: Option<String>,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub restart_count: i32,
    pub max_restart_attempts: i32,
    pub last_restart_at: Option<DateTimeUtc>,
    pub last_health_check: Option<DateTimeUtc>,
    pub health_check_failures: i32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub started_at: Option<DateTimeUtc>,
    pub stopped_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workspace::Entity",
        from = "Column::WorkspaceId",
        to = "super::workspace::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Workspace,
}

impl Related<super::workspace::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workspace.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// Container status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    Creating,
    Running,
    Stopped,
    Exited,
    Failed,
}

impl ContainerStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ContainerStatus::Creating => "creating",
            ContainerStatus::Running => "running",
            ContainerStatus::Stopped => "stopped",
            ContainerStatus::Exited => "exited",
            ContainerStatus::Failed => "failed",
        }
    }
}

impl FromStr for ContainerStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "creating" => Ok(ContainerStatus::Creating),
            "running" => Ok(ContainerStatus::Running),
            "stopped" => Ok(ContainerStatus::Stopped),
            "exited" => Ok(ContainerStatus::Exited),
            "failed" => Ok(ContainerStatus::Failed),
            _ => Err(format!("Invalid container status: {}", s)),
        }
    }
}

/// Container health status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &str {
        match self {
            HealthStatus::Healthy => "Healthy",
            HealthStatus::Unhealthy => "Unhealthy",
            HealthStatus::Unknown => "Unknown",
        }
    }
}

impl FromStr for HealthStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Healthy" => Ok(HealthStatus::Healthy),
            "Unhealthy" => Ok(HealthStatus::Unhealthy),
            "Unknown" => Ok(HealthStatus::Unknown),
            _ => Err(format!("Invalid health status: {}", s)),
        }
    }
}
