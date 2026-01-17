//! Migration: Add webhook_status field to repositories table
//!
//! Adds the webhook_status column to track webhook creation status.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Repositories::Table)
                    .add_column(
                        ColumnDef::new(Repositories::WebhookStatus)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Repositories::Table)
                    .drop_column(Repositories::WebhookStatus)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Repositories {
    Table,
    WebhookStatus,
}
