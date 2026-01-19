# Init Script Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace custom_dockerfile_path with init_script functionality, allowing users to configure shell scripts that execute automatically after container startup.

**Architecture:** Create separate init_scripts table with 1:1 relationship to workspaces. Implement hybrid storage (≤4KB in database, >4KB in filesystem). Add Docker exec integration for script execution with concurrency control and automatic cleanup.

**Tech Stack:** Rust, SeaORM, Axum, Bollard (Docker), PostgreSQL, Tokio

---

## Prerequisites

- Design document reviewed and approved: `docs/plans/2026-01-19-init-script-feature-design.md`
- Working in main branch or dedicated feature branch
- Database connection configured
- Docker available for testing

---

## Phase 1: Database Migration and Entity

### Task 1.1: Create Database Migration

**Files:**
- Create: `backend/src/migration/m20260119_000001_replace_dockerfile_with_init_script.rs`
- Modify: `backend/src/migration/mod.rs`

**Step 1: Create migration file**

Create `backend/src/migration/m20260119_000001_replace_dockerfile_with_init_script.rs`:

```rust
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
```

**Step 2: Register migration in mod.rs**

Modify `backend/src/migration/mod.rs`, add to the migrations vector:

```rust
vec![
    // ... existing migrations
    Box::new(m20260119_000001_replace_dockerfile_with_init_script::Migration),
]
```

**Step 3: Run migration**

```bash
cd backend
cargo run --bin migration up
```

Expected output: Migration applied successfully

**Step 4: Verify migration**

```bash
# Connect to database and verify
psql $DATABASE_URL -c "\d init_scripts"
psql $DATABASE_URL -c "\d workspaces" | grep custom_dockerfile_path
```

Expected: init_scripts table exists, custom_dockerfile_path column removed

**Step 5: Commit**

```bash
git add backend/src/migration/
git commit -m "feat(db): add init_scripts table and remove custom_dockerfile_path

- Create init_scripts table with 1:1 relationship to workspaces
- Add indexes on workspace_id and status
- Remove custom_dockerfile_path from workspaces table
- Support hybrid storage with output_summary and output_file_path

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 1.2: Generate and Update Entity

**Files:**
- Create: `backend/src/entities/init_script.rs`
- Modify: `backend/src/entities/mod.rs`
- Modify: `backend/src/entities/prelude.rs`
- Modify: `backend/src/entities/workspace.rs`

**Step 1: Generate entity from database**

```bash
cd backend
sea-orm-cli generate entity \
  -u $DATABASE_URL \
  -o src/entities \
  --with-serde both \
  --tables init_scripts
```

Expected: `src/entities/init_script.rs` created

**Step 2: Verify generated entity**

Check `backend/src/entities/init_script.rs` contains correct fields and derives.

**Step 3: Add entity to mod.rs**

Modify `backend/src/entities/mod.rs`:

```rust
pub mod init_script;
```

**Step 4: Add entity to prelude**

Modify `backend/src/entities/prelude.rs`:

```rust
pub use super::init_script::Entity as InitScript;
```

**Step 5: Update workspace entity relations**

Modify `backend/src/entities/workspace.rs`, add to Relation enum:

```rust
#[sea_orm(has_one = "super::init_script::Entity")]
InitScript,
```

Add Related implementation:

```rust
impl Related<super::init_script::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InitScript.def()
    }
}
```

**Step 6: Update init_script entity relations**

Modify `backend/src/entities/init_script.rs`, add to Relation enum:

```rust
#[sea_orm(
    belongs_to = "super::workspace::Entity",
    from = "Column::WorkspaceId",
    to = "super::workspace::Column::Id",
    on_update = "Cascade",
    on_delete = "Cascade"
)]
Workspace,
```

Add Related implementation:

```rust
impl Related<super::workspace::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workspace.def()
    }
}
```

**Step 7: Build to verify**

```bash
cd backend
cargo build
```

Expected: Build succeeds

**Step 8: Commit**

```bash
git add backend/src/entities/
git commit -m "feat(entities): add init_script entity and update workspace relations

- Generate init_script entity from database
- Add has_one relationship from workspace to init_script
- Add belongs_to relationship from init_script to workspace

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: Docker Service Enhancement

### Task 2.1: Add exec_in_container Method

**Files:**
- Modify: `backend/src/services/docker_service.rs`

**Step 1: Add ExecOutput struct**

Add to `backend/src/services/docker_service.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}
```

**Step 2: Write failing test**

Add test to `backend/src/services/docker_service.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exec_in_container_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create test container
        let container_id = match service
            .create_container("test-exec", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // Start container
        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        // Execute command
        let result = service
            .exec_in_container(&container_id, vec!["echo".to_string(), "hello".to_string()], 10)
            .await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        // Assert
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello"));
    }
}
```

**Step 3: Run test to verify it fails**

```bash
cd backend
cargo test test_exec_in_container_success -- --nocapture
```

Expected: FAIL with "exec_in_container not found"

**Step 4: Implement exec_in_container**

Add method to `DockerService` impl:

```rust
pub async fn exec_in_container(
    &self,
    container_id: &str,
    cmd: Vec<String>,
    timeout_secs: u64,
) -> Result<ExecOutput> {
    use bollard::exec::{CreateExecOptions, StartExecResults};
    use futures::StreamExt;
    use tokio::time::{timeout, Duration};

    // Create exec instance
    let exec_config = CreateExecOptions {
        cmd: Some(cmd),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        ..Default::default()
    };

    let exec = self
        .docker
        .create_exec(container_id, exec_config)
        .await
        .map_err(|e| VibeRepoError::Internal(format!("Failed to create exec: {}", e)))?;

    // Start exec with timeout
    let exec_id = exec.id.clone();
    let docker = self.docker.clone();

    let result = timeout(Duration::from_secs(timeout_secs), async move {
        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } =
            docker.start_exec(&exec_id, None).await.map_err(|e| {
                VibeRepoError::Internal(format!("Failed to start exec: {}", e))
            })?
        {
            while let Some(Ok(msg)) = output.next().await {
                use bollard::container::LogOutput;
                match msg {
                    LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        // Get exit code
        let inspect = docker.inspect_exec(&exec_id).await.map_err(|e| {
            VibeRepoError::Internal(format!("Failed to inspect exec: {}", e))
        })?;

        let exit_code = inspect.exit_code.unwrap_or(-1);

        Ok::<ExecOutput, VibeRepoError>(ExecOutput {
            exit_code,
            stdout,
            stderr,
        })
    })
    .await;

    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(VibeRepoError::Internal(
            "Command execution timed out".to_string(),
        )),
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cd backend
cargo test test_exec_in_container_success -- --nocapture
```

Expected: PASS

**Step 6: Commit**

```bash
git add backend/src/services/docker_service.rs
git commit -m "feat(docker): add exec_in_container method with timeout support

- Add ExecOutput struct for command results
- Implement exec_in_container using bollard exec API
- Support timeout with tokio::time::timeout
- Capture stdout and stderr separately
- Add test for successful command execution

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

_Plan continues in next message..._

## Phase 3: InitScriptService Implementation

### Task 3.1: Create InitScriptService with Basic CRUD

**Files:**
- Create: `backend/src/services/init_script_service.rs`
- Modify: `backend/src/services/mod.rs`

**Step 1: Create service file structure**

Create `backend/src/services/init_script_service.rs`:

```rust
use crate::entities::{prelude::*, init_script};
use crate::error::{VibeRepoError, Result};
use crate::services::DockerService;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, QueryFilter, ColumnTrait};

#[derive(Clone)]
pub struct InitScriptService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}

impl InitScriptService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }
}
```

**Step 2: Write test for create_init_script**

Add to `backend/src/services/init_script_service.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestDatabase;

    #[tokio::test]
    async fn test_create_init_script_success() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace
        let workspace = create_test_workspace(db).await;

        let service = InitScriptService::new(db.clone(), None);

        // Act
        let result = service
            .create_init_script(
                workspace.id,
                "#!/bin/bash\necho 'test'".to_string(),
                300,
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.workspace_id, workspace.id);
        assert_eq!(script.status, "Pending");
        assert_eq!(script.timeout_seconds, 300);
    }

    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        // Implementation depends on your test utils
        todo!("Implement test workspace creation")
    }
}
```

**Step 3: Run test to verify it fails**

```bash
cd backend
cargo test test_create_init_script_success
```

Expected: FAIL with "create_init_script not found"

**Step 4: Implement create_init_script**

Add to `InitScriptService` impl:

```rust
pub async fn create_init_script(
    &self,
    workspace_id: i32,
    script_content: String,
    timeout_seconds: i32,
) -> Result<init_script::Model> {
    // Verify workspace exists
    let _workspace = Workspace::find_by_id(workspace_id)
        .one(&self.db)
        .await
        .map_err(VibeRepoError::Database)?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!("Workspace {} not found", workspace_id))
        })?;

    // Create script
    let script = init_script::ActiveModel {
        workspace_id: Set(workspace_id),
        script_content: Set(script_content),
        timeout_seconds: Set(timeout_seconds),
        status: Set("Pending".to_string()),
        ..Default::default()
    };

    let script = InitScript::insert(script)
        .exec_with_returning(&self.db)
        .await
        .map_err(VibeRepoError::Database)?;

    tracing::info!(
        workspace_id = workspace_id,
        script_id = script.id,
        "Created init script for workspace"
    );

    Ok(script)
}
```

**Step 5: Run test to verify it passes**

```bash
cd backend
cargo test test_create_init_script_success
```

Expected: PASS

**Step 6: Implement get_init_script_by_workspace_id**

Add method and test:

```rust
pub async fn get_init_script_by_workspace_id(
    &self,
    workspace_id: i32,
) -> Result<Option<init_script::Model>> {
    let script = InitScript::find()
        .filter(init_script::Column::WorkspaceId.eq(workspace_id))
        .one(&self.db)
        .await
        .map_err(VibeRepoError::Database)?;

    Ok(script)
}

#[cfg(test)]
#[tokio::test]
async fn test_get_init_script_by_workspace_id() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let workspace = create_test_workspace(db).await;
    let service = InitScriptService::new(db.clone(), None);

    // Create script
    let created = service
        .create_init_script(workspace.id, "test".to_string(), 300)
        .await
        .unwrap();

    // Act
    let result = service.get_init_script_by_workspace_id(workspace.id).await;

    // Assert
    assert!(result.is_ok());
    let script = result.unwrap();
    assert!(script.is_some());
    assert_eq!(script.unwrap().id, created.id);
}
```

**Step 7: Implement update_init_script**

Add method:

```rust
pub async fn update_init_script(
    &self,
    workspace_id: i32,
    script_content: String,
    timeout_seconds: i32,
) -> Result<init_script::Model> {
    let script = self
        .get_init_script_by_workspace_id(workspace_id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Init script for workspace {} not found",
                workspace_id
            ))
        })?;

    let mut script: init_script::ActiveModel = script.into();
    script.script_content = Set(script_content);
    script.timeout_seconds = Set(timeout_seconds);
    script.status = Set("Pending".to_string()); // Reset status
    script.updated_at = Set(Utc::now());

    let script = script.update(&self.db).await.map_err(VibeRepoError::Database)?;

    tracing::info!(
        workspace_id = workspace_id,
        script_id = script.id,
        "Updated init script"
    );

    Ok(script)
}
```

**Step 8: Register service in mod.rs**

Modify `backend/src/services/mod.rs`:

```rust
mod init_script_service;
pub use init_script_service::InitScriptService;
```

**Step 9: Build to verify**

```bash
cd backend
cargo build
```

Expected: Build succeeds

**Step 10: Commit**

```bash
git add backend/src/services/
git commit -m "feat(service): add InitScriptService with basic CRUD operations

- Implement create_init_script with workspace validation
- Implement get_init_script_by_workspace_id
- Implement update_init_script with status reset
- Add tests for all CRUD operations
- Add structured logging

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3.2: Implement Script Execution Logic

**Files:**
- Modify: `backend/src/services/init_script_service.rs`

**Step 1: Add output storage helper functions**

Add to `InitScriptService`:

```rust
const MAX_SUMMARY_SIZE: usize = 4096; // 4KB

async fn save_script_output(
    script_id: i32,
    workspace_id: i32,
    stdout: String,
    stderr: String,
) -> Result<(Option<String>, Option<String>)> {
    let full_output = format!("=== STDOUT ===\n{}\n\n=== STDERR ===\n{}", stdout, stderr);

    if full_output.len() <= MAX_SUMMARY_SIZE {
        // Small output: store in database only
        Ok((Some(full_output), None))
    } else {
        // Large output: store summary in DB, full in file
        let summary = Self::extract_last_4kb(&full_output);
        let file_path = Self::write_to_file(script_id, workspace_id, &full_output).await?;
        Ok((Some(summary), Some(file_path)))
    }
}

fn extract_last_4kb(output: &str) -> String {
    if output.len() <= MAX_SUMMARY_SIZE {
        output.to_string()
    } else {
        let start = output.len() - MAX_SUMMARY_SIZE;
        format!(
            "... [Output truncated, showing last 4KB]\n\n{}",
            &output[start..]
        )
    }
}

async fn write_to_file(
    script_id: i32,
    workspace_id: i32,
    content: &str,
) -> Result<String> {
    use tokio::fs;

    let base_dir = "/data/gitautodev/init-script-logs";
    let workspace_dir = format!("{}/workspace-{}", base_dir, workspace_id);

    // Create directory
    fs::create_dir_all(&workspace_dir)
        .await
        .map_err(|e| VibeRepoError::Internal(format!("Failed to create log directory: {}", e)))?;

    // Generate filename
    let timestamp = Utc::now().timestamp();
    let filename = format!("script-{}-{}.log", script_id, timestamp);
    let file_path = format!("{}/{}", workspace_dir, filename);

    // Write file
    fs::write(&file_path, content)
        .await
        .map_err(|e| VibeRepoError::Internal(format!("Failed to write log file: {}", e)))?;

    Ok(file_path)
}
```

**Step 2: Write test for execute_script**

Add test:

```rust
#[tokio::test]
async fn test_execute_script_success() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let workspace = create_test_workspace_with_container(db).await;

    let docker = DockerService::new().ok();
    if docker.is_none() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let service = InitScriptService::new(db.clone(), docker);

    // Create script
    let script = service
        .create_init_script(workspace.id, "echo 'test'".to_string(), 10)
        .await
        .unwrap();

    // Act
    let result = service
        .execute_script(workspace.id, workspace.container_id.as_ref().unwrap())
        .await;

    // Assert
    assert!(result.is_ok());

    // Verify status updated
    let updated = service
        .get_init_script_by_workspace_id(workspace.id)
        .await
        .unwrap()
        .unwrap();
    assert!(updated.status == "Success" || updated.status == "Running");
}
```

**Step 3: Run test to verify it fails**

```bash
cd backend
cargo test test_execute_script_success
```

Expected: FAIL with "execute_script not found"

**Step 4: Implement execute_script**

Add method:

```rust
pub async fn execute_script(
    &self,
    workspace_id: i32,
    container_id: &str,
) -> Result<init_script::Model> {
    let docker = self.docker.as_ref().ok_or_else(|| {
        VibeRepoError::ServiceUnavailable("Docker service is not available".to_string())
    })?;

    let script = self
        .get_init_script_by_workspace_id(workspace_id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Init script for workspace {} not found",
                workspace_id
            ))
        })?;

    // Update status to Running
    let mut script_active: init_script::ActiveModel = script.clone().into();
    script_active.status = Set("Running".to_string());
    script_active.updated_at = Set(Utc::now());
    let script = script_active.update(&self.db).await.map_err(VibeRepoError::Database)?;

    tracing::info!(
        workspace_id = workspace_id,
        script_id = script.id,
        container_id = container_id,
        "Starting init script execution"
    );

    // Execute script in container
    let cmd = vec!["/bin/bash".to_string(), "-c".to_string(), script.script_content.clone()];
    let timeout = script.timeout_seconds as u64;

    let result = docker.exec_in_container(container_id, cmd, timeout).await;

    // Process result and update script
    match result {
        Ok(output) => {
            let (summary, file_path) = Self::save_script_output(
                script.id,
                workspace_id,
                output.stdout,
                output.stderr,
            )
            .await?;

            let status = if output.exit_code == 0 {
                "Success"
            } else {
                "Failed"
            };

            let mut script_active: init_script::ActiveModel = script.into();
            script_active.status = Set(status.to_string());
            script_active.output_summary = Set(summary);
            script_active.output_file_path = Set(file_path);
            script_active.executed_at = Set(Some(Utc::now()));
            script_active.updated_at = Set(Utc::now());

            let script = script_active.update(&self.db).await.map_err(VibeRepoError::Database)?;

            tracing::info!(
                workspace_id = workspace_id,
                script_id = script.id,
                exit_code = output.exit_code,
                status = status,
                "Init script execution completed"
            );

            Ok(script)
        }
        Err(e) => {
            // Update status to Failed
            let error_msg = format!("Execution error: {}", e);
            let mut script_active: init_script::ActiveModel = script.into();
            script_active.status = Set("Failed".to_string());
            script_active.output_summary = Set(Some(error_msg.clone()));
            script_active.executed_at = Set(Some(Utc::now()));
            script_active.updated_at = Set(Utc::now());

            let script = script_active.update(&self.db).await.map_err(VibeRepoError::Database)?;

            tracing::error!(
                workspace_id = workspace_id,
                script_id = script.id,
                error = %e,
                "Init script execution failed"
            );

            Err(VibeRepoError::Internal(error_msg))
        }
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cd backend
cargo test test_execute_script_success -- --nocapture
```

Expected: PASS

**Step 6: Commit**

```bash
git add backend/src/services/init_script_service.rs
git commit -m "feat(service): implement script execution with hybrid storage

- Add execute_script method with Docker exec integration
- Implement hybrid storage (≤4KB DB, >4KB file)
- Add output file writing to /data/gitautodev/init-script-logs
- Handle execution success, failure, and timeout
- Add comprehensive logging
- Add test for script execution

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

_Plan continues..._

### Task 3.3: Add Concurrency Control

**Files:**
- Modify: `backend/src/services/init_script_service.rs`

**Step 1: Write test for concurrent execution rejection**

Add test:

```rust
#[tokio::test]
async fn test_concurrent_execution_rejected() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let workspace = create_test_workspace_with_container(db).await;

    let docker = DockerService::new().ok();
    if docker.is_none() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let service = InitScriptService::new(db.clone(), docker);

    // Create script with long-running command
    service
        .create_init_script(workspace.id, "sleep 30".to_string(), 60)
        .await
        .unwrap();

    // Start first execution (don't await)
    let service_clone = service.clone();
    let workspace_id = workspace.id;
    let container_id = workspace.container_id.clone().unwrap();
    tokio::spawn(async move {
        let _ = service_clone.execute_script(workspace_id, &container_id).await;
    });

    // Wait a bit for first execution to start
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Try second execution
    let result = service
        .execute_script(workspace.id, workspace.container_id.as_ref().unwrap())
        .await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(VibeRepoError::Conflict(_)) => {}, // Expected
        _ => panic!("Expected Conflict error"),
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cd backend
cargo test test_concurrent_execution_rejected
```

Expected: FAIL (no concurrency check yet)

**Step 3: Add concurrency check to execute_script**

Modify `execute_script` method, add check before updating status:

```rust
pub async fn execute_script(
    &self,
    workspace_id: i32,
    container_id: &str,
) -> Result<init_script::Model> {
    let docker = self.docker.as_ref().ok_or_else(|| {
        VibeRepoError::ServiceUnavailable("Docker service is not available".to_string())
    })?;

    let script = self
        .get_init_script_by_workspace_id(workspace_id)
        .await?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Init script for workspace {} not found",
                workspace_id
            ))
        })?;

    // Check if already running
    if script.status == "Running" {
        tracing::warn!(
            workspace_id = workspace_id,
            script_id = script.id,
            current_status = script.status,
            "Rejected concurrent script execution"
        );
        return Err(VibeRepoError::Conflict(
            "Script is already running".to_string(),
        ));
    }

    // ... rest of implementation
}
```

**Step 4: Run test to verify it passes**

```bash
cd backend
cargo test test_concurrent_execution_rejected
```

Expected: PASS

**Step 5: Add database lock for race condition prevention**

Modify `execute_script` to use transaction with lock:

```rust
pub async fn execute_script(
    &self,
    workspace_id: i32,
    container_id: &str,
) -> Result<init_script::Model> {
    use sea_orm::{TransactionTrait, LockType};

    let docker = self.docker.as_ref().ok_or_else(|| {
        VibeRepoError::ServiceUnavailable("Docker service is not available".to_string())
    })?;

    // Start transaction
    let txn = self.db.begin().await.map_err(VibeRepoError::Database)?;

    // Lock and get script
    let script = InitScript::find()
        .filter(init_script::Column::WorkspaceId.eq(workspace_id))
        .lock(LockType::Update)
        .one(&txn)
        .await
        .map_err(VibeRepoError::Database)?
        .ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Init script for workspace {} not found",
                workspace_id
            ))
        })?;

    // Check if already running
    if script.status == "Running" {
        txn.rollback().await.map_err(VibeRepoError::Database)?;
        tracing::warn!(
            workspace_id = workspace_id,
            script_id = script.id,
            "Rejected concurrent script execution"
        );
        return Err(VibeRepoError::Conflict(
            "Script is already running".to_string(),
        ));
    }

    // Update status to Running
    let mut script_active: init_script::ActiveModel = script.clone().into();
    script_active.status = Set("Running".to_string());
    script_active.updated_at = Set(Utc::now());
    let script = script_active.update(&txn).await.map_err(VibeRepoError::Database)?;

    // Commit transaction
    txn.commit().await.map_err(VibeRepoError::Database)?;

    tracing::info!(
        workspace_id = workspace_id,
        script_id = script.id,
        container_id = container_id,
        "Starting init script execution"
    );

    // Execute script (outside transaction)
    let cmd = vec!["/bin/bash".to_string(), "-c".to_string(), script.script_content.clone()];
    let timeout = script.timeout_seconds as u64;

    // ... rest of execution logic
}
```

**Step 6: Commit**

```bash
git add backend/src/services/init_script_service.rs
git commit -m "feat(service): add concurrency control with database locking

- Add status check to reject concurrent execution
- Implement database row locking (SELECT FOR UPDATE)
- Return 409 Conflict when script is already running
- Add test for concurrent execution rejection
- Add structured logging for rejected requests

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 4: API Layer Implementation

### Task 4.1: Update API Models

**Files:**
- Modify: `backend/src/api/workspaces/models.rs`

**Step 1: Remove custom_dockerfile_path from models**

Modify `CreateWorkspaceRequest`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceRequest {
    pub repository_id: i32,
    pub init_script: Option<String>,  // New field
    #[serde(default = "default_script_timeout")]
    pub script_timeout_seconds: i32,  // New field
    #[serde(default = "default_image_source")]
    pub image_source: String,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: i32,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_memory_limit")]
    pub memory_limit: String,
    #[serde(default = "default_disk_limit")]
    pub disk_limit: String,
}

fn default_script_timeout() -> i32 {
    300
}
```

Remove `custom_dockerfile_path` from `WorkspaceResponse` and add `init_script`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceResponse {
    pub id: i32,
    pub repository_id: i32,
    pub workspace_status: String,
    pub container_id: Option<String>,
    pub container_status: Option<String>,
    pub image_source: String,
    pub init_script: Option<InitScriptResponse>,  // New field
    pub max_concurrent_tasks: i32,
    pub cpu_limit: f64,
    pub memory_limit: String,
    pub disk_limit: String,
    pub work_dir: Option<String>,
    pub health_status: Option<String>,
    pub last_health_check: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}
```

**Step 2: Add new API models**

Add to `backend/src/api/workspaces/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InitScriptResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub script_content: String,
    pub timeout_seconds: i32,
    pub status: String,
    pub output_summary: Option<String>,
    pub has_full_log: bool,
    pub executed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<init_script::Model> for InitScriptResponse {
    fn from(model: init_script::Model) -> Self {
        Self {
            id: model.id,
            workspace_id: model.workspace_id,
            script_content: model.script_content,
            timeout_seconds: model.timeout_seconds,
            status: model.status,
            output_summary: model.output_summary,
            has_full_log: model.output_file_path.is_some(),
            executed_at: model.executed_at.map(|dt| dt.to_string()),
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateInitScriptRequest {
    pub script_content: String,
    #[serde(default = "default_script_timeout")]
    pub timeout_seconds: i32,
    #[serde(default)]
    pub execute_immediately: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecuteScriptRequest {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InitScriptLogsResponse {
    pub status: String,
    pub output_summary: Option<String>,
    pub has_full_log: bool,
    pub executed_at: Option<String>,
}
```

**Step 3: Update WorkspaceResponse From impl**

Modify the From implementation to include init_script:

```rust
impl From<(workspace::Model, Option<init_script::Model>)> for WorkspaceResponse {
    fn from((workspace, init_script): (workspace::Model, Option<init_script::Model>)) -> Self {
        Self {
            id: workspace.id,
            repository_id: workspace.repository_id,
            workspace_status: workspace.workspace_status,
            container_id: workspace.container_id,
            container_status: workspace.container_status,
            image_source: workspace.image_source,
            init_script: init_script.map(InitScriptResponse::from),
            max_concurrent_tasks: workspace.max_concurrent_tasks,
            cpu_limit: workspace.cpu_limit,
            memory_limit: workspace.memory_limit,
            disk_limit: workspace.disk_limit,
            work_dir: workspace.work_dir,
            health_status: workspace.health_status,
            last_health_check: workspace.last_health_check.map(|dt| dt.to_string()),
            created_at: workspace.created_at.to_string(),
            updated_at: workspace.updated_at.to_string(),
            deleted_at: workspace.deleted_at.map(|dt| dt.to_string()),
        }
    }
}
```

**Step 4: Build to verify**

```bash
cd backend
cargo build
```

Expected: Build succeeds

**Step 5: Commit**

```bash
git add backend/src/api/workspaces/models.rs
git commit -m "feat(api): update workspace models for init script support

- Remove custom_dockerfile_path from request/response models
- Add init_script and script_timeout_seconds to CreateWorkspaceRequest
- Add init_script field to WorkspaceResponse
- Add InitScriptResponse, UpdateInitScriptRequest, ExecuteScriptRequest models
- Add InitScriptLogsResponse for log endpoints
- Update From implementations

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

_Plan continues in next section..._

### Task 4.2: Add Init Script API Handlers

**Files:**
- Create: `backend/src/api/init_scripts/mod.rs`
- Create: `backend/src/api/init_scripts/handlers.rs`
- Create: `backend/src/api/init_scripts/routes.rs`
- Modify: `backend/src/api/mod.rs`

**Implementation Summary:**
- Create handlers for: update_init_script, get_logs, download_full_log, execute_script
- Add routes: PUT /:id/init-script, GET /:id/init-script/logs, GET /:id/init-script/logs/full, POST /:id/init-script/execute
- Integrate with InitScriptService
- Add OpenAPI documentation
- Handle 409 Conflict for concurrent execution

**Step-by-step implementation follows TDD pattern with tests first**

---

### Task 4.3: Update Workspace Handlers

**Files:**
- Modify: `backend/src/api/workspaces/handlers.rs`

**Changes:**
- Update create_workspace to accept init_script parameter
- Update get_workspace to include init_script in response
- Update list_workspaces to include init_script in responses
- Integrate InitScriptService for script creation

---

## Phase 5: Integration and Testing

### Task 5.1: Integration Tests

**Files:**
- Create: `backend/tests/init_scripts/mod.rs`
- Create: `backend/tests/init_scripts/init_script_api_tests.rs`

**Test Coverage:**
1. test_create_workspace_with_init_script
2. test_update_init_script
3. test_execute_init_script_success
4. test_execute_init_script_failure
5. test_execute_init_script_timeout
6. test_concurrent_execution_rejected
7. test_get_init_script_logs
8. test_download_full_log
9. test_log_file_cleanup

---

### Task 5.2: End-to-End Testing

**Manual Testing Checklist:**

1. **Create workspace with script**
   ```bash
   curl -X POST http://localhost:3000/api/workspaces \
     -H "Content-Type: application/json" \
     -d '{
       "repository_id": 1,
       "init_script": "#!/bin/bash\napt-get update && apt-get install -y git"
     }'
   ```

2. **Verify script execution**
   ```bash
   curl http://localhost:3000/api/workspaces/1/init-script/logs
   ```

3. **Download full log**
   ```bash
   curl http://localhost:3000/api/workspaces/1/init-script/logs/full -o script.log
   ```

4. **Test concurrent execution**
   ```bash
   # Start long-running script
   curl -X POST http://localhost:3000/api/workspaces/1/init-script/execute
   # Immediately try again
   curl -X POST http://localhost:3000/api/workspaces/1/init-script/execute
   # Should return 409 Conflict
   ```

5. **Test force execution**
   ```bash
   curl -X POST http://localhost:3000/api/workspaces/1/init-script/execute?force=true
   ```

---

## Phase 6: Cleanup and Documentation

### Task 6.1: Log Cleanup Service

**Files:**
- Create: `backend/src/services/log_cleanup_service.rs`
- Modify: `backend/src/services/mod.rs`
- Modify: `backend/src/main.rs`

**Implementation:**
- Create background task for log cleanup
- Run daily at 2 AM
- Delete logs older than 30 days
- Delete empty workspace directories
- Add structured logging

---

### Task 6.2: Update OpenAPI Documentation

**Files:**
- Modify: `backend/src/api/mod.rs`

**Changes:**
- Update OpenAPI schema with new endpoints
- Add examples for init script requests
- Document error responses (409 Conflict)
- Update workspace schema

---

### Task 6.3: Update README and Migration Guide

**Files:**
- Modify: `README.md`
- Create: `docs/migration-guide-init-scripts.md`

**Content:**
- Document new init script feature
- Provide migration guide from custom_dockerfile_path
- Add usage examples
- Document API endpoints
- Explain hybrid storage strategy

---

## Phase 7: Final Verification

### Task 7.1: Run All Tests

```bash
cd backend
cargo test
```

Expected: All tests pass

### Task 7.2: Run Migration on Clean Database

```bash
cd backend
cargo run --bin migration fresh
cargo run --bin migration up
```

Expected: All migrations apply successfully

### Task 7.3: Manual Smoke Test

1. Start server
2. Create workspace with init script
3. Verify script executes
4. Check logs
5. Test all API endpoints
6. Verify file storage
7. Test cleanup

### Task 7.4: Final Commit

```bash
git add .
git commit -m "feat: complete init script feature implementation

Replace custom_dockerfile_path with comprehensive init script functionality:

Database:
- New init_scripts table with 1:1 relationship to workspaces
- Hybrid storage (≤4KB DB, >4KB filesystem)
- 6 states: Pending/Running/Success/Failed/Timeout/Cancelled

Services:
- InitScriptService with CRUD and execution
- Docker exec integration with timeout
- Concurrency control with database locking
- Automatic log cleanup (30-day retention)

API:
- 4 new endpoints for script management
- Updated workspace endpoints
- 409 Conflict for concurrent execution
- Full log download support

Testing:
- Comprehensive unit tests
- Integration tests for all endpoints
- Concurrency and race condition tests

Documentation:
- OpenAPI schema updates
- Migration guide
- Usage examples

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Summary

This implementation plan provides:

1. **Phase 1**: Database migration and entity generation
2. **Phase 2**: Docker service enhancement for exec support
3. **Phase 3**: InitScriptService with CRUD, execution, and concurrency control
4. **Phase 4**: API layer with new endpoints and updated models
5. **Phase 5**: Comprehensive testing (unit, integration, E2E)
6. **Phase 6**: Cleanup service and documentation
7. **Phase 7**: Final verification and deployment

**Estimated Tasks**: ~25 tasks
**Estimated Time**: 2-3 days for experienced developer

**Key Principles Applied**:
- TDD: Write tests first, then implementation
- DRY: Reuse code, avoid duplication
- YAGNI: Only implement what's needed
- Frequent commits: After each task completion

---

## Execution Options

Plan complete and saved to `docs/plans/2026-01-19-init-script-implementation-plan.md`.

**Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration
   - Use @superpowers:subagent-driven-development

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints
   - Use @superpowers:executing-plans in new session

**Which approach do you prefer?**
