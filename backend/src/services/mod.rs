//! Services module
//!
//! Contains background services and service lifecycle management.

pub mod agent_service;
pub mod docker_service;
pub mod repository_service;
pub mod service_manager;
pub mod task_service;
pub mod workspace_service;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod repository_service_tests;

pub use agent_service::AgentService;
pub use docker_service::DockerService;
pub use repository_service::RepositoryService;
pub use service_manager::{BackgroundService, ServiceManager};
pub use task_service::TaskService;
pub use workspace_service::WorkspaceService;
