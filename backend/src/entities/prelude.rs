//! Entity prelude
//!
//! Re-exports commonly used entity types for convenience.

pub use super::repo_provider::{Entity as RepoProvider, Model as RepoProviderModel};
pub use super::repository::{Entity as Repository, Model as RepositoryModel};
pub use super::webhook_config::{Entity as WebhookConfig, Model as WebhookConfigModel};

// Entity re-exports will be added here as entities are created
// pub use super::repo_provider::Entity as RepoProvider;
// pub use super::repository::Entity as Repository;
// pub use super::workspace::Entity as Workspace;
// pub use super::agent::Entity as Agent;
// pub use super::task::Entity as Task;
