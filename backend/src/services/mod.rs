//! Services module
//!
//! Contains background services and service lifecycle management.

pub mod repository_service;
pub mod service_manager;
pub mod workspace_service;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod repository_service_tests;

pub use repository_service::RepositoryService;
pub use service_manager::{BackgroundService, ServiceManager};
pub use workspace_service::WorkspaceService;
