//! Services module
//!
//! Contains background services and service lifecycle management.

pub mod agent_service;
pub mod docker_service;
pub mod health_check_service;
pub mod repository_service;
pub mod service_manager;
pub mod task_service;
pub mod webhook_cleanup_service;
pub mod webhook_retry_service;
pub mod workspace_service;

#[cfg(test)]
mod tests;

pub use agent_service::AgentService;
pub use docker_service::DockerService;
pub use health_check_service::HealthCheckService;
pub use repository_service::RepositoryService;
pub use service_manager::{BackgroundService, ServiceManager};
pub use task_service::TaskService;
pub use webhook_cleanup_service::WebhookCleanupService;
pub use webhook_retry_service::WebhookRetryService;
pub use workspace_service::WorkspaceService;
