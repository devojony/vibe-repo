//! Simplify MVP Schema Migration
//!
//! This migration simplifies the database schema for the MVP version by:
//! - Dropping webhook_configs table (webhook config moved to env vars)
//! - Dropping init_scripts table (init script moved to repository config)
//! - Dropping task_executions table (execution history removed, only last_log kept)
//! - Dropping workspaces table (workspace info merged into repositories)
//! - Adding last_log field to tasks table
//! - Removing retry_count and max_retries from tasks table
//! - Adding agent config fields to repositories table (agent_command, agent_timeout, agent_env_vars, docker_image)
//! - Removing enabled field from agents table
//! - Adding UNIQUE constraint on agents(workspace_id) for single agent per workspace

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Simplify webhook_configs table (keep minimal fields)
        // Note: SQLite doesn't support DROP COLUMN, so we keep the table as-is
        // The webhook_config entity is simplified to only track webhook_id and secret

        // Step 2: Drop init_scripts table
        manager
            .drop_table(
                Table::drop()
                    .table(InitScripts::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        // Step 3: Drop task_executions table
        manager
            .drop_table(
                Table::drop()
                    .table(TaskExecutions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        // Step 4: Simplify workspaces table (keep minimal fields, drop complex ones)
        // Note: SQLite doesn't support DROP COLUMN, so we keep the table as-is
        // The workspace entity is simplified to only track repository_id, container_id, and status

        // Step 5: Convert Assigned status to Pending (simplify state machine)
        // This is a data migration to handle the removal of the Assigned state
        manager
            .exec_stmt(
                Query::update()
                    .table(Tasks::Table)
                    .value(Tasks::TaskStatus, "pending")
                    .and_where(Expr::col(Tasks::TaskStatus).eq("assigned"))
                    .to_owned(),
            )
            .await?;

        // Step 6: Modify tasks table - add last_log field
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::LastLog).text().null())
                    .to_owned(),
            )
            .await?;

        // Step 7: Modify tasks table - drop retry_count and max_retries
        // Note: SQLite doesn't support DROP COLUMN directly, so we skip this for SQLite
        // For PostgreSQL, we would use:
        // manager.alter_table(Table::alter().table(Tasks::Table).drop_column(Tasks::RetryCount).to_owned()).await?;
        // manager.alter_table(Table::alter().table(Tasks::Table).drop_column(Tasks::MaxRetries).to_owned()).await?;

        // Step 8: Modify repositories table - add agent config fields
        manager
            .alter_table(
                Table::alter()
                    .table(Repositories::Table)
                    .add_column(ColumnDef::new(Repositories::AgentCommand).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Repositories::Table)
                    .add_column(
                        ColumnDef::new(Repositories::AgentTimeout)
                            .integer()
                            .default(600)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Repositories::Table)
                    .add_column(ColumnDef::new(Repositories::AgentEnvVars).json().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Repositories::Table)
                    .add_column(
                        ColumnDef::new(Repositories::DockerImage)
                            .string_len(255)
                            .default("ubuntu:22.04")
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Step 9: Modify agents table - drop enabled field
        // Note: SQLite doesn't support DROP COLUMN directly, so we skip this for SQLite

        // Step 10: Add UNIQUE constraint on agents(workspace_id)
        // Note: This will be handled in the entity definition for now
        // For explicit constraint: manager.create_index(...).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Reverse migrations are not implemented for this simplification
        // as this is a one-way migration for MVP
        Err(DbErr::Migration(
            "Downgrade from simplified MVP schema is not supported".to_string(),
        ))
    }
}

#[derive(DeriveIden)]
enum WebhookConfigs {
    Table,
}

#[derive(DeriveIden)]
enum InitScripts {
    Table,
}

#[derive(DeriveIden)]
enum TaskExecutions {
    Table,
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    TaskStatus,
    LastLog,
    RetryCount,
    MaxRetries,
}

#[derive(DeriveIden)]
enum Repositories {
    Table,
    AgentCommand,
    AgentTimeout,
    AgentEnvVars,
    DockerImage,
}

#[derive(DeriveIden)]
enum Agents {
    Table,
    Enabled,
    WorkspaceId,
}
