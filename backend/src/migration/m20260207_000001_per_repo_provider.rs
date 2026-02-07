//! Per-Repository Provider Configuration Migration
//!
//! This migration transforms the database schema from Provider-Centric to Repository-Centric:
//! - Drops webhook_configs table (webhook config moved to repository)
//! - Drops repo_providers table (provider config moved to repository)
//! - Recreates repositories table with embedded provider configuration
//!
//! New repositories table includes:
//! - All existing repository fields
//! - Provider configuration: provider_type, provider_base_url, access_token
//! - Webhook configuration: webhook_secret
//!
//! This is a breaking migration that requires a fresh database.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Drop webhook_configs table (has FK to repositories)
        manager
            .drop_table(
                Table::drop()
                    .table(WebhookConfigs::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        // Step 2: Drop repositories table (has FK to repo_providers)
        manager
            .drop_table(
                Table::drop()
                    .table(Repositories::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        // Step 3: Drop repo_providers table
        manager
            .drop_table(
                Table::drop()
                    .table(RepoProviders::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        // Step 4: Create new repositories table with embedded provider configuration
        manager
            .create_table(
                Table::create()
                    .table(Repositories::Table)
                    .if_not_exists()
                    // Primary key
                    .col(
                        ColumnDef::new(Repositories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    // Repository identification
                    .col(ColumnDef::new(Repositories::Name).string().not_null())
                    .col(ColumnDef::new(Repositories::FullName).string().not_null())
                    .col(ColumnDef::new(Repositories::CloneUrl).string().not_null())
                    .col(
                        ColumnDef::new(Repositories::DefaultBranch)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Repositories::Branches)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    // Provider configuration (NEW)
                    .col(
                        ColumnDef::new(Repositories::ProviderType)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Repositories::ProviderBaseUrl)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Repositories::AccessToken)
                            .string()
                            .not_null(),
                    )
                    // Webhook configuration (NEW)
                    .col(ColumnDef::new(Repositories::WebhookSecret).string().null())
                    // Validation status
                    .col(
                        ColumnDef::new(Repositories::ValidationStatus)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(Repositories::HasRequiredBranches)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Repositories::HasRequiredLabels)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Repositories::CanManagePrs)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Repositories::CanManageIssues)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Repositories::ValidationMessage)
                            .string()
                            .null(),
                    )
                    // Repository status
                    .col(
                        ColumnDef::new(Repositories::Status)
                            .string_len(20)
                            .not_null()
                            .default("uninitialized"),
                    )
                    .col(
                        ColumnDef::new(Repositories::HasWorkspace)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Repositories::WebhookStatus)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    // Agent configuration
                    .col(ColumnDef::new(Repositories::AgentCommand).text().null())
                    .col(
                        ColumnDef::new(Repositories::AgentTimeout)
                            .integer()
                            .default(600)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Repositories::AgentEnvVars).json().null())
                    .col(
                        ColumnDef::new(Repositories::DockerImage)
                            .string_len(255)
                            .default("ubuntu:22.04")
                            .not_null(),
                    )
                    // Soft delete
                    .col(ColumnDef::new(Repositories::DeletedAt).timestamp().null())
                    // Timestamps
                    .col(
                        ColumnDef::new(Repositories::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Repositories::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Step 5: Create indexes

        // Index on provider_type for filtering by provider
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_provider_type")
                    .table(Repositories::Table)
                    .col(Repositories::ProviderType)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on full_name (repository must be unique)
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_fullname_unique")
                    .table(Repositories::Table)
                    .col(Repositories::FullName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on validation_status
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_validation_status")
                    .table(Repositories::Table)
                    .col(Repositories::ValidationStatus)
                    .to_owned(),
            )
            .await?;

        // Index on status
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_status")
                    .table(Repositories::Table)
                    .col(Repositories::Status)
                    .to_owned(),
            )
            .await?;

        // Index on deleted_at for soft delete queries
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_deleted_at")
                    .table(Repositories::Table)
                    .col(Repositories::DeletedAt)
                    .to_owned(),
            )
            .await?;

        // Index on has_workspace
        manager
            .create_index(
                Index::create()
                    .name("idx_repositories_has_workspace")
                    .table(Repositories::Table)
                    .col(Repositories::HasWorkspace)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Downgrade is not supported for this breaking migration
        Err(DbErr::Migration(
            "Downgrade from per-repository provider configuration is not supported".to_string(),
        ))
    }
}

#[derive(DeriveIden)]
enum WebhookConfigs {
    Table,
}

#[derive(DeriveIden)]
enum RepoProviders {
    Table,
}

#[derive(DeriveIden)]
enum Repositories {
    Table,
    // Primary key
    Id,
    // Repository identification
    Name,
    FullName,
    CloneUrl,
    DefaultBranch,
    Branches,
    // Provider configuration
    ProviderType,
    ProviderBaseUrl,
    AccessToken,
    // Webhook configuration
    WebhookSecret,
    // Validation status
    ValidationStatus,
    HasRequiredBranches,
    HasRequiredLabels,
    CanManagePrs,
    CanManageIssues,
    ValidationMessage,
    // Repository status
    Status,
    HasWorkspace,
    WebhookStatus,
    // Agent configuration
    AgentCommand,
    AgentTimeout,
    AgentEnvVars,
    DockerImage,
    // Soft delete
    DeletedAt,
    // Timestamps
    CreatedAt,
    UpdatedAt,
}
