# Workspace Phase 5 Implementation Plan - Docker Integration

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Docker integration for Workspace module (container management, health checks, resource limits)

**Architecture:** Use bollard crate to interact with Docker API. Implement DockerService to manage container lifecycle. Integrate with WorkspaceService for automatic container management.

**Tech Stack:** 
- bollard 0.16 (Docker API client)
- Tokio (Async runtime)
- Docker Engine (required on host)
- SeaORM (for workspace data)

**Reference:** 
- `docs/plans/2026-01-17-workspace-concept-design.md`
- Previous phase implementations
- Bollard documentation: https://docs.rs/bollard/

**Prerequisites:** Phase 1-4 completed (database, services, APIs, integration tests)

---

## Task 1: Add bollard dependency and create DockerService

**Files:**
- Modify: `backend/Cargo.toml`
- Create: `backend/src/services/docker_service.rs`
- Modify: `backend/src/services/mod.rs`

**Context:** Add bollard crate and create basic DockerService structure.

**Step 1: Add bollard to Cargo.toml**

Modify `backend/Cargo.toml`, add to dependencies:

```toml
bollard = "0.16"
```

**Step 2: Create DockerService with basic connection**

Create `backend/src/services/docker_service.rs`:

```rust
use bollard::Docker;
use crate::error::{GitAutoDevError, Result};

#[derive(Clone)]
pub struct DockerService {
    docker: Option<Docker>,
}

impl DockerService {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to connect to Docker: {}", e)))?;
        
        Ok(Self { docker: Some(docker) })
    }
    
    pub async fn ping(&self) -> Result<bool> {
        let docker = self.docker.as_ref().ok_or_else(|| 
            GitAutoDevError::Internal("Docker service not available".to_string()))?;
        
        docker.ping()
            .await
            .map(|_| true)
            .map_err(|e| GitAutoDevError::Internal(format!("Docker ping failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_service_new_success() {
        let result = DockerService::new();
        
        // This test requires Docker to be running
        if result.is_err() {
            eprintln!("Skipping test: Docker not available");
            return;
        }
        
        let service = result.unwrap();
        assert!(service.ping().await.is_ok());
    }
}
```

**Step 3: Register service in mod.rs**

Modify `backend/src/services/mod.rs`:

```rust
pub mod docker_service;

pub use docker_service::DockerService;
```

**Step 4: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 5: Run test**

Run: `cargo test test_docker_service_new_success`

Expected: PASS (if Docker is running) or SKIP (if Docker not available)

**Step 6: Commit**

```bash
git add backend/Cargo.toml backend/src/services/
git commit -m "feat(docker): add DockerService with basic connection"
```

---

## Task 2: Implement container creation with configuration

**Files:**
- Modify: `backend/src/services/docker_service.rs`

**Context:** Add method to create Docker containers with proper configuration and resource limits.

**Step 1: Write failing test**

Add to tests module:

```rust
#[tokio::test]
async fn test_create_container_success() {
    let service = match DockerService::new() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Skipping test: Docker not available");
            return;
        }
    };
    
    let result = service.create_container(
        "test-workspace-1",
        "alpine:latest",
        vec!["/workspace".to_string()],
        2.0,
        "4GB",
    ).await;
    
    assert!(result.is_ok());
    let container_id = result.unwrap();
    assert!(!container_id.is_empty());
    
    // Cleanup
    let _ = service.remove_container(&container_id, true).await;
}
```

**Step 2: Implement create_container method**

Add to `DockerService` impl:

```rust
use bollard::container::{Config, CreateContainerOptions, Mount, MountTypeEnum};
use std::collections::HashMap;

pub async fn create_container(
    &self,
    name: &str,
    image: &str,
    volumes: Vec<String>,
    cpu_limit: f64,
    memory_limit: &str,
) -> Result<String> {
    let docker = self.docker.as_ref().ok_or_else(|| 
        GitAutoDevError::Internal("Docker service not available".to_string()))?;
    
    // Parse memory limit (e.g., "4GB" -> bytes)
    let memory_bytes = parse_memory_limit(memory_limit)?;
    
    // Create mounts for volumes
    let mounts: Vec<Mount> = volumes
        .iter()
        .map(|v| Mount {
            target: Some(v.clone()),
            source: Some(format!("/tmp/gitautodev/{}", name)),
            typ: Some(MountTypeEnum::BIND),
            ..Default::default()
        })
        .collect();
    
    let host_config = HostConfig {
        mounts: Some(mounts),
        memory: Some(memory_bytes),
        nano_cpus: Some((cpu_limit * 1_000_000_000.0) as i64),
        ..Default::default()
    };
    
    let config = Config {
        image: Some(image.to_string()),
        host_config: Some(host_config),
        cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
        ..Default::default()
    };
    
    let options = CreateContainerOptions {
        name: name.to_string(),
        ..Default::default()
    };
    
    let response = docker
        .create_container(Some(options), config)
        .await
        .map_err(|e| GitAutoDevError::Internal(format!("Failed to create container: {}", e)))?;
    
    Ok(response.id)
}

fn parse_memory_limit(limit: &str) -> Result<i64> {
    let limit = limit.to_uppercase();
    
    if let Some(gb) = limit.strip_suffix("GB") {
        let value = gb.parse()
            .map_err(|_| GitAutoDevError::Validation("Invalid memory limit format".to_string()))?;
        Ok((value * 1024.0 * 1024.0 * 1024.0) as i64)
    } else if let Some(mb) = limit.strip_suffix("MB") {
        let value = mb.parse()
            .map_err(|_| GitAutoDevError::Validation("Invalid memory limit format".to_string()))?;
        Ok((value * 1024.0 * 1024.0) as i64)
    } else {
        Err(GitAutoDevError::Validation("Memory limit must end with GB or MB".to_string()))
    }
}
```

**Step 3: Run test**

Run: `cargo test test_create_container_success`

Expected: PASS

**Step 4: Commit**

```bash
git add backend/src/services/docker_service.rs
git commit -m "feat(docker): add container creation with resource limits"
```

---

## Task 3: Implement container lifecycle management

**Files:**
- Modify: `backend/src/services/docker_service.rs`

**Context:** Add methods to start, stop, and inspect containers.

**Step 1: Write failing tests**

Add to tests module:

```rust
#[tokio::test]
async fn test_start_stop_container() {
    let service = match DockerService::new() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Skipping test: Docker not available");
            return;
        }
    };
    
    let container_id = service.create_container(
        "test-lifecycle",
        "alpine:latest",
        vec![],
        2.0,
        "1GB",
    ).await.unwrap();
    
    // Start container
    let start_result = service.start_container(&container_id).await;
    assert!(start_result.is_ok());
    
    // Check status
    let status = service.get_container_status(&container_id).await.unwrap();
    assert_eq!(status, "running");
    
    // Stop container
    let stop_result = service.stop_container(&container_id, 10).await;
    assert!(stop_result.is_ok());
    
    // Check status
    let status = service.get_container_status(&container_id).await.unwrap();
    assert_eq!(status, "exited" || status == "stopped");
    
    // Cleanup
    let _ = service.remove_container(&container_id, true).await;
}
```

**Step 2: Implement lifecycle methods**

Add to `DockerService` impl:

```rust
use bollard::container::{StartContainerOptions, StopContainerOptions, InspectContainerOptions};

pub async fn start_container(&self, container_id: &str) -> Result<()> {
    let docker = self.docker.as_ref().ok_or_else(|| 
        GitAutoDevError::Internal("Docker service not available".to_string()))?;
    
    let options = StartContainerOptions {
        ..Default::default()
    };
    
    docker
        .start_container(container_id, Some(options))
        .await
        .map_err(|e| GitAutoDevError::Internal(format!("Failed to start container: {}", e)))?;

    Ok(())
}

pub async fn stop_container(&self, container_id: &str, timeout: i64) -> Result<()> {
    let docker = self.docker.as_ref().ok_or_else(|| 
        GitAutoDevError::Internal("Docker service not available".to_string()))?;
    
    let options = StopContainerOptions {
        t: Some(timeout),
        ..Default::default()
    };
    
    docker
        .stop_container(container_id, Some(options))
        .await
        .map_err(|e| GitAutoDevError::Internal(format!("Failed to stop container: {}", e)))?;

    Ok(())
}

pub async fn get_container_status(&self, container_id: &str) -> Result<ContainerHealth> {
    let docker = self.docker.as_ref().ok_or_else(|| 
        GitAutoDevError::Internal("Docker service not available".to_string()))?;
    
    let inspect = docker
        .inspect_container(container_id, None::<InspectContainerOptions>())
        .await
        .map_err(|e| GitAutoDevError::Internal(format!("Failed to inspect container: {}", e)))?;
    
    let state = inspect.state.ok_or_else(|| 
        GitAutoDevError::Internal("Container state not available".to_string()))?;
    
    let is_running = state.running.unwrap_or(false);
    let status = state.status
        .map(|s| format!("{:?}", s).to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());
    
    Ok(ContainerHealth {
        is_running,
        status,
        exit_code: state.exit_code,
        error: state.error,
    })
}

pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
    let docker = self.docker.as_ref().ok_or_else(|| 
        GitAutoDevError::Internal("Docker service not available".to_string()))?;
    
    let options = bollard::container::RemoveContainerOptions {
        force,
        ..Default::default()
    };
    
    docker
        .remove_container(container_id, Some(options))
        .await
        .map_err(|e| GitAutoDevError::Internal(format!("Failed to remove container: {}", e)))?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct ContainerHealth {
    pub is_running: bool,
    pub status: String,
    pub exit_code: Option<i64>,
    pub error: Option<String>,
}
```

**Step 3: Run tests**

Run: `cargo test docker_service`

Expected: All tests PASS

**Step 4: Commit**

```bash
git add backend/src/services/docker_service.rs
git commit -m "feat(docker): add container lifecycle management"
```

---

## Task 4: Integrate DockerService with WorkspaceService

**Files:**
- Modify: `backend/src/services/workspace_service.rs`
- Modify: `backend/src/state.rs`

**Context:** Integrate Docker container management into workspace lifecycle.

**Step 1: Add DockerService to WorkspaceService**

Modify `backend/src/services/workspace_service.rs`:

Add field to struct:

```rust
use crate::services::DockerService;

pub struct WorkspaceService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}
```

Update `new` method:

```rust
pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }
```

**Step 2: Update create_workspace to create container**

Add to `create_workspace` method in `WorkspaceService`:

```rust
pub async fn create_workspace(
    &self,
    repository_id: i32,
    image_source: &str,
    custom_dockerfile_path: Option<&str>,
    max_concurrent_tasks: i32,
    cpu_limit: f64,
    memory_limit: &str,
) -> Result<workspace::Model> {
    // Create workspace record
    let workspace = self.create_workspace_db(repository_id).await?;
    
    // Create Docker container if available
    if let Some(docker) = &self.docker {
        let container_name = format!("workspace-{}", workspace.id);
        
        let volumes = vec![
            format!("/tmp/gitautodev/{}:/workspace", container_name),
        ];
        
        let cpu_limit = workspace.cpu_limit;
        let memory_limit = &workspace.memory_limit;
        
        match docker.create_container(
            &container_name,
            image_source,
            custom_dockerfile_path,
            volumes,
            cpu_limit,
            memory_limit,
        ).await {
            Ok(container_id) => {
                // Update workspace with container info
                let mut ws: workspace::ActiveModel = workspace.into();
                ws.container_id = Set(Some(container_id));
                ws.container_status = Set(Some("running".to_string()));
                ws.workspace_status = Set("Active".to_string());
                ws.last_health_check = Set(Some(chrono::Utc::now().naive_utc()));
                ws.updated_at = Set(chrono::Utc::now().naive_utc());
                
                ws.update(&self.db).await
                    .map_err(crate::error::GitAutoDevError::Database)?;
                
                Ok(ws)
            },
            Err(e) => {
                tracing::error!("Failed to create container for workspace: {}", e);
                Err(e)
            },
        }
    } else {
        // Mark workspace as no Docker support
        tracing::warn!("Docker service not available, workspace created without container");
        Ok(workspace)
    }
}
```

**Step 3: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 4: Commit**

```bash
git add backend/src/services/
git commit -m "feat(workspace): integrate Docker container creation"
```

---

## Task 5: Implement container health checks

**Files:**
- Create: `backend/src/services/health_check_service.rs`
- Modify: `backend/src/services/mod.rs`

**Context:** Implement health check service to monitor container status.

**Step 1: Create HealthCheckService**

Create `backend/src/services/health_check_service.rs`:

```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use crate::entities::{workspace, prelude::*};
use crate::services::DockerService;
use crate::error::Result;

pub struct HealthCheckService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
    interval: Duration,
}

impl HealthCheckService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self {
            db,
            docker,
            interval: Duration::from_secs(300), // 5 minutes
        }
    }
    
    pub async fn run(&self) {
        let mut interval = time::interval(self.interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_all_workspaces().await {
                tracing::error!("Health check failed: {}", e);
            }
        }
    }
    
    async fn check_all_workspaces(&self) -> Result<()> {
        let docker = match &self.docker {
            Some(d) => d,
            None => return Ok(()), // Skip if Docker not available
        };
        
        // Get all active workspaces
        let workspaces = Workspace::find()
            .all(&self.db)
            .await
            .map_err(crate::error::GitAutoDevError::Database)?;
        
        for workspace in workspaces {
            if let Some(container_id) = &workspace.container_id {
                match docker.check_container_health(container_id).await {
                    Ok(health) => {
                        // Update workspace health status
                        let mut ws: workspace::ActiveModel = workspace.into();
                        ws.health_status = Set(Some(if health.is_running {
                            "Healthy".to_string()
                        } else {
                            "Unhealthy".to_string()
                        }));
                        ws.last_health_check = Set(Some(chrono::Utc::now().naive_utc()));
                        ws.container_status = Set(Some(health.status.clone()));
                        
                        if let Err(e) = ws.update(&self.db).await {
                            tracing::error!("Failed to update workspace health: {}", e);
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to check container health: {}", e);
                        // Mark workspace as unhealthy
                        let mut ws: workspace::ActiveModel = workspace.into();
                        ws.health_status = Set(Some("Unhealthy".to_string()));
                        ws.container_status = Set(Some("Health Check Failed".to_string()));
                        ws.last_health_check = Set(Some(chrono::Utc::now().naive_utc()));
                        
                        if let Err(e) = ws.update(&self.db).await {
                            tracing::error!("Failed to update workspace health: {}", e);
                        }
                    },
                }
            }
        }
        
        Ok(())
    }
}
```

**Step 2: Register service in mod.rs**

Modify `backend/src/services/mod.rs`:

```rust
pub mod health_check_service;

pub use health_check_service::HealthCheckService;
```

**Step 3: Verify compilation**

Run: `cargo build`

Expected: Compiles successfully

**Step 4: Commit**

```bash
git add backend/src/services/
git commit -m "feat(docker): add container health check service"
```

---

## Task 6: Run all tests and verify

**Files:**
- None (verification only)

**Context:** Final verification of Docker integration.

**Step 1: Run unit tests**

Run: `cargo test --lib`

Expected: All tests pass (existing tests + new Docker tests)

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

**Phase 5 Complete:** Docker integration for Workspace module

**What we built:**
- DockerService with container lifecycle management
- Container creation with resource limits
- Start/stop/inspect operations
- HealthCheckService for monitoring containers
- Integration with WorkspaceService for automatic container management
- Graceful degradation when Docker is not available

**Features:**
- Create containers with CPU and memory limits
- Mount volumes for workspace data
- Start/stop containers with timeout
- Monitor container health (running, status, exit codes)
- Automatic workspace health status updates
- 5-minute health check interval
- Error handling and logging

**Verification:**
- All tests pass
- No clippy warnings
- Code properly formatted
- Docker integration optional (graceful degradation if Docker not available)

**What's next (Phase 6):**
- Git worktree management
- Task execution logic
- Agent tool invocation
- Error handling and retry mechanisms
- WebSocket for real-time logs
