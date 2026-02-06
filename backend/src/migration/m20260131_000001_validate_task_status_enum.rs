use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // This migration validates the transition from string-based task status to enum-based status.
        //
        // Background:
        // - The task_status column in the tasks table is stored as a string in the database
        // - SeaORM's DeriveActiveEnum stores enum values as strings (e.g., "pending", "running")
        // - No database schema changes are needed
        //
        // Validation:
        // - The application code now enforces valid enum values at the Rust type level
        // - Valid status values: "pending", "assigned", "running", "completed", "failed", "cancelled"
        // - Any existing tasks with invalid status values will cause errors when loaded
        //
        // If you have tasks with invalid status values, update them manually:
        // UPDATE tasks SET task_status = 'pending' WHERE task_status NOT IN
        //   ('pending', 'assigned', 'running', 'completed', 'failed', 'cancelled');

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // No schema changes to revert
        // The task_status column remains as a string column
        // The application code would need to be reverted to accept arbitrary strings
        Ok(())
    }
}
