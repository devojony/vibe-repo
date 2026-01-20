use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::entities::{init_script, workspace};

/// Response model for workspace
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceResponse {
    pub id: i32,
    pub repository_id: i32,
    pub workspace_status: String,
    pub container_id: Option<String>,
    pub container_status: Option<String>,
    pub image_source: String,
    pub init_script: Option<InitScriptResponse>,
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
    pub init_script: Option<String>,
    #[serde(default = "default_script_timeout")]
    pub script_timeout_seconds: i32,
    #[serde(default = "default_image_source")]
    pub image_source: String,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: i32,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_memory_limit")]
    pub memory_limit: String,
    #[serde(default = "default_disk_limit")]
    pub disk_limit: String,
}

fn default_script_timeout() -> i32 {
    300
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

/// Response model for init script
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InitScriptResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub script_content: String,
    pub timeout_seconds: i32,
    pub status: String,
    pub output_summary: Option<String>,
    pub has_full_log: bool,
    pub executed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<init_script::Model> for InitScriptResponse {
    fn from(model: init_script::Model) -> Self {
        Self {
            id: model.id,
            workspace_id: model.workspace_id,
            script_content: model.script_content,
            timeout_seconds: model.timeout_seconds,
            status: model.status,
            output_summary: model.output_summary,
            has_full_log: model.output_file_path.is_some(),
            executed_at: model.executed_at.map(|dt| dt.to_string()),
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
        }
    }
}

/// Request model for updating init script
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateInitScriptRequest {
    pub script_content: String,
    #[serde(default = "default_script_timeout")]
    pub timeout_seconds: i32,
    #[serde(default)]
    pub execute_immediately: bool,
}

/// Request model for executing init script
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecuteScriptRequest {
    #[serde(default)]
    pub force: bool,
}

/// Response model for init script logs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InitScriptLogsResponse {
    pub status: String,
    pub output_summary: Option<String>,
    pub has_full_log: bool,
    pub executed_at: Option<String>,
}

// Backward compatibility: From implementation for workspace only
impl From<workspace::Model> for WorkspaceResponse {
    fn from(workspace: workspace::Model) -> Self {
        Self {
            id: workspace.id,
            repository_id: workspace.repository_id,
            workspace_status: workspace.workspace_status,
            container_id: workspace.container_id,
            container_status: workspace.container_status,
            image_source: workspace.image_source,
            init_script: None,
            max_concurrent_tasks: workspace.max_concurrent_tasks,
            cpu_limit: workspace.cpu_limit,
            memory_limit: workspace.memory_limit,
            disk_limit: workspace.disk_limit,
            work_dir: workspace.work_dir,
            health_status: workspace.health_status,
            last_health_check: workspace.last_health_check.map(|dt| dt.to_string()),
            created_at: workspace.created_at.to_string(),
            updated_at: workspace.updated_at.to_string(),
            deleted_at: workspace.deleted_at.map(|dt| dt.to_string()),
        }
    }
}

// New From implementation with init_script support
impl From<(workspace::Model, Option<init_script::Model>)> for WorkspaceResponse {
    fn from((workspace, init_script): (workspace::Model, Option<init_script::Model>)) -> Self {
        Self {
            id: workspace.id,
            repository_id: workspace.repository_id,
            workspace_status: workspace.workspace_status,
            container_id: workspace.container_id,
            container_status: workspace.container_status,
            image_source: workspace.image_source,
            init_script: init_script.map(InitScriptResponse::from),
            max_concurrent_tasks: workspace.max_concurrent_tasks,
            cpu_limit: workspace.cpu_limit,
            memory_limit: workspace.memory_limit,
            disk_limit: workspace.disk_limit,
            work_dir: workspace.work_dir,
            health_status: workspace.health_status,
            last_health_check: workspace.last_health_check.map(|dt| dt.to_string()),
            created_at: workspace.created_at.to_string(),
            updated_at: workspace.updated_at.to_string(),
            deleted_at: workspace.deleted_at.map(|dt| dt.to_string()),
        }
    }
}

// ============================================================================
// Lifecycle API Models
// ============================================================================

/// Response model for restart workspace operation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RestartWorkspaceResponse {
    pub message: String,
    pub workspace_id: i32,
    pub container: ContainerInfo,
}

/// Container information for lifecycle operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ContainerInfo {
    pub id: i32,
    pub container_id: String,
    pub status: String,
    pub restart_count: i32,
    pub last_restart_at: Option<String>,
}

/// Response model for workspace stats operation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceStatsResponse {
    pub workspace_id: i32,
    pub container_id: String,
    pub stats: ContainerStatsInfo,
    pub collected_at: String,
}

/// Container resource usage statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ContainerStatsInfo {
    pub cpu_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub memory_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}
