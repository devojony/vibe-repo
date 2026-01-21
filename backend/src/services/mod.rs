//! Services module
//!
//! Contains background services and service lifecycle management.

pub mod agent_service;
pub mod container_config;
pub mod container_service;
pub mod docker_service;
pub mod health_check_service;
pub mod image_management_service;
pub mod init_script_service;
pub mod issue_polling_service;
pub mod log_cleanup_service;
pub mod repository_service;
pub mod service_manager;
pub mod task_execution_history_service;
pub mod task_executor_service;
pub mod task_failure_analyzer;
pub mod task_scheduler_service;
pub mod task_service;
pub mod webhook_cleanup_service;
pub mod webhook_retry_service;
pub mod workspace_service;

#[cfg(test)]
mod tests;

pub use agent_service::AgentService;
pub use container_config::ContainerConfig;
pub use container_service::ContainerService;
pub use docker_service::{BuildImageResult, DockerService, ImageInfo};
pub use health_check_service::HealthCheckService;
pub use image_management_service::ImageManagementService;
pub use init_script_service::InitScriptService;
pub use issue_polling_service::IssuePollingService;
pub use log_cleanup_service::LogCleanupService;
pub use repository_service::RepositoryService;
pub use service_manager::{BackgroundService, ServiceManager};
pub use task_execution_history_service::TaskExecutionService as TaskExecutionHistoryService;
pub use task_executor_service::TaskExecutorService;
pub use task_failure_analyzer::{FailureAnalysis, FailureCategory, TaskFailureAnalyzer};
pub use task_scheduler_service::{SchedulerConfig, TaskSchedulerService};
pub use task_service::TaskService;
pub use webhook_cleanup_service::WebhookCleanupService;
pub use webhook_retry_service::WebhookRetryService;
pub use workspace_service::WorkspaceService;
