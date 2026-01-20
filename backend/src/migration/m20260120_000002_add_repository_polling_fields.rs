//! Migration: Add repository polling fields
//!
//! Adds polling_enabled, polling_interval_seconds, and last_issue_poll_at fields to repositories table.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add polling_enabled column with default value false
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .add_column(
                        ColumnDef::new(Alias::new("polling_enabled"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Add polling_interval_seconds column with default value 300 (5 minutes)
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .add_column(
                        ColumnDef::new(Alias::new("polling_interval_seconds"))
                            .integer()
                            .not_null()
                            .default(300),
                    )
                    .to_owned(),
            )
            .await?;

        // Add last_issue_poll_at column (nullable)
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .add_column(ColumnDef::new(Alias::new("last_issue_poll_at")).timestamp())
                    .to_owned(),
            )
            .await?;

        // Create index on polling_enabled for efficient queries
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_polling_enabled")
                    .table(Alias::new("repositories"))
                    .col(Alias::new("polling_enabled"))
                    .to_owned(),
            )
            .await?;

        // Create index on last_issue_poll_at for efficient polling scheduling
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_last_issue_poll_at")
                    .table(Alias::new("repositories"))
                    .col(Alias::new("last_issue_poll_at"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_repositories_last_issue_poll_at")
                    .table(Alias::new("repositories"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_repositories_polling_enabled")
                    .table(Alias::new("repositories"))
                    .to_owned(),
            )
            .await?;

        // Drop columns
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .drop_column(Alias::new("last_issue_poll_at"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .drop_column(Alias::new("polling_interval_seconds"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .drop_column(Alias::new("polling_enabled"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
