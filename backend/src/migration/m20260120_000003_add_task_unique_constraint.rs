//! Migration: Add unique constraint to tasks table
//!
//! Adds unique index on (workspace_id, issue_number) to prevent duplicate tasks for the same issue.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create unique index on workspace_id and issue_number
        // This prevents creating multiple tasks for the same issue in a workspace
        // Note: SQLite doesn't support partial indexes with WHERE clause,
        // so soft-deleted records (deleted_at IS NOT NULL) will also be covered by this index.
        // Application layer must handle this by checking deleted_at before creating new tasks.
        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_workspace_issue_unique")
                    .table(Alias::new("tasks"))
                    .col(Alias::new("workspace_id"))
                    .col(Alias::new("issue_number"))
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop unique index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_tasks_workspace_issue_unique")
                    .table(Alias::new("tasks"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
