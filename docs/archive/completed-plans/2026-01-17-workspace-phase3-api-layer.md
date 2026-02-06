# Workspace Phase 3 Implementation Plan - API Layer

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement API layer for Workspace module (HTTP handlers, routes, request/response models, OpenAPI docs)

**Architecture:** RESTful API following GitAutoDev's layered design (API → Service → Data). Handlers are thin, delegating to service layer. Full OpenAPI documentation with utoipa.

**Tech Stack:** 
- Axum 0.7 (Web framework)
- utoipa 4.x (OpenAPI documentation)
- serde (Serialization)

**Reference:** 
- `docs/plans/2026-01-17-workspace-concept-design.md`
- `docs/plans/2026-01-17-workspace-phase2-service-layer.md`

**Prerequisites:** Phase 1 & 2 completed (entities and services exist)

---

## Task 1: Create Workspace API models

**Files:**
- Create: `backend/src/api/workspaces/models.rs`
- Create: `backend/src/api/workspaces/mod.rs`

**Context:** Define request/response models for Workspace API with OpenAPI schemas.

**Step 1: Write the models file**

Create `backend/src/api/workspaces/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response model for workspace
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceResponse {
    pub id: i32,
    pub repository_id: i32,
    pub workspace_status: String,
    pub container_id: Option<String>,
    pub container_status: Option<String>,
    pub image_source: String,
    pub custom_dockerfile_path: Option<String>,
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

/// Request model for creating workspace
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceRequest {
    pub repository_id: i32,
    #[serde(default = "default_image_source")]
    pub image_source: String,
    pub custom_dockerfile_path: Option<String>,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: i32,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_memory_limit")]
    pub memory_limit: String,
    #[serde(default = "default_disk_limit")]
    pub disk_limit: String,
}

fn default_image_source() -> String {
    "default".to_string()
}

fn default_max_concurrent_tasks() -> i32 {
    3
}

fn default_cpu_limit() -> f64 {
    2.0
}

fn default_memory_limit() -> String {
    "4GB".to_string()
}

fn default_disk_limit() -> String {
    "10GB".to_string()
}

/// Request model for updating workspace status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkspaceStatusRequest {
    pub status: String,
}

impl From<crate::entities::workspace::Model> for WorkspaceResponse {
    fn from(model: crate::entities::workspace::Model) -> Self {
        Self {
            id: model.id,
            repository_id: model.repository_id,
            workspace_status: model.workspace_status,
            container_id: model.container_id,
            container_status: model.container_status,
            image_source: model.image_source,
            custom_dockerfile_path: model.custom_dockerfile_path,
            max_concurrent_tasks: model.max_concurrent_tasks,
            cpu_limit: model.cpu_limit,
            memory_limit: model.memory_limit,
            disk_limit: model.disk_limit,
            work_dir: model.work_dir,
            health_status: model.health_status,
            last_health_check: model.last_health_check.map(|dt| dt.to_string()),
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
            deleted_at: model.deleted_at.map(|dt| dt.to_string()),
        }
    }
}
```

**Step 2: Create module file**

Create `backend/src/api/workspaces/mod.rs`:

```rust
pub mod models;

pub use models::*;
```

**Step 3: Register in api/mod.rs**

Modify `backend/src/api/mod.rs`, add:

```rust
pub mod workspaces;
```

**Step 4: Verify compilation**

Run: `cd backend && cargo build`

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add backend/src/api/workspaces/
git commit -m "feat(api): add workspace API models"
```

---

## Task 2: Create Workspace API handlers

**Files:**
- Create: `backend/src/api/workspaces/handlers.rs`
- Modify: `backend/src/api/workspaces/mod.rs`

**Context:** Implement HTTP handlers for workspace CRUD operations.

**Step 1: Write handlers file**

Create `backend/src/api/workspaces/handlers.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    api::workspaces::models::*,
    error::{GitAutoDevError, Result},
    services::WorkspaceService,
    state::AppState,
};

/// Create a new workspace
#[utoipa::path(
    post,
    path = "/api/workspaces",
    request_body = CreateWorkspaceRequest,
    responses(
        (status = 201, description = "Workspace created successfully", body = WorkspaceResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>)> {
    let service = WorkspaceService::new(state.db_pool.connection());
    
    let workspace = service.create_workspace(req.repository_id).await?;
    
    Ok((StatusCode::CREATED, Json(workspace.into())))
}

/// Get workspace by ID
#[utoipa::path(
    get,
    path = "/api/workspaces/{id}",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace found", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn get_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<WorkspaceResponse>> {
    let service = WorkspaceService::new(state.db_pool.connection());
    
    let workspace = service.get_workspace_by_id(id).await?;
    
    Ok(Json(workspace.into()))
}

/// List all workspaces
#[utoipa::path(
    get,
    path = "/api/workspaces",
    responses(
        (status = 200, description = "List of workspaces", body = Vec<WorkspaceResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn list_workspaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorkspaceResponse>>> {
    let service = WorkspaceService::new(state.db_pool.connection());
    
    let workspaces = service.list_workspaces().await?;
    
    let responses: Vec<WorkspaceResponse> = workspaces
        .into_iter()
        .map(|w| w.into())
        .collect();
    
    Ok(Json(responses))
}

/// Update workspace status
#[utoipa::path(
    patch,
    path = "/api/workspaces/{id}/status",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    request_body = UpdateWorkspaceStatusRequest,
    responses(
        (status = 200, description = "Workspace status updated", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn update_workspace_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWorkspaceStatusRequest>,
) -> Result<Json<WorkspaceResponse>> {
    let service = WorkspaceService::new(state.db_pool.connection());
    
    let workspace = service.update_workspace_status(id, &req.status).await?;
    
    Ok(Json(workspace.into()))
}

/// Soft delete workspace
#[utoipa::path(
    delete,
    path = "/api/workspaces/{id}",
    params(
        ("id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace deleted", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "workspaces"
)]
pub async fn delete_workspace(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<WorkspaceResponse>> {
    let service = WorkspaceService::new(state.db_pool.connection());
    
    let workspace = service.soft_delete_workspace(id).await?;
    
    Ok(Json(workspace.into()))
}
```

**Step 2: Update mod.rs**

Modify `backend/src/api/workspaces/mod.rs`:

```rust
pub mod handlers;
pub mod models;

pub use handlers::*;
pub use models::*;
```

**Step 3: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 4: Commit**

```bash
git add backend/src/api/workspaces/
git commit -m "feat(api): add workspace API handlers"
```

---

## Task 3: Create Workspace API routes

**Files:**
- Create: `backend/src/api/workspaces/routes.rs`
- Modify: `backend/src/api/workspaces/mod.rs`
- Modify: `backend/src/api/mod.rs`

**Context:** Define routes and register them in the main router.

**Step 1: Write routes file**

Create `backend/src/api/workspaces/routes.rs`:

```rust
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers::*;

pub fn workspace_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/workspaces", post(create_workspace))
        .route("/api/workspaces", get(list_workspaces))
        .route("/api/workspaces/:id", get(get_workspace))
        .route("/api/workspaces/:id/status", patch(update_workspace_status))
        .route("/api/workspaces/:id", delete(delete_workspace))
}
```

**Step 2: Update mod.rs**

Modify `backend/src/api/workspaces/mod.rs`:

```rust
pub mod handlers;
pub mod models;
pub mod routes;

pub use handlers::*;
pub use models::*;
pub use routes::*;
```

**Step 3: Register routes in api/mod.rs**

Modify `backend/src/api/mod.rs`, in the `create_router` function, add:

```rust
use workspaces::workspace_routes;

// In create_router function, merge the routes:
.merge(workspace_routes())
```

**Step 4: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add backend/src/api/
git commit -m "feat(api): add workspace API routes"
```

---

## Task 4: Create Agent API models and handlers

**Files:**
- Create: `backend/src/api/agents/models.rs`
- Create: `backend/src/api/agents/handlers.rs`
- Create: `backend/src/api/agents/mod.rs`

**Context:** Implement Agent API similar to Workspace API.

**Step 1: Write models file**

Create `backend/src/api/agents/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// Response model for agent
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub name: String,
    pub tool_type: String,
    pub enabled: bool,
    pub command: String,
    pub env_vars: JsonValue,
    pub timeout: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Request model for creating agent
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    pub workspace_id: i32,
    pub name: String,
    pub tool_type: String,
    pub command: String,
    #[serde(default = "default_env_vars")]
    pub env_vars: JsonValue,
    #[serde(default = "default_timeout")]
    pub timeout: i32,
}

fn default_env_vars() -> JsonValue {
    serde_json::json!({})
}

fn default_timeout() -> i32 {
    1800 // 30 minutes
}

/// Request model for updating agent enabled status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAgentEnabledRequest {
    pub enabled: bool,
}

impl From<crate::entities::agent::Model> for AgentResponse {
    fn from(model: crate::entities::agent::Model) -> Self {
        Self {
            id: model.id,
            workspace_id: model.workspace_id,
            name: model.name,
            tool_type: model.tool_type,
            enabled: model.enabled,
            command: model.command,
            env_vars: model.env_vars,
            timeout: model.timeout,
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
        }
    }
}
```

**Step 2: Write handlers file**

Create `backend/src/api/agents/handlers.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    api::agents::models::*,
    error::Result,
    services::AgentService,
    state::AppState,
};

/// Create a new agent
#[utoipa::path(
    post,
    path = "/api/agents",
    request_body = CreateAgentRequest,
    responses(
        (status = 201, description = "Agent created successfully", body = AgentResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<AgentResponse>)> {
    let service = AgentService::new(state.db_pool.connection());
    
    let agent = service.create_agent(
        req.workspace_id,
        &req.name,
        &req.tool_type,
        &req.command,
        req.env_vars,
        req.timeout,
    ).await?;
    
    Ok((StatusCode::CREATED, Json(agent.into())))
}

/// Get agent by ID
#[utoipa::path(
    get,
    path = "/api/agents/{id}",
    params(
        ("id" = i32, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Agent found", body = AgentResponse),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<AgentResponse>> {
    let service = AgentService::new(state.db_pool.connection());
    
    let agent = service.get_agent_by_id(id).await?;
    
    Ok(Json(agent.into()))
}

/// List agents by workspace
#[utoipa::path(
    get,
    path = "/api/workspaces/{workspace_id}/agents",
    params(
        ("workspace_id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "List of agents", body = Vec<AgentResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn list_agents_by_workspace(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<i32>,
) -> Result<Json<Vec<AgentResponse>>> {
    let service = AgentService::new(state.db_pool.connection());
    
    let agents = service.list_agents_by_workspace(workspace_id).await?;
    
    let responses: Vec<AgentResponse> = agents
        .into_iter()
        .map(|a| a.into())
        .collect();
    
    Ok(Json(responses))
}

/// Update agent enabled status
#[utoipa::path(
    patch,
    path = "/api/agents/{id}/enabled",
    params(
        ("id" = i32, Path, description = "Agent ID")
    ),
    request_body = UpdateAgentEnabledRequest,
    responses(
        (status = 200, description = "Agent updated", body = AgentResponse),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn update_agent_enabled(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateAgentEnabledRequest>,
) -> Result<Json<AgentResponse>> {
    let service = AgentService::new(state.db_pool.connection());
    
    let agent = service.update_agent_enabled(id, req.enabled).await?;
    
    Ok(Json(agent.into()))
}

/// Delete agent
#[utoipa::path(
    delete,
    path = "/api/agents/{id}",
    params(
        ("id" = i32, Path, description = "Agent ID")
    ),
    responses(
        (status = 204, description = "Agent deleted"),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<StatusCode> {
    let service = AgentService::new(state.db_pool.connection());
    
    service.delete_agent(id).await?;
    
    Ok(StatusCode::NO_CONTENT)
}
```

**Step 3: Create mod.rs**

Create `backend/src/api/agents/mod.rs`:

```rust
pub mod handlers;
pub mod models;

pub use handlers::*;
pub use models::*;
```

**Step 4: Register in api/mod.rs**

Modify `backend/src/api/mod.rs`, add:

```rust
pub mod agents;
```

**Step 5: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 6: Commit**

```bash
git add backend/src/api/agents/
git commit -m "feat(api): add agent API models and handlers"
```

---

## Task 5: Create Agent API routes and update OpenAPI

**Files:**
- Create: `backend/src/api/agents/routes.rs`
- Modify: `backend/src/api/agents/mod.rs`
- Modify: `backend/src/api/mod.rs`

**Context:** Define agent routes and update OpenAPI documentation.

**Step 1: Write routes file**

Create `backend/src/api/agents/routes.rs`:

```rust
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::state::AppState;

use super::handlers::*;

pub fn agent_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/agents", post(create_agent))
        .route("/api/agents/:id", get(get_agent))
        .route("/api/workspaces/:workspace_id/agents", get(list_agents_by_workspace))
        .route("/api/agents/:id/enabled", patch(update_agent_enabled))
        .route("/api/agents/:id", delete(delete_agent))
}
```

**Step 2: Update agents/mod.rs**

Modify `backend/src/api/agents/mod.rs`:

```rust
pub mod handlers;
pub mod models;
pub mod routes;

pub use handlers::*;
pub use models::*;
pub use routes::*;
```

**Step 3: Register routes and update OpenAPI**

Modify `backend/src/api/mod.rs`:

```rust
use agents::agent_routes;

// In create_router:
.merge(agent_routes())

// In OpenApiDoc, add to components(schemas(...)):
crate::api::workspaces::WorkspaceResponse,
crate::api::workspaces::CreateWorkspaceRequest,
crate::api::workspaces::UpdateWorkspaceStatusRequest,
crate::api::agents::AgentResponse,
crate::api::agents::CreateAgentRequest,
crate::api::agents::UpdateAgentEnabledRequest,

// Add to paths:
crate::api::workspaces::handlers::create_workspace,
crate::api::workspaces::handlers::get_workspace,
crate::api::workspaces::handlers::list_workspaces,
crate::api::workspaces::handlers::update_workspace_status,
crate::api::workspaces::handlers::delete_workspace,
crate::api::agents::handlers::create_agent,
crate::api::agents::handlers::get_agent,
crate::api::agents::handlers::list_agents_by_workspace,
crate::api::agents::handlers::update_agent_enabled,
crate::api::agents::handlers::delete_agent,
```

**Step 4: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add backend/src/api/
git commit -m "feat(api): add agent routes and update OpenAPI docs"
```

---

## Task 6: Run all tests and verify API

**Files:**
- None (verification only)

**Context:** Ensure everything compiles, tests pass, and API is accessible.

**Step 1: Run unit tests**

Run: `cargo test --lib`

Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`

Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt`

**Step 4: Start server and test API**

Run: `cargo run`

Then test endpoints:
- `curl http://localhost:3000/swagger-ui` - Should show Swagger UI with new endpoints
- `curl http://localhost:3000/api-docs/openapi.json` - Should include workspace and agent schemas

**Step 5: Commit if needed**

```bash
git add -u
git commit -m "style: format code"
```

---

## Summary

**Phase 3 Complete:** API layer for Workspace and Agent modules

**What we built:**
- Workspace API (5 endpoints: create, get, list, update status, delete)
- Agent API (5 endpoints: create, get, list by workspace, update enabled, delete)
- Request/response models with OpenAPI schemas
- Full OpenAPI documentation in Swagger UI

**What's next:**
- Integration tests for API endpoints
- Docker integration (Phase 4)
- Task API implementation

**Verification:**
- All endpoints registered
- OpenAPI docs updated
- All tests pass
- No clippy warnings
- Server starts successfully
