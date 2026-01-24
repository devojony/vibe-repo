use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TaskExecutions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskExecutions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskExecutions::TaskId).integer().not_null())
                    .col(ColumnDef::new(TaskExecutions::AgentId).integer().null())
                    .col(
                        ColumnDef::new(TaskExecutions::Status)
                            .string()
                            .not_null()
                            .default("running"),
                    )
                    .col(ColumnDef::new(TaskExecutions::Command).text().not_null())
                    .col(ColumnDef::new(TaskExecutions::ExitCode).integer().null())
                    .col(ColumnDef::new(TaskExecutions::StdoutSummary).text().null())
                    .col(ColumnDef::new(TaskExecutions::StderrSummary).text().null())
                    .col(
                        ColumnDef::new(TaskExecutions::StdoutFilePath)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(TaskExecutions::StderrFilePath)
                            .string()
                            .null(),
                    )
                    .col(ColumnDef::new(TaskExecutions::ErrorMessage).text().null())
                    .col(ColumnDef::new(TaskExecutions::PrNumber).integer().null())
                    .col(ColumnDef::new(TaskExecutions::PrUrl).string().null())
                    .col(ColumnDef::new(TaskExecutions::BranchName).string().null())
                    .col(
                        ColumnDef::new(TaskExecutions::DurationMs)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(TaskExecutions::StartedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TaskExecutions::CompletedAt)
                            .timestamp()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(TaskExecutions::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TaskExecutions::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_executions_task_id")
                            .from(TaskExecutions::Table, TaskExecutions::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_executions_agent_id")
                            .from(TaskExecutions::Table, TaskExecutions::AgentId)
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
                    .name("idx_task_executions_task_id")
                    .table(TaskExecutions::Table)
                    .col(TaskExecutions::TaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_executions_status")
                    .table(TaskExecutions::Table)
                    .col(TaskExecutions::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_executions_started_at")
                    .table(TaskExecutions::Table)
                    .col(TaskExecutions::StartedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaskExecutions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TaskExecutions {
    Table,
    Id,
    TaskId,
    AgentId,
    Status,
    Command,
    ExitCode,
    StdoutSummary,
    StderrSummary,
    StdoutFilePath,
    StderrFilePath,
    ErrorMessage,
    PrNumber,
    PrUrl,
    BranchName,
    DurationMs,
    StartedAt,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Agents {
    Table,
    Id,
}
