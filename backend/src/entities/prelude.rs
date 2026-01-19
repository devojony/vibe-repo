//! SeaORM entity prelude
//!
//! Re-exports all entity types for convenient importing.

pub use super::agent::Entity as Agent;
pub use super::init_script::Entity as InitScript;
pub use super::repo_provider::Entity as RepoProvider;
pub use super::repository::Entity as Repository;
pub use super::task::Entity as Task;
pub use super::task_log::Entity as TaskLog;
pub use super::webhook_config::Entity as WebhookConfig;
pub use super::workspace::Entity as Workspace;
