//! Add ACP Integration Fields to Tasks Table
//!
//! This migration adds two JSONB fields to the tasks table for ACP integration:
//! - plans: Stores agent execution plans with steps and status
//! - events: Stores agent events (tool calls, messages, etc.)
//!
//! Both fields are nullable and use:
//! - JSONB type for PostgreSQL
//! - TEXT type for SQLite (JSON stored as text)

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add plans field (JSONB for PostgreSQL, TEXT for SQLite)
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(
                        ColumnDef::new(Tasks::Plans)
                            .json()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        // Add events field (JSONB for PostgreSQL, TEXT for SQLite)
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(
                        ColumnDef::new(Tasks::Events)
                            .json()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove events field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::Events)
                    .to_owned(),
            )
            .await?;

        // Remove plans field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::Plans)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Plans,
    Events,
}
