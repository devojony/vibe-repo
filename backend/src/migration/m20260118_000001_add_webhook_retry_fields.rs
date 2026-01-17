//! Migration: Add webhook retry fields
//!
//! Adds retry tracking fields to webhook_configs table for exponential backoff retry logic.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite doesn't support multiple ALTER TABLE operations in one statement
        // Add each column separately
        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .add_column(
                        ColumnDef::new(WebhookConfigs::RetryCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .add_column(ColumnDef::new(WebhookConfigs::LastRetryAt).timestamp().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .add_column(ColumnDef::new(WebhookConfigs::NextRetryAt).timestamp().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .add_column(ColumnDef::new(WebhookConfigs::LastError).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite doesn't support multiple ALTER TABLE operations in one statement
        // Drop each column separately
        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .drop_column(WebhookConfigs::LastError)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .drop_column(WebhookConfigs::NextRetryAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .drop_column(WebhookConfigs::LastRetryAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WebhookConfigs::Table)
                    .drop_column(WebhookConfigs::RetryCount)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum WebhookConfigs {
    Table,
    RetryCount,
    LastRetryAt,
    NextRetryAt,
    LastError,
}
