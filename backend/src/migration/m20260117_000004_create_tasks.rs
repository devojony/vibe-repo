use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tasks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Tasks::WorkspaceId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Tasks::IssueNumber)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Tasks::IssueTitle)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Tasks::IssueBody).text())
                    .col(
                        ColumnDef::new(Tasks::TaskStatus)
                            .string()
                            .not_null()
                            .default("Pending"),
                    )
                    .col(
                        ColumnDef::new(Tasks::Priority)
                            .string()
                            .not_null()
                            .default("Medium"),
                    )
                    .col(ColumnDef::new(Tasks::AssignedAgentId).integer())
                    .col(ColumnDef::new(Tasks::BranchName).string())
                    .col(ColumnDef::new(Tasks::PrNumber).integer())
                    .col(ColumnDef::new(Tasks::PrUrl).string())
                    .col(ColumnDef::new(Tasks::ErrorMessage).text())
                    .col(ColumnDef::new(Tasks::RetryCount).integer().not_null().default(0))
                    .col(
                        ColumnDef::new(Tasks::MaxRetries)
                            .integer()
                            .not_null()
                            .default(3),
                    )
                    .col(ColumnDef::new(Tasks::StartedAt).timestamp())
                    .col(ColumnDef::new(Tasks::CompletedAt).timestamp())
                    .col(
                        ColumnDef::new(Tasks::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Tasks::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Tasks::DeletedAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tasks_workspace_id")
                            .from(Tasks::Table, Tasks::WorkspaceId)
                            .to(Workspaces::Table, Workspaces::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tasks_assigned_agent_id")
                            .from(Tasks::Table, Tasks::AssignedAgentId)
                            .to(Agents::Table, Agents::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indices
        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_workspace_id")
                    .table(Tasks::Table)
                    .col(Tasks::WorkspaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_status")
                    .table(Tasks::Table)
                    .col(Tasks::TaskStatus)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_issue_number")
                    .table(Tasks::Table)
                    .col(Tasks::IssueNumber)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_assigned_agent_id")
                    .table(Tasks::Table)
                    .col(Tasks::AssignedAgentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_deleted_at")
                    .table(Tasks::Table)
                    .col(Tasks::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tasks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
    WorkspaceId,
    IssueNumber,
    IssueTitle,
    IssueBody,
    TaskStatus,
    Priority,
    AssignedAgentId,
    BranchName,
    PrNumber,
    PrUrl,
    ErrorMessage,
    RetryCount,
    MaxRetries,
    StartedAt,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Agents {
    Table,
    Id,
}
