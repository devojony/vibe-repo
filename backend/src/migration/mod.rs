//! Migration module
//!
//! Contains database migrations managed by SeaORM Migration framework.

pub use sea_orm_migration::prelude::*;

mod m20240101_000001_init;
mod m20250114_000001_create_repo_providers;
mod m20250114_000002_create_repositories;
mod m20250114_000003_add_provider_unique_constraint;
mod m20250117_000001_add_repository_status_and_soft_delete;
mod m20260117_000001_create_workspaces;
mod m20260117_000002_create_agents;
mod m20260117_000003_create_webhook_configs;
mod m20260117_000004_add_repository_webhook_status;
mod m20260117_000005_create_tasks;
mod m20260117_000006_create_task_logs;
mod m20260118_000001_add_webhook_retry_fields;
mod m20260119_000001_replace_dockerfile_with_init_script;
mod m20260120_000001_create_containers_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_init::Migration),
            Box::new(m20250114_000001_create_repo_providers::Migration),
            Box::new(m20250114_000002_create_repositories::Migration),
            Box::new(m20250114_000003_add_provider_unique_constraint::Migration),
            Box::new(m20250117_000001_add_repository_status_and_soft_delete::Migration),
            Box::new(m20260117_000001_create_workspaces::Migration),
            Box::new(m20260117_000002_create_agents::Migration),
            Box::new(m20260117_000003_create_webhook_configs::Migration),
            Box::new(m20260117_000004_add_repository_webhook_status::Migration),
            Box::new(m20260117_000005_create_tasks::Migration),
            Box::new(m20260117_000006_create_task_logs::Migration),
            Box::new(m20260118_000001_add_webhook_retry_fields::Migration),
            Box::new(m20260119_000001_replace_dockerfile_with_init_script::Migration),
            Box::new(m20260120_000001_create_containers_table::Migration),
        ]
    }
}
