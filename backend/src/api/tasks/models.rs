use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response model for task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub issue_number: i32,
    pub issue_title: String,
    pub issue_body: Option<String>,
    pub task_status: String,
    pub priority: String,
    pub assigned_agent_id: Option<i32>,
    pub branch_name: Option<String>,
    pub pr_number: Option<i32>,
    pub pr_url: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Request model for creating task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaskRequest {
    pub workspace_id: i32,
    pub issue_number: i32,
    pub issue_title: String,
    pub issue_body: Option<String>,
    pub assigned_agent_id: Option<i32>,
    #[serde(default = "default_priority")]
    pub priority: String,
}

fn default_priority() -> String {
    "medium".to_string()
}

/// Request model for updating task status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTaskStatusRequest {
    pub status: String,
}

impl From<crate::entities::task::Model> for TaskResponse {
    fn from(model: crate::entities::task::Model) -> Self {
        Self {
            id: model.id,
            workspace_id: model.workspace_id,
            issue_number: model.issue_number,
            issue_title: model.issue_title,
            issue_body: model.issue_body,
            task_status: model.task_status,
            priority: model.priority,
            assigned_agent_id: model.assigned_agent_id,
            branch_name: model.branch_name,
            pr_number: model.pr_number,
            pr_url: model.pr_url,
            error_message: model.error_message,
            retry_count: model.retry_count,
            max_retries: model.max_retries,
            started_at: model.started_at.map(|dt| dt.to_string()),
            completed_at: model.completed_at.map(|dt| dt.to_string()),
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
            deleted_at: model.deleted_at.map(|dt| dt.to_string()),
        }
    }
}
