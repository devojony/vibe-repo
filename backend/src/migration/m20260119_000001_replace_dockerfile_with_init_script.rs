use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create init_scripts table
        manager
            .create_table(
                Table::create()
                    .table(InitScripts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InitScripts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InitScripts::WorkspaceId)
                            .integer()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(InitScripts::ScriptContent)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InitScripts::TimeoutSeconds)
                            .integer()
                            .not_null()
                            .default(300),
                    )
                    .col(
                        ColumnDef::new(InitScripts::Status)
                            .string()
                            .not_null()
                            .default("Pending"),
                    )
                    .col(ColumnDef::new(InitScripts::OutputSummary).text())
                    .col(ColumnDef::new(InitScripts::OutputFilePath).string_len(500))
                    .col(ColumnDef::new(InitScripts::ExecutedAt).timestamp())
                    .col(
                        ColumnDef::new(InitScripts::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(InitScripts::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_init_scripts_workspace_id")
                            .from(InitScripts::Table, InitScripts::WorkspaceId)
                            .to(Workspaces::Table, Workspaces::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_init_scripts_workspace_id")
                    .table(InitScripts::Table)
                    .col(InitScripts::WorkspaceId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_init_scripts_status")
                    .table(InitScripts::Table)
                    .col(InitScripts::Status)
                    .to_owned(),
            )
            .await?;

        // Drop custom_dockerfile_path column from workspaces
        manager
            .alter_table(
                Table::alter()
                    .table(Workspaces::Table)
                    .drop_column(Workspaces::CustomDockerfilePath)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add back custom_dockerfile_path column
        manager
            .alter_table(
                Table::alter()
                    .table(Workspaces::Table)
                    .add_column(ColumnDef::new(Workspaces::CustomDockerfilePath).string())
                    .to_owned(),
            )
            .await?;

        // Drop init_scripts table
        manager
            .drop_table(Table::drop().table(InitScripts::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum InitScripts {
    Table,
    Id,
    WorkspaceId,
    ScriptContent,
    TimeoutSeconds,
    Status,
    OutputSummary,
    OutputFilePath,
    ExecutedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
    CustomDockerfilePath,
}
