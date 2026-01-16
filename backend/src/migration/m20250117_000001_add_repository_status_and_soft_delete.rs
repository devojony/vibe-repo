//! Migration: Add repository status and soft delete fields
//!
//! Adds status, has_workspace, and deleted_at fields to repositories table.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add status column with default value 'uninitialized'
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .add_column(
                        ColumnDef::new(Alias::new("status"))
                            .string_len(20)
                            .not_null()
                            .default("uninitialized"),
                    )
                    .to_owned(),
            )
            .await?;

        // Add has_workspace column with default value false
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .add_column(
                        ColumnDef::new(Alias::new("has_workspace"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Add deleted_at column (nullable)
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .add_column(ColumnDef::new(Alias::new("deleted_at")).timestamp())
                    .to_owned(),
            )
            .await?;

        // Create index on status
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_repo_status")
                    .table(Alias::new("repositories"))
                    .col(Alias::new("status"))
                    .to_owned(),
            )
            .await?;

        // Create index on deleted_at
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_deleted_at")
                    .table(Alias::new("repositories"))
                    .col(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await?;

        // Create index on has_workspace
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_has_workspace")
                    .table(Alias::new("repositories"))
                    .col(Alias::new("has_workspace"))
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
                    .name("idx_repositories_has_workspace")
                    .table(Alias::new("repositories"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_repositories_deleted_at")
                    .table(Alias::new("repositories"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_repositories_repo_status")
                    .table(Alias::new("repositories"))
                    .to_owned(),
            )
            .await?;

        // Drop columns
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .drop_column(Alias::new("deleted_at"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .drop_column(Alias::new("has_workspace"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repositories"))
                    .drop_column(Alias::new("status"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
