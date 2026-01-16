//! Migration: Create webhook_configs table
//!
//! Creates the webhook_configs table for storing webhook configurations.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebhookConfigs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::ProviderId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::RepositoryId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::WebhookId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::WebhookSecret)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::WebhookUrl)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::Events)
                            .string_len(1024)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_provider")
                            .from(WebhookConfigs::Table, WebhookConfigs::ProviderId)
                            .to(Alias::new("repo_providers"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_repository")
                            .from(WebhookConfigs::Table, WebhookConfigs::RepositoryId)
                            .to(Alias::new("repositories"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on provider_id for queries filtering by provider
        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_configs_provider")
                    .table(WebhookConfigs::Table)
                    .col(WebhookConfigs::ProviderId)
                    .to_owned(),
            )
            .await?;

        // Create index on repository_id for queries filtering by repository
        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_configs_repository")
                    .table(WebhookConfigs::Table)
                    .col(WebhookConfigs::RepositoryId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint on (provider_id, repository_id) to prevent duplicates
        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_configs_provider_repository")
                    .table(WebhookConfigs::Table)
                    .col(WebhookConfigs::ProviderId)
                    .col(WebhookConfigs::RepositoryId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_webhook_configs_provider_repository")
                    .table(WebhookConfigs::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_webhook_configs_repository")
                    .table(WebhookConfigs::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_webhook_configs_provider")
                    .table(WebhookConfigs::Table)
                    .to_owned(),
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(WebhookConfigs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WebhookConfigs {
    Table,
    Id,
    ProviderId,
    RepositoryId,
    WebhookId,
    WebhookSecret,
    WebhookUrl,
    Events,
    Enabled,
    CreatedAt,
    UpdatedAt,
}
