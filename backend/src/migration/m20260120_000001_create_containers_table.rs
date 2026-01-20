use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Create containers table
        manager
            .create_table(
                Table::create()
                    .table(Containers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Containers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Containers::WorkspaceId)
                            .integer()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Containers::ContainerId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Containers::ContainerName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Containers::ImageName).string().not_null())
                    .col(ColumnDef::new(Containers::ImageId).string())
                    .col(
                        ColumnDef::new(Containers::Status)
                            .string()
                            .not_null()
                            .default("creating"),
                    )
                    .col(ColumnDef::new(Containers::HealthStatus).string())
                    .col(ColumnDef::new(Containers::ExitCode).integer())
                    .col(ColumnDef::new(Containers::ErrorMessage).text())
                    .col(
                        ColumnDef::new(Containers::RestartCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Containers::MaxRestartAttempts)
                            .integer()
                            .not_null()
                            .default(3),
                    )
                    .col(ColumnDef::new(Containers::LastRestartAt).timestamp())
                    .col(ColumnDef::new(Containers::LastHealthCheck).timestamp())
                    .col(
                        ColumnDef::new(Containers::HealthCheckFailures)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Containers::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Containers::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Containers::StartedAt).timestamp())
                    .col(ColumnDef::new(Containers::StoppedAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_containers_workspace_id")
                            .from(Containers::Table, Containers::WorkspaceId)
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
                    .name("idx_containers_workspace_id")
                    .table(Containers::Table)
                    .col(Containers::WorkspaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_containers_status")
                    .table(Containers::Table)
                    .col(Containers::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_containers_container_id")
                    .table(Containers::Table)
                    .col(Containers::ContainerId)
                    .to_owned(),
            )
            .await?;

        // Step 2: Migrate existing container data from workspaces table
        // Note: This uses raw SQL because SeaORM migration doesn't support complex data migration
        let db = manager.get_connection();

        // Check if there are any workspaces with container data
        let sql = r#"
            INSERT INTO containers (
                workspace_id,
                container_id,
                container_name,
                image_name,
                status,
                health_status,
                last_health_check,
                created_at,
                updated_at
            )
            SELECT 
                id as workspace_id,
                container_id,
                'workspace-' || id as container_name,
                COALESCE(image_source, 'vibe-repo-workspace:latest') as image_name,
                COALESCE(container_status, 'unknown') as status,
                health_status,
                last_health_check,
                created_at,
                updated_at
            FROM workspaces
            WHERE container_id IS NOT NULL AND container_id != ''
        "#;

        db.execute_unprepared(sql).await?;

        // Step 3: Drop old container-related columns from workspaces table
        // Note: SQLite doesn't support DROP COLUMN, so we need to recreate the table
        // For now, we'll keep the columns but they will be deprecated
        // In a future migration, we can recreate the table without these columns

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop containers table
        manager
            .drop_table(Table::drop().table(Containers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Containers {
    Table,
    Id,
    WorkspaceId,
    ContainerId,
    ContainerName,
    ImageName,
    ImageId,
    Status,
    HealthStatus,
    ExitCode,
    ErrorMessage,
    RestartCount,
    MaxRestartAttempts,
    LastRestartAt,
    LastHealthCheck,
    HealthCheckFailures,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    StoppedAt,
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
}
