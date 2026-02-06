# Workspace Phase 2 Implementation Plan - Service Layer

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement service layer for Workspace module (business logic, CRUD operations, validation)

**Architecture:** TDD approach with service layer implementing business logic. Services coordinate between data layer (entities) and API layer. Each service handles one entity type.

**Tech Stack:** 
- SeaORM 0.12 (ORM)
- Tokio (Async runtime)
- Rust standard error handling

**Reference:** 
- `docs/plans/2026-01-17-workspace-concept-design.md`
- `docs/plans/2026-01-17-workspace-phase1-implementation.md`

**Prerequisites:** Phase 1 completed (migrations and entities exist)

---

## Task 1: Create WorkspaceService with basic CRUD

**Files:**
- Create: `backend/src/services/workspace_service.rs`
- Modify: `backend/src/services/mod.rs`

**Context:** WorkspaceService handles business logic for workspace management. Start with basic CRUD operations.

**Step 1: Write failing test for create_workspace**

Create `backend/src/services/workspace_service.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::db::TestDatabase;
    use crate::entities::prelude::*;
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn test_create_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = test_db.connection();
        
        // Create a test repository first
        let repo = crate::entities::repository::ActiveModel {
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(1),
            ..Default::default()
        };
        let repo = Repository::insert(repo).exec(db).await.unwrap();
        
        let service = WorkspaceService::new(db.clone());
        
        // Act
        let result = service.create_workspace(repo.last_insert_id).await;
        
        // Assert
        assert!(result.is_ok());
        let workspace = result.unwrap();
        assert_eq!(workspace.repository_id, repo.last_insert_id);
        assert_eq!(workspace.workspace_status, "Initializing");
        assert_eq!(workspace.max_concurrent_tasks, 3);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd backend && cargo test test_create_workspace_success`

Expected: FAIL with "WorkspaceService not found" or similar

**Step 3: Implement minimal WorkspaceService**

Add to `backend/src/services/workspace_service.rs`:

```rust
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait};
use crate::entities::{workspace, prelude::*};
use crate::error::{GitAutoDevError, Result};

#[derive(Clone)]
pub struct WorkspaceService {
    db: DatabaseConnection,
}

impl WorkspaceService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    pub async fn create_workspace(&self, repository_id: i32) -> Result<workspace::Model> {
        let workspace = workspace::ActiveModel {
            repository_id: Set(repository_id),
            workspace_status: Set("Initializing".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
            ..Default::default()
        };
        
        let workspace = Workspace::insert(workspace)
            .exec_with_returning(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))?;
        
        Ok(workspace)
    }
}

#[cfg(test)]
mod tests {
    // ... tests from Step 1
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_create_workspace_success`

Expected: PASS

**Step 5: Register service in mod.rs**

Modify `backend/src/services/mod.rs`:

```rust
pub mod workspace_service;

pub use workspace_service::WorkspaceService;
```

**Step 6: Commit**

```bash
git add backend/src/services/
git commit -m "feat(service): add WorkspaceService with create operation"
```

---

## Task 2: Add WorkspaceService read operations

**Files:**
- Modify: `backend/src/services/workspace_service.rs`

**Context:** Add methods to retrieve workspaces by ID and list all workspaces.

**Step 1: Write failing tests**

Add to tests module in `workspace_service.rs`:

```rust
#[tokio::test]
async fn test_get_workspace_by_id_success() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    
    // Create test repository and workspace
    let repo = create_test_repository(db).await;
    let service = WorkspaceService::new(db.clone());
    let created = service.create_workspace(repo.id).await.unwrap();
    
    // Act
    let result = service.get_workspace_by_id(created.id).await;
    
    // Assert
    assert!(result.is_ok());
    let workspace = result.unwrap();
    assert_eq!(workspace.id, created.id);
    assert_eq!(workspace.repository_id, repo.id);
}

#[tokio::test]
async fn test_get_workspace_by_id_not_found() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = WorkspaceService::new(db.clone());
    
    // Act
    let result = service.get_workspace_by_id(99999).await;
    
    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        GitAutoDevError::NotFound(_) => {},
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_list_workspaces_success() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = WorkspaceService::new(db.clone());
    
    // Create multiple workspaces
    let repo1 = create_test_repository(db).await;
    let repo2 = create_test_repository(db).await;
    service.create_workspace(repo1.id).await.unwrap();
    service.create_workspace(repo2.id).await.unwrap();
    
    // Act
    let result = service.list_workspaces().await;
    
    // Assert
    assert!(result.is_ok());
    let workspaces = result.unwrap();
    assert!(workspaces.len() >= 2);
}

// Helper function
async fn create_test_repository(db: &DatabaseConnection) -> repository::Model {
    use crate::entities::repository;
    let repo = repository::ActiveModel {
        name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
        full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
        clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
        default_branch: Set("main".to_string()),
        provider_id: Set(1),
        ..Default::default()
    };
    Repository::insert(repo).exec_with_returning(db).await.unwrap()
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test workspace_service`

Expected: FAIL with "method not found" errors

**Step 3: Implement read methods**

Add to `WorkspaceService` impl:

```rust
pub async fn get_workspace_by_id(&self, id: i32) -> Result<workspace::Model> {
    Workspace::find_by_id(id)
        .one(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))?
        .ok_or_else(|| GitAutoDevError::NotFound(format!("Workspace with id {} not found", id)))
}

pub async fn list_workspaces(&self) -> Result<Vec<workspace::Model>> {
    Workspace::find()
        .all(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test workspace_service`

Expected: All tests PASS

**Step 5: Commit**

```bash
git add backend/src/services/workspace_service.rs
git commit -m "feat(service): add WorkspaceService read operations"
```

---

## Task 3: Add WorkspaceService update and delete operations

**Files:**
- Modify: `backend/src/services/workspace_service.rs`

**Context:** Add methods to update workspace configuration and soft delete workspaces.

**Step 1: Write failing tests**

Add to tests module:

```rust
#[tokio::test]
async fn test_update_workspace_success() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = WorkspaceService::new(db.clone());
    let repo = create_test_repository(db).await;
    let workspace = service.create_workspace(repo.id).await.unwrap();
    
    // Act
    let result = service.update_workspace_status(workspace.id, "Active").await;
    
    // Assert
    assert!(result.is_ok());
    let updated = result.unwrap();
    assert_eq!(updated.workspace_status, "Active");
}

#[tokio::test]
async fn test_soft_delete_workspace_success() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = WorkspaceService::new(db.clone());
    let repo = create_test_repository(db).await;
    let workspace = service.create_workspace(repo.id).await.unwrap();
    
    // Act
    let result = service.soft_delete_workspace(workspace.id).await;
    
    // Assert
    assert!(result.is_ok());
    let deleted = result.unwrap();
    assert!(deleted.deleted_at.is_some());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test workspace_service`

Expected: FAIL

**Step 3: Implement update and delete methods**

Add to `WorkspaceService` impl:

```rust
use sea_orm::{ActiveValue::NotSet, QueryFilter, ColumnTrait};
use chrono::Utc;

pub async fn update_workspace_status(&self, id: i32, status: &str) -> Result<workspace::Model> {
    let workspace = self.get_workspace_by_id(id).await?;
    
    let mut workspace: workspace::ActiveModel = workspace.into();
    workspace.workspace_status = Set(status.to_string());
    workspace.updated_at = Set(Utc::now().naive_utc());
    
    let workspace = workspace.update(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))?;
    
    Ok(workspace)
}

pub async fn soft_delete_workspace(&self, id: i32) -> Result<workspace::Model> {
    let workspace = self.get_workspace_by_id(id).await?;
    
    let mut workspace: workspace::ActiveModel = workspace.into();
    workspace.deleted_at = Set(Some(Utc::now().naive_utc()));
    workspace.updated_at = Set(Utc::now().naive_utc());
    
    let workspace = workspace.update(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))?;
    
    Ok(workspace)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test workspace_service`

Expected: All tests PASS

**Step 5: Commit**

```bash
git add backend/src/services/workspace_service.rs
git commit -m "feat(service): add WorkspaceService update and delete operations"
```

---

## Task 4: Create AgentService with basic CRUD

**Files:**
- Create: `backend/src/services/agent_service.rs`
- Modify: `backend/src/services/mod.rs`

**Context:** AgentService handles AI agent configuration management.

**Step 1: Write failing test**

Create `backend/src/services/agent_service.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::db::TestDatabase;
    use crate::entities::prelude::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_create_agent_success() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = test_db.connection();
        
        // Create workspace
        let workspace = create_test_workspace(db).await;
        let service = AgentService::new(db.clone());
        
        let env_vars = json!({"API_KEY": "test-key"});
        
        // Act
        let result = service.create_agent(
            workspace.id,
            "OpenCode Primary",
            "opencode",
            "opencode --model claude-3.5",
            env_vars,
            1800,
        ).await;
        
        // Assert
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.workspace_id, workspace.id);
        assert_eq!(agent.name, "OpenCode Primary");
        assert_eq!(agent.tool_type, "opencode");
        assert_eq!(agent.enabled, true);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_create_agent_success`

Expected: FAIL

**Step 3: Implement minimal AgentService**

Add to `backend/src/services/agent_service.rs`:

```rust
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait};
use crate::entities::{agent, prelude::*};
use crate::error::{GitAutoDevError, Result};
use serde_json::Value as JsonValue;

#[derive(Clone)]
pub struct AgentService {
    db: DatabaseConnection,
}

impl AgentService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    pub async fn create_agent(
        &self,
        workspace_id: i32,
        name: &str,
        tool_type: &str,
        command: &str,
        env_vars: JsonValue,
        timeout: i32,
    ) -> Result<agent::Model> {
        let agent = agent::ActiveModel {
            workspace_id: Set(workspace_id),
            name: Set(name.to_string()),
            tool_type: Set(tool_type.to_string()),
            command: Set(command.to_string()),
            env_vars: Set(env_vars),
            timeout: Set(timeout),
            enabled: Set(true),
            ..Default::default()
        };
        
        let agent = Agent::insert(agent)
            .exec_with_returning(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))?;
        
        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    // ... tests from Step 1
    
    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        use crate::entities::workspace;
        let repo = create_test_repository(db).await;
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
            workspace_status: Set("Active".to_string()),
            ..Default::default()
        };
        Workspace::insert(ws).exec_with_returning(db).await.unwrap()
    }
    
    async fn create_test_repository(db: &DatabaseConnection) -> repository::Model {
        // Same as WorkspaceService helper
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_create_agent_success`

Expected: PASS

**Step 5: Register service**

Modify `backend/src/services/mod.rs`:

```rust
pub mod agent_service;

pub use agent_service::AgentService;
```

**Step 6: Commit**

```bash
git add backend/src/services/
git commit -m "feat(service): add AgentService with create operation"
```

---

## Task 5: Add AgentService read, update, delete operations

**Files:**
- Modify: `backend/src/services/agent_service.rs`

**Context:** Complete AgentService with full CRUD operations.

**Step 1: Write failing tests**

Add to tests module:

```rust
#[tokio::test]
async fn test_list_agents_by_workspace() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = AgentService::new(db.clone());
    let workspace = create_test_workspace(db).await;
    
    // Create multiple agents
    service.create_agent(workspace.id, "Agent 1", "opencode", "cmd1", json!({}), 1800).await.unwrap();
    service.create_agent(workspace.id, "Agent 2", "aider", "cmd2", json!({}), 1800).await.unwrap();
    
    // Act
    let result = service.list_agents_by_workspace(workspace.id).await;
    
    // Assert
    assert!(result.is_ok());
    let agents = result.unwrap();
    assert_eq!(agents.len(), 2);
}

#[tokio::test]
async fn test_update_agent_enabled() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = AgentService::new(db.clone());
    let workspace = create_test_workspace(db).await;
    let agent = service.create_agent(workspace.id, "Test", "opencode", "cmd", json!({}), 1800).await.unwrap();
    
    // Act
    let result = service.update_agent_enabled(agent.id, false).await;
    
    // Assert
    assert!(result.is_ok());
    let updated = result.unwrap();
    assert_eq!(updated.enabled, false);
}

#[tokio::test]
async fn test_delete_agent() {
    // Arrange
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = AgentService::new(db.clone());
    let workspace = create_test_workspace(db).await;
    let agent = service.create_agent(workspace.id, "Test", "opencode", "cmd", json!({}), 1800).await.unwrap();
    
    // Act
    let result = service.delete_agent(agent.id).await;
    
    // Assert
    assert!(result.is_ok());
    
    // Verify deleted
    let get_result = service.get_agent_by_id(agent.id).await;
    assert!(get_result.is_err());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test agent_service`

Expected: FAIL

**Step 3: Implement methods**

Add to `AgentService` impl:

```rust
use sea_orm::QueryFilter;

pub async fn get_agent_by_id(&self, id: i32) -> Result<agent::Model> {
    Agent::find_by_id(id)
        .one(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))?
        .ok_or_else(|| GitAutoDevError::NotFound(format!("Agent with id {} not found", id)))
}

pub async fn list_agents_by_workspace(&self, workspace_id: i32) -> Result<Vec<agent::Model>> {
    Agent::find()
        .filter(agent::Column::WorkspaceId.eq(workspace_id))
        .all(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))
}

pub async fn update_agent_enabled(&self, id: i32, enabled: bool) -> Result<agent::Model> {
    let agent = self.get_agent_by_id(id).await?;
    
    let mut agent: agent::ActiveModel = agent.into();
    agent.enabled = Set(enabled);
    agent.updated_at = Set(Utc::now().naive_utc());
    
    let agent = agent.update(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))?;
    
    Ok(agent)
}

pub async fn delete_agent(&self, id: i32) -> Result<()> {
    let agent = self.get_agent_by_id(id).await?;
    
    let agent: agent::ActiveModel = agent.into();
    agent.delete(&self.db)
        .await
        .map_err(|e| GitAutoDevError::Database(e))?;
    
    Ok(())
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test agent_service`

Expected: All tests PASS

**Step 5: Commit**

```bash
git add backend/src/services/agent_service.rs
git commit -m "feat(service): add AgentService full CRUD operations"
```

---

## Task 6: Run all tests to verify baseline

**Files:**
- None (verification only)

**Context:** Ensure all tests still pass after adding service layer.

**Step 1: Run unit tests**

Run: `cargo test --lib`

Expected: All tests pass (151+ original + new service tests)

**Step 2: Run clippy**

Run: `cargo clippy`

Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt`

**Step 4: Commit if needed**

```bash
git add -u
git commit -m "style: format code"
```

---

## Summary

**Phase 2 Complete:** Service layer for Workspace and Agent modules

**What we built:**
- WorkspaceService with full CRUD operations
- AgentService with full CRUD operations
- Comprehensive unit tests for all service methods
- Error handling with GitAutoDevError

**What's next (Phase 3):**
- TaskService implementation
- API handlers for Workspace and Agent
- Integration tests

**Verification:**
- All service methods tested
- All existing tests still pass
- No clippy warnings
- Code properly formatted
