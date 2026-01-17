use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Agents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Agents::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Agents::WorkspaceId).integer().not_null())
                    .col(ColumnDef::new(Agents::Name).string().not_null())
                    .col(ColumnDef::new(Agents::ToolType).string().not_null())
                    .col(
                        ColumnDef::new(Agents::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Agents::Command).string().not_null())
                    .col(
                        ColumnDef::new(Agents::EnvVars)
                            .json()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(Agents::Timeout)
                            .integer()
                            .not_null()
                            .default(1800), // 30 minutes
                    )
                    .col(
                        ColumnDef::new(Agents::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Agents::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_agents_workspace_id")
                            .from(Agents::Table, Agents::WorkspaceId)
                            .to(Workspaces::Table, Workspaces::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indices
        manager
            .create_index(
                Index::create()
                    .name("idx_agents_workspace_id")
                    .table(Agents::Table)
                    .col(Agents::WorkspaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_agents_enabled")
                    .table(Agents::Table)
                    .col(Agents::Enabled)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Agents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Agents {
    Table,
    Id,
    WorkspaceId,
    Name,
    ToolType,
    Enabled,
    Command,
    EnvVars,
    Timeout,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
}
