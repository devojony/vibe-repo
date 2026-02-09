//! Services module
//!
//! Contains background services and service lifecycle management.

pub mod acp;
pub mod agent_manager;
pub mod agent_service;
pub mod container_config;
pub mod container_service;
pub mod docker_service;
pub mod git_service;
pub mod issue_closure_service;
pub mod pr_creation_service;
pub mod repository_service;
pub mod service_manager;
pub mod task_executor_service;
pub mod task_scheduler_service;
pub mod task_service;
pub mod timeout_watchdog;
pub mod workspace_service;

#[cfg(test)]
mod tests;

pub use agent_manager::{AgentConfig, AgentHandle, AgentManager, AgentStatus, AgentType, ResourceUsage};
pub use agent_service::AgentService;
pub use container_config::ContainerConfig;
pub use container_service::ContainerService;
pub use docker_service::{BuildImageResult, DockerService, ImageInfo};
pub use git_service::GitService;
pub use issue_closure_service::IssueClosureService;
pub use pr_creation_service::PRCreationService;
pub use repository_service::RepositoryService;
pub use service_manager::{BackgroundService, ServiceManager};
pub use task_executor_service::TaskExecutorService;
pub use task_scheduler_service::{SchedulerConfig, TaskSchedulerService};
pub use task_service::TaskService;
pub use timeout_watchdog::{TaskExecution, TimeoutWatchdog};
pub use workspace_service::WorkspaceService;
