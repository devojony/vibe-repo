use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response model for workspace
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceResponse {
    pub id: i32,
    pub repository_id: i32,
    pub workspace_status: String,
    pub container_id: Option<String>,
    pub container_status: Option<String>,
    pub image_source: String,
    pub custom_dockerfile_path: Option<String>,
    pub max_concurrent_tasks: i32,
    pub cpu_limit: f64,
    pub memory_limit: String,
    pub disk_limit: String,
    pub work_dir: Option<String>,
    pub health_status: Option<String>,
    pub last_health_check: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Request model for creating workspace
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceRequest {
    pub repository_id: i32,
    #[serde(default = "default_image_source")]
    pub image_source: String,
    pub custom_dockerfile_path: Option<String>,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: i32,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_memory_limit")]
    pub memory_limit: String,
    #[serde(default = "default_disk_limit")]
    pub disk_limit: String,
}

fn default_image_source() -> String {
    "default".to_string()
}

fn default_max_concurrent_tasks() -> i32 {
    3
}

fn default_cpu_limit() -> f64 {
    2.0
}

fn default_memory_limit() -> String {
    "4GB".to_string()
}

fn default_disk_limit() -> String {
    "10GB".to_string()
}

/// Request model for updating workspace status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkspaceStatusRequest {
    pub status: String,
}

impl From<crate::entities::workspace::Model> for WorkspaceResponse {
    fn from(model: crate::entities::workspace::Model) -> Self {
        Self {
            id: model.id,
            repository_id: model.repository_id,
            workspace_status: model.workspace_status,
            container_id: model.container_id,
            container_status: model.container_status,
            image_source: model.image_source,
            custom_dockerfile_path: model.custom_dockerfile_path,
            max_concurrent_tasks: model.max_concurrent_tasks,
            cpu_limit: model.cpu_limit,
            memory_limit: model.memory_limit,
            disk_limit: model.disk_limit,
            work_dir: model.work_dir,
            health_status: model.health_status,
            last_health_check: model.last_health_check.map(|dt| dt.to_string()),
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
            deleted_at: model.deleted_at.map(|dt| dt.to_string()),
        }
    }
}
