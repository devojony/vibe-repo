use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TaskLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskLogs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TaskLogs::TaskId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskLogs::LogLevel)
                            .string()
                            .not_null()
                            .default("Info"),
                    )
                    .col(
                        ColumnDef::new(TaskLogs::Message)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TaskLogs::Metadata).json())
                    .col(
                        ColumnDef::new(TaskLogs::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_logs_task_id")
                            .from(TaskLogs::Table, TaskLogs::TaskId)
                            .to(Tasks::Table, Tasks::Id)
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
                    .name("idx_task_logs_task_id")
                    .table(TaskLogs::Table)
                    .col(TaskLogs::TaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_logs_level")
                    .table(TaskLogs::Table)
                    .col(TaskLogs::LogLevel)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_task_logs_created_at")
                    .table(TaskLogs::Table)
                    .col(TaskLogs::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaskLogs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TaskLogs {
    Table,
    Id,
    TaskId,
    LogLevel,
    Message,
    Metadata,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
}
