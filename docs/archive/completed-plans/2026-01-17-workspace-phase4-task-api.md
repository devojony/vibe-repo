# Workspace Phase 4 Implementation Plan - Task API

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Task API layer (HTTP handlers, routes, request/response models, OpenAPI docs)

**Architecture:** RESTful API following the same pattern as Workspace and Agent APIs. Tasks represent development work units that execute in workspaces using agents.

**Tech Stack:** 
- Axum 0.7 (Web framework)
- utoipa 4.x (OpenAPI documentation)
- serde (Serialization)

**Reference:** 
- `docs/plans/2026-01-17-workspace-concept-design.md`
- Previous phase implementations

**Prerequisites:** Phase 1-3 completed (entities, services, and workspace/agent APIs exist)

---

## Task 1: Create TaskService with basic CRUD

**Files:**
- Create: `backend/src/services/task_service.rs`
- Modify: `backend/src/services/mod.rs`

**Context:** TaskService handles business logic for task management. Start with basic CRUD operations.

**Step 1: Write failing test for create_task**

Create `backend/src/services/task_service.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::db::TestDatabase;
    use crate::entities::prelude::*;
    use sea_orm::EntityTrait;

    #[tokio::test]
    async fn test_create_task_success() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = test_db.connection();
        
        // Create test workspace and agent
        let workspace = create_test_workspace(db).await;
        let agent = create_test_agent(db, workspace.id).await;
        
        let service = TaskService::new(db.clone());
        
        // Act
        let result = service.create_task(
            workspace.id,
            agent.id,
            Some(123),
            Some("https://git.example.com/owner/repo/issues/123".to_string()),
            "manual",
        ).await;
        
        // Assert
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.workspace_id, workspace.id);
        assert_eq!(task.agent_id, agent.id);
        assert_eq!(task.issue_id, Some(123));
        assert_eq!(task.status, "Created");
        assert_eq!(task.priority, "medium");
    }
    
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
    
    async fn create_test_agent(db: &DatabaseConnection, workspace_id: i32) -> agent::Model {
        use crate::entities::agent;
        let ag = agent::ActiveModel {
            workspace_id: Set(workspace_id),
            name: Set("Test Agent".to_string()),
            tool_type: Set("opencode".to_string()),
            command: Set("opencode".to_string()),
            env_vars: Set(serde_json::json!({})),
            timeout: Set(1800),
            ..Default::default()
        };
        Agent::insert(ag).exec_with_returning(db).await.unwrap()
    }
    
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
}
```

**Step 2: Run test to verify it fails**

Run: `cd backend && cargo test test_create_task_success`

Expected: FAIL with "TaskService not found"

**Step 3: Implement minimal TaskService**

Add to `backend/src/services/task_service.rs`:

```rust
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait};
use crate::entities::{task, prelude::*};
use crate::error::{GitAutoDevError, Result};

#[derive(Clone)]
pub struct TaskService {
    db: DatabaseConnection,
}

impl TaskService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    pub async fn create_task(
        &self,
        workspace_id: i32,
        agent_id: i32,
        issue_id: Option<i32>,
        issue_url: Option<String>,
        created_by: &str,
    ) -> Result<task::Model> {
        let task = task::ActiveModel {
            workspace_id: Set(workspace_id),
            agent_id: Set(agent_id),
            issue_id: Set(issue_id),
            issue_url: Set(issue_url),
            status: Set("Created".to_string()),
            priority: Set("medium".to_string()),
            created_by: Set(created_by.to_string()),
            retry_count: Set(0),
            ..Default::default()
        };
        
        let task = Task::insert(task)
            .exec_with_returning(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?;
        
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    // ... tests from Step 1
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_create_task_success`

Expected: PASS

**Step 5: Register service in mod.rs**

Modify `backend/src/services/mod.rs`:

```rust
pub mod task_service;

pub use task_service::TaskService;
```

**Step 6: Commit**

```bash
git add backend/src/services/
git commit -m "feat(service): add TaskService with create operation"
```

---

## Task 2: Add TaskService read and update operations

**Files:**
- Modify: `backend/src/services/task_service.rs`

**Context:** Add methods to retrieve tasks and update task status.

**Step 1: Write failing tests**

Add to tests module:

```rust
#[tokio::test]
async fn test_get_task_by_id_success() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = TaskService::new(db.clone());
    
    let workspace = create_test_workspace(db).await;
    let agent = create_test_agent(db, workspace.id).await;
    let created = service.create_task(workspace.id, agent.id, None, None, "manual").await.unwrap();
    
    let result = service.get_task_by_id(created.id).await;
    
    assert!(result.is_ok());
    let task = result.unwrap();
    assert_eq!(task.id, created.id);
}

#[tokio::test]
async fn test_list_tasks_by_workspace() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = TaskService::new(db.clone());
    
    let workspace = create_test_workspace(db).await;
    let agent = create_test_agent(db, workspace.id).await;
    
    service.create_task(workspace.id, agent.id, None, None, "manual").await.unwrap();
    service.create_task(workspace.id, agent.id, None, None, "manual").await.unwrap();
    
    let result = service.list_tasks_by_workspace(workspace.id).await;
    
    assert!(result.is_ok());
    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 2);
}

#[tokio::test]
async fn test_update_task_status() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = test_db.connection();
    let service = TaskService::new(db.clone());
    
    let workspace = create_test_workspace(db).await;
    let agent = create_test_agent(db, workspace.id).await;
    let task = service.create_task(workspace.id, agent.id, None, None, "manual").await.unwrap();
    
    let result = service.update_task_status(task.id, "Running").await;
    
    assert!(result.is_ok());
    let updated = result.unwrap();
    assert_eq!(updated.status, "Running");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test task_service`

Expected: FAIL

**Step 3: Implement methods**

Add to `TaskService` impl:

```rust
use sea_orm::{QueryFilter, ColumnTrait};
use chrono::Utc;

pub async fn get_task_by_id(&self, id: i32) -> Result<task::Model> {
    Task::find_by_id(id)
        .one(&self.db)
        .await
        .map_err(GitAutoDevError::Database)?
        .ok_or_else(|| GitAutoDevError::NotFound(format!("Task with id {} not found", id)))
}

pub async fn list_tasks_by_workspace(&self, workspace_id: i32) -> Result<Vec<task::Model>> {
    Task::find()
        .filter(task::Column::WorkspaceId.eq(workspace_id))
        .all(&self.db)
        .await
        .map_err(GitAutoDevError::Database)
}

pub async fn update_task_status(&self, id: i32, status: &str) -> Result<task::Model> {
    let task = self.get_task_by_id(id).await?;
    
    let mut task: task::ActiveModel = task.into();
    task.status = Set(status.to_string());
    task.updated_at = Set(Utc::now().naive_utc());
    
    let task = task.update(&self.db)
        .await
        .map_err(GitAutoDevError::Database)?;
    
    Ok(task)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test task_service`

Expected: All tests PASS

**Step 5: Commit**

```bash
git add backend/src/services/task_service.rs
git commit -m "feat(service): add TaskService read and update operations"
```

---

## Task 3: Create Task API models

**Files:**
- Create: `backend/src/api/tasks/models.rs`
- Create: `backend/src/api/tasks/mod.rs`

**Context:** Define request/response models for Task API.

**Step 1: Write models file**

Create `backend/src/api/tasks/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response model for task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub agent_id: i32,
    pub issue_id: Option<i32>,
    pub issue_url: Option<String>,
    pub pr_url: Option<String>,
    pub worktree_path: Option<String>,
    pub status: String,
    pub priority: String,
    pub timeout: Option<i32>,
    pub retry_count: i32,
    pub created_by: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub error_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Request model for creating task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaskRequest {
    pub workspace_id: i32,
    pub agent_id: i32,
    pub issue_id: Option<i32>,
    pub issue_url: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
    pub timeout: Option<i32>,
}

fn default_priority() -> String {
    "medium".to_string()
}

/// Request model for updating task status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTaskStatusRequest {
    pub status: String,
}

impl From<crate::entities::task::Model> for TaskResponse {
    fn from(model: crate::entities::task::Model) -> Self {
        Self {
            id: model.id,
            workspace_id: model.workspace_id,
            agent_id: model.agent_id,
            issue_id: model.issue_id,
            issue_url: model.issue_url,
            pr_url: model.pr_url,
            worktree_path: model.worktree_path,
            status: model.status,
            priority: model.priority,
            timeout: model.timeout,
            retry_count: model.retry_count,
            created_by: model.created_by,
            started_at: model.started_at.map(|dt| dt.to_string()),
            completed_at: model.completed_at.map(|dt| dt.to_string()),
            error_message: model.error_message,
            error_type: model.error_type,
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
        }
    }
}
```

**Step 2: Create module file**

Create `backend/src/api/tasks/mod.rs`:

```rust
pub mod models;

pub use models::*;
```

**Step 3: Register in api/mod.rs**

Modify `backend/src/api/mod.rs`, add:

```rust
pub mod tasks;
```

**Step 4: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add backend/src/api/tasks/
git commit -m "feat(api): add task API models"
```

---

## Task 4: Create Task API handlers and routes

**Files:**
- Create: `backend/src/api/tasks/handlers.rs`
- Create: `backend/src/api/tasks/routes.rs`
- Modify: `backend/src/api/tasks/mod.rs`
- Modify: `backend/src/api/mod.rs`

**Context:** Implement HTTP handlers and routes for task operations.

**Step 1: Write handlers file**

Create `backend/src/api/tasks/handlers.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    api::tasks::models::*,
    error::Result,
    services::TaskService,
    state::AppState,
};

/// Create a new task
#[utoipa::path(
    post,
    path = "/api/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created successfully", body = TaskResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>)> {
    let service = TaskService::new(state.db.clone());
    
    let task = service.create_task(
        req.workspace_id,
        req.agent_id,
        req.issue_id,
        req.issue_url,
        "manual",
    ).await?;
    
    Ok((StatusCode::CREATED, Json(task.into())))
}

/// Get task by ID
#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task found", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());
    
    let task = service.get_task_by_id(id).await?;
    
    Ok(Json(task.into()))
}

/// List tasks by workspace
#[utoipa::path(
    get,
    path = "/api/workspaces/{workspace_id}/tasks",
    params(
        ("workspace_id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "List of tasks", body = Vec<TaskResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn list_tasks_by_workspace(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<i32>,
) -> Result<Json<Vec<TaskResponse>>> {
    let service = TaskService::new(state.db.clone());
    
    let tasks = service.list_tasks_by_workspace(workspace_id).await?;
    
    let responses: Vec<TaskResponse> = tasks
        .into_iter()
        .map(|t| t.into())
        .collect();
    
    Ok(Json(responses))
}

/// Update task status
#[utoipa::path(
    patch,
    path = "/api/tasks/{id}/status",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = UpdateTaskStatusRequest,
    responses(
        (status = 200, description = "Task status updated", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "tasks"
)]
pub async fn update_task_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTaskStatusRequest>,
) -> Result<Json<TaskResponse>> {
    let service = TaskService::new(state.db.clone());
    
    let task = service.update_task_status(id, &req.status).await?;
    
    Ok(Json(task.into()))
}
```

**Step 2: Write routes file**

Create `backend/src/api/tasks/routes.rs`:

```rust
use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers::*;

pub fn task_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tasks", post(create_task))
        .route("/api/tasks/:id", get(get_task))
        .route("/api/workspaces/:workspace_id/tasks", get(list_tasks_by_workspace))
        .route("/api/tasks/:id/status", patch(update_task_status))
}
```

**Step 3: Update tasks/mod.rs**

Modify `backend/src/api/tasks/mod.rs`:

```rust
pub mod handlers;
pub mod models;
pub mod routes;

pub use handlers::*;
pub use models::*;
pub use routes::*;
```

**Step 4: Register routes and update OpenAPI**

Modify `backend/src/api/mod.rs`:

```rust
use tasks::task_routes;

// In create_router:
.merge(task_routes())

// In OpenApiDoc schemas:
crate::api::tasks::TaskResponse,
crate::api::tasks::CreateTaskRequest,
crate::api::tasks::UpdateTaskStatusRequest,

// In OpenApiDoc paths:
crate::api::tasks::handlers::create_task,
crate::api::tasks::handlers::get_task,
crate::api::tasks::handlers::list_tasks_by_workspace,
crate::api::tasks::handlers::update_task_status,
```

**Step 5: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 6: Commit**

```bash
git add backend/src/api/
git commit -m "feat(api): add task API handlers and routes"
```

---

## Task 5: Run all tests and verify

**Files:**
- None (verification only)

**Context:** Final verification of all implementations.

**Step 1: Run unit tests**

Run: `cargo test --lib`

Expected: All tests pass (161+ original + new task tests)

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

**Phase 4 Complete:** Task API implementation

**What we built:**
- TaskService with CRUD operations
- Task API (4 endpoints: create, get, list by workspace, update status)
- Request/response models with OpenAPI schemas
- Full OpenAPI documentation

**API Endpoints:**
- POST /api/tasks - Create task
- GET /api/tasks/:id - Get task
- GET /api/workspaces/:workspace_id/tasks - List tasks by workspace
- PATCH /api/tasks/:id/status - Update task status

**Verification:**
- All tests pass
- No clippy warnings
- Code properly formatted
- OpenAPI docs updated
