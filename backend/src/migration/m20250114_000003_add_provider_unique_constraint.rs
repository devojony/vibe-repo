//! Migration: Add unique constraint to repo_providers
//!
//! Adds a unique constraint on (name, base_url, access_token) to prevent duplicate provider configurations.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add unique constraint on (name, base_url, access_token)
        manager
            .create_index(
                Index::create()
                    .name("idx_repo_providers_unique")
                    .table(Alias::new("repo_providers"))
                    .col(Alias::new("name"))
                    .col(Alias::new("base_url"))
                    .col(Alias::new("access_token"))
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the unique constraint
        manager
            .drop_index(
                Index::drop()
                    .name("idx_repo_providers_unique")
                    .table(Alias::new("repo_providers"))
                    .to_owned(),
            )
            .await
    }
}
