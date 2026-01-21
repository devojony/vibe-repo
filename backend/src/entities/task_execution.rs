//! `SeaORM` Entity for task_executions table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "task_executions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub task_id: i32,
    pub agent_id: Option<i32>,
    pub status: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub stdout_file_path: Option<String>,
    pub stderr_file_path: Option<String>,
    pub error_message: Option<String>,
    pub pr_number: Option<i32>,
    pub pr_url: Option<String>,
    pub branch_name: Option<String>,
    pub duration_ms: Option<i64>,
    pub started_at: DateTimeUtc,
    pub completed_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::task::Entity",
        from = "Column::TaskId",
        to = "super::task::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Task,
    #[sea_orm(
        belongs_to = "super::agent::Entity",
        from = "Column::AgentId",
        to = "super::agent::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Agent,
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl Related<super::agent::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Agent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
