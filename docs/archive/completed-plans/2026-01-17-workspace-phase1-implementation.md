# Workspace Phase 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement database foundation for Workspace module (migrations, entities, basic CRUD)

**Architecture:** TDD approach with database migrations first, then SeaORM entities, following GitAutoDev's layered design (Data → Service → API). Phase 1 focuses only on data layer.

**Tech Stack:** 
- SeaORM 0.12 (ORM)
- SQLite/PostgreSQL (Database)
- sea-orm-migration (Migrations)

**Reference:** `docs/plans/2026-01-17-workspace-concept-design.md`

---

## Task 1: Create workspaces table migration

**Files:**
- Create: `backend/src/migration/m20260117_000001_create_workspaces.rs`
- Modify: `backend/src/migration/mod.rs`

**Context:** This is the first table in the Workspace module. It has a one-to-one relationship with repositories table.

**Step 1: Write the migration file**

Create `backend/src/migration/m20260117_000001_create_workspaces.rs`:

```rust
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
```

**Step 2: Register migration in mod.rs**

Modify `backend/src/migration/mod.rs`:

```rust
// Add to imports
mod m20260117_000001_create_workspaces;

// Add to vec! in impl MigratorTrait
Box::new(m20260117_000001_create_workspaces::Migration),
```

**Step 3: Run migration to verify it works**

Run: `cargo run --bin backend`

Expected: Server starts successfully, migration runs, workspaces table created

**Step 4: Verify table structure**

Run: `sqlite3 ./data/gitautodev/db/gitautodev.db ".schema workspaces"`

Expected: Table with all columns and indices

**Step 5: Commit**

```bash
git add backend/src/migration/
git commit -m "feat(db): add workspaces table migration"
```

---

## Task 2: Create agents table migration

**Files:**
- Create: `backend/src/migration/m20260117_000002_create_agents.rs`
- Modify: `backend/src/migration/mod.rs`

**Context:** Agents table stores AI CLI tool configurations. One workspace can have multiple agents.

**Step 1: Write the migration file**

Create `backend/src/migration/m20260117_000002_create_agents.rs`:

```rust
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
                    .col(
                        ColumnDef::new(Agents::WorkspaceId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Agents::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Agents::ToolType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Agents::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Agents::Command)
                            .string()
                            .not_null(),
                    )
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
```

**Step 2: Register migration in mod.rs**

Modify `backend/src/migration/mod.rs`:

```rust
// Add to imports
mod m20260117_000002_create_agents;

// Add to vec! in impl MigratorTrait
Box::new(m20260117_000002_create_agents::Migration),
```

**Step 3: Run migration to verify it works**

Run: `cargo run --bin backend`

Expected: Server starts successfully, migration runs, agents table created

**Step 4: Verify table structure**

Run: `sqlite3 ./data/gitautodev/db/gitautodev.db ".schema agents"`

Expected: Table with all columns, indices, and foreign key

**Step 5: Commit**

```bash
git add backend/src/migration/
git commit -m "feat(db): add agents table migration"
```

---

## Task 3: Create tasks table migration

**Files:**
- Create: `backend/src/migration/m20260117_000003_create_tasks.rs`
- Modify: `backend/src/migration/mod.rs`

**Context:** Tasks table stores development tasks. Each task belongs to a workspace and uses an agent.

**Step 1: Write the migration file**

Create `backend/src/migration/m20260117_000003_create_tasks.rs`:

```rust
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
                        ColumnDef::new(Tasks::AgentId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Tasks::IssueId)
                            .integer(),
                    )
                    .col(ColumnDef::new(Tasks::IssueUrl).string())
                    .col(ColumnDef::new(Tasks::PrUrl).string())
                    .col(ColumnDef::new(Tasks::WorktreePath).string())
                    .col(
                        ColumnDef::new(Tasks::Status)
                            .string()
                            .not_null()
                            .default("Created"),
                    )
                    .col(
                        ColumnDef::new(Tasks::Priority)
                            .string()
                            .not_null()
                            .default("medium"),
                    )
                    .col(ColumnDef::new(Tasks::Timeout).integer())
                    .col(
                        ColumnDef::new(Tasks::RetryCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Tasks::CreatedBy)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Tasks::StartedAt).timestamp())
                    .col(ColumnDef::new(Tasks::CompletedAt).timestamp())
                    .col(ColumnDef::new(Tasks::ErrorMessage).text())
                    .col(ColumnDef::new(Tasks::ErrorType).string())
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
                            .name("fk_tasks_agent_id")
                            .from(Tasks::Table, Tasks::AgentId)
                            .to(Agents::Table, Agents::Id)
                            .on_delete(ForeignKeyAction::Restrict)
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
                    .name("idx_tasks_agent_id")
                    .table(Tasks::Table)
                    .col(Tasks::AgentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_status")
                    .table(Tasks::Table)
                    .col(Tasks::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_created_at")
                    .table(Tasks::Table)
                    .col(Tasks::CreatedAt)
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
    AgentId,
    IssueId,
    IssueUrl,
    PrUrl,
    WorktreePath,
    Status,
    Priority,
    Timeout,
    RetryCount,
    CreatedBy,
    StartedAt,
    CompletedAt,
    ErrorMessage,
    ErrorType,
    CreatedAt,
    UpdatedAt,
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
```

**Step 2: Register migration in mod.rs**

Modify `backend/src/migration/mod.rs`:

```rust
// Add to imports
mod m20260117_000003_create_tasks;

// Add to vec! in impl MigratorTrait
Box::new(m20260117_000003_create_tasks::Migration),
```

**Step 3: Run migration to verify it works**

Run: `cargo run --bin backend`

Expected: Server starts successfully, migration runs, tasks table created

**Step 4: Verify table structure**

Run: `sqlite3 ./data/gitautodev/db/gitautodev.db ".schema tasks"`

Expected: Table with all columns, indices, and foreign keys

**Step 5: Commit**

```bash
git add backend/src/migration/
git commit -m "feat(db): add tasks table migration"
```

---

## Task 4: Create task_logs table migration

**Files:**
- Create: `backend/src/migration/m20260117_000004_create_task_logs.rs`
- Modify: `backend/src/migration/mod.rs`

**Context:** Task logs table stores key events and summaries for tasks.

**Step 1: Write the migration file**

Create `backend/src/migration/m20260117_000004_create_task_logs.rs`:

```rust
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
                        ColumnDef::new(TaskLogs::EventType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskLogs::Message)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskLogs::Metadata)
                            .json(),
                    )
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
                    .name("idx_task_logs_event_type")
                    .table(TaskLogs::Table)
                    .col(TaskLogs::EventType)
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
    EventType,
    Message,
    Metadata,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
}
```

**Step 2: Register migration in mod.rs**

Modify `backend/src/migration/mod.rs`:

```rust
// Add to imports
mod m20260117_000004_create_task_logs;

// Add to vec! in impl MigratorTrait
Box::new(m20260117_000004_create_task_logs::Migration),
```

**Step 3: Run migration to verify it works**

Run: `cargo run --bin backend`

Expected: Server starts successfully, migration runs, task_logs table created

**Step 4: Verify table structure**

Run: `sqlite3 ./data/gitautodev/db/gitautodev.db ".schema task_logs"`

Expected: Table with all columns, indices, and foreign key

**Step 5: Commit**

```bash
git add backend/src/migration/
git commit -m "feat(db): add task_logs table migration"
```

---

## Task 5: Generate SeaORM entities

**Files:**
- Create: `backend/src/entities/workspace.rs`
- Create: `backend/src/entities/agent.rs`
- Create: `backend/src/entities/task.rs`
- Create: `backend/src/entities/task_log.rs`
- Modify: `backend/src/entities/mod.rs`
- Modify: `backend/src/entities/prelude.rs`

**Context:** Use sea-orm-cli to generate entities from database schema.

**Step 1: Generate entities**

Run: `cd backend && sea-orm-cli generate entity -o src/entities --with-serde both`

Expected: Four new entity files created

**Step 2: Verify generated files**

Check that these files exist:
- `backend/src/entities/workspace.rs`
- `backend/src/entities/agent.rs`
- `backend/src/entities/task.rs`
- `backend/src/entities/task_log.rs`

**Step 3: Update mod.rs**

Modify `backend/src/entities/mod.rs`:

```rust
pub mod workspace;
pub mod agent;
pub mod task;
pub mod task_log;
```

**Step 4: Update prelude.rs**

Modify `backend/src/entities/prelude.rs`:

```rust
pub use super::workspace::Entity as Workspace;
pub use super::agent::Entity as Agent;
pub use super::task::Entity as Task;
pub use super::task_log::Entity as TaskLog;
```

**Step 5: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 6: Commit**

```bash
git add backend/src/entities/
git commit -m "feat(entities): add workspace, agent, task, and task_log entities"
```

---

## Task 6: Run all tests to verify baseline

**Files:**
- None (verification only)

**Context:** Ensure all existing tests still pass after adding new migrations and entities.

**Step 1: Run unit tests**

Run: `cargo test --lib`

Expected: All tests pass (151+ tests)

**Step 2: Run integration tests**

Run: `cargo test --test '*'`

Expected: All tests pass

**Step 3: Verify no warnings**

Run: `cargo clippy`

Expected: No warnings or errors

**Step 4: Verify formatting**

Run: `cargo fmt --check`

Expected: All files properly formatted

**Step 5: Final commit if needed**

If any formatting changes:
```bash
cargo fmt
git add -u
git commit -m "style: format code"
```

---

## Summary

**Phase 1 Complete:** Database foundation for Workspace module

**What we built:**
- 4 database tables (workspaces, agents, tasks, task_logs)
- 4 SeaORM entities
- All migrations tested and working
- Foreign key relationships established
- Indices for performance

**What's next (Phase 2):**
- Workspace service layer
- Agent service layer
- Task service layer
- Basic CRUD operations

**Verification:**
- All migrations run successfully
- All entities generated
- All existing tests still pass
- No clippy warnings
- Code properly formatted
