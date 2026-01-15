//! Migration module
//!
//! Contains database migrations managed by SeaORM Migration framework.

pub use sea_orm_migration::prelude::*;

mod m20240101_000001_init;
mod m20250114_000001_create_repo_providers;
mod m20250114_000002_create_repositories;
mod m20250114_000003_add_provider_unique_constraint;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_init::Migration),
            Box::new(m20250114_000001_create_repo_providers::Migration),
            Box::new(m20250114_000002_create_repositories::Migration),
            Box::new(m20250114_000003_add_provider_unique_constraint::Migration),
        ]
    }
}
