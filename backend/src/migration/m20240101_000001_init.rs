//! Initial migration
//!
//! Creates the initial database schema (placeholder for now).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Initial migration - schema will be added in future tasks
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Rollback - will be implemented with schema
        Ok(())
    }
}
