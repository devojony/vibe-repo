use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Workspaces::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Workspaces::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Workspaces::RepositoryId)
                            .integer()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Workspaces::WorkspaceStatus)
                            .string()
                            .not_null()
                            .default("Initializing"),
                    )
                    .col(ColumnDef::new(Workspaces::ContainerId).string())
                    .col(ColumnDef::new(Workspaces::ContainerStatus).string())
                    .col(
                        ColumnDef::new(Workspaces::ImageSource)
                            .string()
                            .not_null()
                            .default("default"),
                    )
                    .col(ColumnDef::new(Workspaces::CustomDockerfilePath).string())
                    .col(
                        ColumnDef::new(Workspaces::MaxConcurrentTasks)
                            .integer()
                            .not_null()
                            .default(3),
                    )
                    .col(
                        ColumnDef::new(Workspaces::CpuLimit)
                            .double()
                            .not_null()
                            .default(2.0),
                    )
                    .col(
                        ColumnDef::new(Workspaces::MemoryLimit)
                            .string()
                            .not_null()
                            .default("4GB"),
                    )
                    .col(
                        ColumnDef::new(Workspaces::DiskLimit)
                            .string()
                            .not_null()
                            .default("10GB"),
                    )
                    .col(ColumnDef::new(Workspaces::WorkDir).string())
                    .col(ColumnDef::new(Workspaces::HealthStatus).string())
                    .col(ColumnDef::new(Workspaces::LastHealthCheck).timestamp())
                    .col(
                        ColumnDef::new(Workspaces::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Workspaces::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Workspaces::DeletedAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_workspaces_repository_id")
                            .from(Workspaces::Table, Workspaces::RepositoryId)
                            .to(Repositories::Table, Repositories::Id)
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
                    .name("idx_workspaces_repository_id")
                    .table(Workspaces::Table)
                    .col(Workspaces::RepositoryId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_workspaces_status")
                    .table(Workspaces::Table)
                    .col(Workspaces::WorkspaceStatus)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_workspaces_deleted_at")
                    .table(Workspaces::Table)
                    .col(Workspaces::DeletedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Workspaces::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
    RepositoryId,
    WorkspaceStatus,
    ContainerId,
    ContainerStatus,
    ImageSource,
    CustomDockerfilePath,
    MaxConcurrentTasks,
    CpuLimit,
    MemoryLimit,
    DiskLimit,
    WorkDir,
    HealthStatus,
    LastHealthCheck,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Repositories {
    Table,
    Id,
}
