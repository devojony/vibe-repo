use crate::entities::task::TaskStatus;
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
    #[serde(default = "default_priority")]
    pub priority: String,
}

fn default_priority() -> String {
    "medium".to_string()
}

/// Request model for updating task status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTaskStatusRequest {
    pub status: TaskStatus,
}

/// Request model for updating task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTaskRequest {
    pub priority: Option<String>,
}

/// Request model for assigning agent
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignAgentRequest {
    pub agent_id: Option<i32>,
}

/// Request model for completing task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompleteTaskRequest {
    pub pr_number: i32,
    pub pr_url: String,
    pub branch_name: String,
}

/// Request model for failing task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FailTaskRequest {
    pub error_message: String,
}

/// Response model for task list with pagination
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

/// Response model for task plans
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskPlansResponse {
    pub plans: serde_json::Value,
}

/// Response model for task events
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskEventsResponse {
    pub events: serde_json::Value,
}

/// Response model for task progress
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskProgressResponse {
    /// Progress percentage (0.0 to 1.0)
    pub progress: f32,
    /// Number of completed steps
    pub completed_steps: usize,
    /// Total number of steps
    pub total_steps: usize,
}

/// Response model for task status with progress
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskStatusResponse {
    pub task_id: i32,
    pub status: String,
    pub progress: Option<f32>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::entities::task::Model> for TaskResponse {
    fn from(model: crate::entities::task::Model) -> Self {
        Self {
            id: model.id,
            workspace_id: model.workspace_id,
            issue_number: model.issue_number,
            issue_title: model.issue_title,
            issue_body: model.issue_body,
            task_status: model.task_status.to_string(),
            priority: model.priority,
            assigned_agent_id: model.assigned_agent_id,
            branch_name: model.branch_name,
            pr_number: model.pr_number,
            pr_url: model.pr_url,
            error_message: model.error_message,
            started_at: model.started_at.map(|dt| dt.to_string()),
            completed_at: model.completed_at.map(|dt| dt.to_string()),
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
            deleted_at: model.deleted_at.map(|dt| dt.to_string()),
        }
    }
}
