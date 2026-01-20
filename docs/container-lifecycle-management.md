# Container Lifecycle Management

**Version:** 0.3.0  
**Status:** Production Ready

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Components](#components)
- [API Endpoints](#api-endpoints)
- [Configuration](#configuration)
- [Usage Examples](#usage-examples)
- [Troubleshooting](#troubleshooting)
- [Development](#development)

---

## Overview

Container Lifecycle Management is a comprehensive system for managing Docker containers that provide isolated development environments for VibeRepo workspaces. The system handles the complete lifecycle from creation to deletion, including automatic health monitoring and recovery.

### Key Features

- **Automatic Container Creation**: Containers are created and started automatically when workspaces are initialized
- **Health Monitoring**: Continuous health checks with automatic restart on failure
- **Resource Monitoring**: Real-time CPU, memory, and network usage statistics
- **Image Management**: Build, rebuild, and manage workspace Docker images
- **Manual Control**: API endpoints for manual restart and monitoring
- **Restart Policies**: Configurable restart limits with automatic failure detection
- **Graceful Degradation**: Containers marked as failed after exceeding restart limits

### Benefits

- **Isolation**: Each workspace runs in its own container with dedicated resources
- **Reliability**: Automatic recovery from container failures
- **Observability**: Real-time metrics and status tracking
- **Flexibility**: Customizable resource limits and restart policies
- **Safety**: Conflict detection prevents accidental image deletion

---

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                     VibeRepo Backend                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  Workspace   │  │  Container   │  │    Image     │    │
│  │   Service    │──│   Service    │──│ Management   │    │
│  │              │  │              │  │   Service    │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│         │                  │                  │            │
│         └──────────────────┴──────────────────┘            │
│                            │                               │
│                   ┌────────▼────────┐                      │
│                   │ Docker Service  │                      │
│                   └────────┬────────┘                      │
│                            │                               │
│  ┌─────────────────────────▼──────────────────────────┐   │
│  │          HealthCheck Service                       │   │
│  │  (Monitors containers, triggers auto-restart)     │   │
│  └────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                   ┌────────────────┐
                   │  Docker Engine │
                   └────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   ┌─────────┐         ┌─────────┐         ┌─────────┐
   │Container│         │Container│         │Container│
   │workspace│         │workspace│         │workspace│
   │   -1    │         │   -2    │         │   -3    │
   └─────────┘         └─────────┘         └─────────┘
```

### Data Flow

1. **Workspace Creation**:
   - User creates workspace via API
   - WorkspaceService creates workspace record
   - ContainerService creates and starts container
   - Container record stored in database

2. **Health Monitoring**:
   - HealthCheckService runs periodic checks (every 30 seconds)
   - Detects unhealthy containers
   - Triggers automatic restart if within limits
   - Marks container as failed if limits exceeded

3. **Manual Operations**:
   - User triggers restart via API
   - ContainerService restarts container
   - Restart count incremented
   - Status updated in database

---

## Components

### 1. ContainerService

**Location**: `backend/src/services/container_service.rs`

Manages the complete lifecycle of Docker containers.

**Key Methods**:

- `create_and_start_container()` - Creates and starts a new container
- `manual_restart_container()` - Manually restarts a container
- `auto_restart_container()` - Automatically restarts a container (called by health checks)
- `get_container_by_workspace_id()` - Retrieves container by workspace ID
- `update_container_status()` - Updates container status in database

**Features**:

- Automatic container naming: `workspace-{workspace_id}`
- Default workspace mount: `/workspace`
- Restart count tracking
- Max restart attempts: 3 (configurable)
- Status management: creating, running, stopped, exited, failed

**Example Usage**:

```rust
let container_service = ContainerService::new(db.clone(), docker.clone());

// Create and start container
let container = container_service
    .create_and_start_container(
        workspace_id,
        "vibe-repo-workspace:latest",
        1.0,  // CPU limit
        "512m" // Memory limit
    )
    .await?;

// Manual restart
let updated = container_service
    .manual_restart_container(container.id)
    .await?;
```

### 2. ImageManagementService

**Location**: `backend/src/services/image_management_service.rs`

Manages workspace Docker images.

**Key Methods**:

- `get_image_info()` - Retrieves image metadata
- `delete_image()` - Deletes image with conflict detection
- `rebuild_image()` - Rebuilds image from Dockerfile
- `get_workspaces_using_image()` - Lists workspaces using an image

**Features**:

- Conflict detection (prevents deletion if image is in use)
- Force rebuild option
- Image size and creation time tracking
- Workspace usage tracking

**Example Usage**:

```rust
let image_service = ImageManagementService::new(db.clone(), docker.clone());

// Get image info
let info = image_service
    .get_image_info("vibe-repo-workspace:latest")
    .await?;

// Rebuild image
let result = image_service
    .rebuild_image("vibe-repo-workspace:latest", false)
    .await?;
```

### 3. DockerService Enhancements

**Location**: `backend/src/services/docker_service.rs`

Extended with 7 new methods for container and image operations.

**New Methods**:

- `image_exists()` - Check if image exists
- `build_image()` - Build Docker image from Dockerfile
- `remove_image()` - Remove Docker image
- `inspect_image()` - Get image metadata
- `list_containers_using_image()` - List containers by image
- `restart_container()` - Restart container with timeout
- `get_container_stats()` - Get real-time resource usage

**Example Usage**:

```rust
let docker = DockerService::new()?;

// Check if image exists
if !docker.image_exists("vibe-repo-workspace:latest").await? {
    // Build image
    docker.build_image(
        "docker/workspace",
        "vibe-repo-workspace:latest"
    ).await?;
}

// Get container stats
let stats = docker.get_container_stats("container-id").await?;
println!("CPU: {}%, Memory: {} MB", stats.cpu_percent, stats.memory_usage_mb);
```

### 4. WorkspaceService Updates

**Location**: `backend/src/services/workspace_service.rs`

Enhanced to integrate with container management.

**New Methods**:

- `create_workspace_with_container()` - Creates workspace with container
- `ensure_image_exists()` - Auto-builds images when needed

**Breaking Change**:

The `create_workspace_with_container()` method now returns a tuple:

```rust
// Old (v0.2.0)
let workspace = workspace_service.create_workspace_with_container(...).await?;

// New (v0.3.0)
let (workspace, container) = workspace_service.create_workspace_with_container(...).await?;
```

### 5. HealthCheckService Enhancements

**Location**: `backend/src/services/health_check_service.rs`

Enhanced with automatic container recovery.

**New Features**:

- Auto-restart unhealthy containers
- Respect max restart attempts
- Mark containers as failed after limit exceeded
- Detailed logging of restart operations

**Configuration**:

- Check interval: 30 seconds
- Max restart attempts: 3 (per container)
- Container stop timeout: 10 seconds

---

## API Endpoints

### 1. Restart Workspace Container

Manually restart a workspace container.

**Endpoint**: `POST /api/workspaces/:id/restart`

**Parameters**:
- `id` (path, integer) - Workspace ID

**Response** (200 OK):

```json
{
  "message": "Container restarted successfully",
  "workspace_id": 1,
  "container": {
    "id": 1,
    "container_id": "abc123def456",
    "status": "running",
    "restart_count": 2,
    "last_restart_at": "2026-01-20T10:30:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found` - Workspace or container not found
- `500 Internal Server Error` - Docker operation failed

**Example**:

```bash
curl -X POST http://localhost:3000/api/workspaces/1/restart
```

---

### 2. Get Container Statistics

Get real-time resource usage statistics for a workspace container.

**Endpoint**: `GET /api/workspaces/:id/stats`

**Parameters**:
- `id` (path, integer) - Workspace ID

**Response** (200 OK):

```json
{
  "workspace_id": 1,
  "container_id": "abc123def456",
  "stats": {
    "cpu_percent": 15.5,
    "memory_usage_mb": 256.8,
    "memory_limit_mb": 512.0,
    "memory_percent": 50.16,
    "network_rx_bytes": 1048576,
    "network_tx_bytes": 524288
  },
  "collected_at": "2026-01-20T10:35:00Z"
}
```

**Error Responses**:
- `404 Not Found` - Workspace or container not found
- `409 Conflict` - Container is not running
- `503 Service Unavailable` - Docker not available

**Example**:

```bash
curl http://localhost:3000/api/workspaces/1/stats
```

---

### 3. Get Workspace Image Information

Query information about the workspace Docker image.

**Endpoint**: `GET /api/settings/workspace/image`

**Response** (200 OK) - Image exists:

```json
{
  "exists": true,
  "image_name": "vibe-repo-workspace:latest",
  "image_id": "sha256:abc123...",
  "size_mb": 215.4,
  "created_at": "2026-01-20T09:00:00Z",
  "in_use_by_workspaces": 3,
  "workspace_ids": [1, 2, 3]
}
```

**Response** (200 OK) - Image does not exist:

```json
{
  "exists": false,
  "image_name": "vibe-repo-workspace:latest",
  "message": "Image 'vibe-repo-workspace:latest' does not exist. Use POST /api/settings/workspace/image/rebuild to build it."
}
```

**Example**:

```bash
curl http://localhost:3000/api/settings/workspace/image
```

---

### 4. Delete Workspace Image

Delete the workspace Docker image.

**Endpoint**: `DELETE /api/settings/workspace/image`

**Response** (200 OK) - Success:

```json
{
  "message": "Image 'vibe-repo-workspace:latest' deleted successfully",
  "image_name": "vibe-repo-workspace:latest"
}
```

**Error Response** (409 Conflict) - Image in use:

```json
{
  "error": "Cannot delete image: 3 workspace(s) are using it. Active workspace IDs: [1, 2, 3]. Stop or delete these workspaces first."
}
```

**Example**:

```bash
curl -X DELETE http://localhost:3000/api/settings/workspace/image
```

---

### 5. Rebuild Workspace Image

Rebuild the workspace Docker image from the Dockerfile.

**Endpoint**: `POST /api/settings/workspace/image/rebuild`

**Request Body**:

```json
{
  "force": false
}
```

**Parameters**:
- `force` (boolean, optional) - Force rebuild even if workspaces are using the image. Default: `false`

**Response** (200 OK) - Success:

```json
{
  "message": "Image 'vibe-repo-workspace:latest' rebuilt successfully",
  "image_name": "vibe-repo-workspace:latest",
  "image_id": "sha256:def456...",
  "build_time_seconds": 45.2,
  "size_mb": 218.7
}
```

**Response** (200 OK) - Success with force=true:

```json
{
  "message": "Image 'vibe-repo-workspace:latest' rebuilt successfully",
  "image_name": "vibe-repo-workspace:latest",
  "image_id": "sha256:def456...",
  "build_time_seconds": 45.2,
  "size_mb": 218.7,
  "active_workspace_ids": [1, 2, 3],
  "warning": "3 workspace(s) are using the old image and may need to be restarted",
  "suggestion": "Restart workspaces [1, 2, 3] to use the new image"
}
```

**Error Response** (409 Conflict) - Image in use and force=false:

```json
{
  "error": "Cannot rebuild image: 3 workspace(s) are using it. Active workspace IDs: [1, 2, 3]. Use force=true to rebuild anyway, or stop these workspaces first."
}
```

**Examples**:

```bash
# Normal rebuild (fails if image is in use)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": false}'

# Force rebuild (rebuilds even if image is in use)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": true}'
```

---

## Configuration

### Default Workspace Image

**Image Name**: `vibe-repo-workspace:latest`  
**Dockerfile**: `docker/workspace/Dockerfile`  
**Base Image**: Ubuntu 22.04 LTS

**Pre-installed Tools**:
- Git
- curl, wget
- vim, nano
- build-essential (GCC, G++, make)
- ca-certificates
- unzip, zip, jq

### Resource Limits

Default resource limits for containers:

| Resource | Default | Configurable |
|----------|---------|--------------|
| CPU | 1.0 cores | Yes (via workspace creation) |
| Memory | 512 MB | Yes (via workspace creation) |
| Disk | Unlimited | No (uses host disk) |

### Restart Policies

| Setting | Default | Description |
|---------|---------|-------------|
| Max Restart Attempts | 3 | Maximum automatic restarts before marking as failed |
| Container Stop Timeout | 10 seconds | Timeout for graceful container stop |
| Health Check Interval | 30 seconds | Frequency of health checks |

### Container Naming

Containers are automatically named using the pattern:

```
workspace-{workspace_id}
```

Example: `workspace-1`, `workspace-42`

### Workspace Mount

Containers have a default workspace mount at:

```
/workspace
```

This directory is available for storing workspace-specific files.

---

## Usage Examples

### Example 1: Creating a Workspace with Container

```bash
# Create workspace (container is created automatically)
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 1
  }'
```

**Response**:

```json
{
  "id": 1,
  "repository_id": 1,
  "workspace_status": "Active",
  "container_id": "abc123def456",
  "container_status": "running",
  "image_source": "vibe-repo-workspace:latest",
  "created_at": "2026-01-20T10:00:00Z"
}
```

---

### Example 2: Monitoring Container Health

```bash
# Get container statistics
curl http://localhost:3000/api/workspaces/1/stats
```

**Response**:

```json
{
  "workspace_id": 1,
  "container_id": "abc123def456",
  "stats": {
    "cpu_percent": 12.3,
    "memory_usage_mb": 245.6,
    "memory_limit_mb": 512.0,
    "memory_percent": 47.97,
    "network_rx_bytes": 2097152,
    "network_tx_bytes": 1048576
  },
  "collected_at": "2026-01-20T10:30:00Z"
}
```

---

### Example 3: Manually Restarting a Container

```bash
# Restart container
curl -X POST http://localhost:3000/api/workspaces/1/restart
```

**Response**:

```json
{
  "message": "Container restarted successfully",
  "workspace_id": 1,
  "container": {
    "id": 1,
    "container_id": "abc123def456",
    "status": "running",
    "restart_count": 1,
    "last_restart_at": "2026-01-20T10:35:00Z"
  }
}
```

---

### Example 4: Managing Workspace Images

```bash
# Check if image exists
curl http://localhost:3000/api/settings/workspace/image

# Rebuild image (fails if in use)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": false}'

# Force rebuild (rebuilds even if in use)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": true}'

# Delete image (fails if in use)
curl -X DELETE http://localhost:3000/api/settings/workspace/image
```

---

### Example 5: Handling Container Failures

When a container fails repeatedly, it's marked as "failed" after exceeding the restart limit:

```bash
# Check workspace status
curl http://localhost:3000/api/workspaces/1

# Response shows failed container
{
  "id": 1,
  "workspace_status": "Failed",
  "container_status": "failed",
  "restart_count": 3,
  "message": "Container exceeded maximum restart attempts"
}
```

**Recovery Steps**:

1. Investigate logs: `docker logs workspace-1`
2. Fix underlying issue (e.g., update init script, increase resources)
3. Manually restart: `POST /api/workspaces/1/restart`

---

### Example 6: Customizing Container Resources

```bash
# Create workspace with custom resource limits
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 1,
    "cpu_limit": 2.0,
    "memory_limit": "1024m"
  }'
```

---

### Example 7: Rebuilding Image After Dockerfile Changes

```bash
# 1. Update Dockerfile at docker/workspace/Dockerfile
# 2. Rebuild image with force (if workspaces are running)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": true}'

# 3. Restart workspaces to use new image
curl -X POST http://localhost:3000/api/workspaces/1/restart
curl -X POST http://localhost:3000/api/workspaces/2/restart
```

---

## Troubleshooting

### Issue: Docker Not Available

**Symptoms**:
- API returns `503 Service Unavailable`
- Error message: "Docker not available"

**Solutions**:

1. Check if Docker daemon is running:
   ```bash
   docker ps
   ```

2. Verify Docker socket permissions:
   ```bash
   ls -l /var/run/docker.sock
   ```

3. Restart Docker daemon:
   ```bash
   sudo systemctl restart docker
   ```

4. Check VibeRepo logs:
   ```bash
   tail -f logs/vibe-repo.log
   ```

---

### Issue: Container Fails to Start

**Symptoms**:
- Container status is "failed"
- Workspace status is "Failed"
- Restart count at maximum (3)

**Solutions**:

1. Check Docker logs:
   ```bash
   docker logs workspace-{id}
   ```

2. Verify image exists:
   ```bash
   curl http://localhost:3000/api/settings/workspace/image
   ```

3. Check resource availability:
   ```bash
   docker stats
   ```

4. Rebuild image if corrupted:
   ```bash
   curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
     -H "Content-Type: application/json" \
     -d '{"force": true}'
   ```

5. Manually restart container:
   ```bash
   curl -X POST http://localhost:3000/api/workspaces/{id}/restart
   ```

---

### Issue: Image Build Failures

**Symptoms**:
- Rebuild endpoint returns 500 error
- Error message mentions build failure

**Solutions**:

1. Check Dockerfile syntax:
   ```bash
   docker build -f docker/workspace/Dockerfile -t test .
   ```

2. Verify internet connectivity (for apt-get):
   ```bash
   ping archive.ubuntu.com
   ```

3. Check disk space:
   ```bash
   df -h
   ```

4. Review build logs in VibeRepo logs:
   ```bash
   grep "build_image" logs/vibe-repo.log
   ```

5. Try building manually:
   ```bash
   cd /path/to/vibe-repo
   docker build -f docker/workspace/Dockerfile -t vibe-repo-workspace:latest .
   ```

---

### Issue: Health Check Issues

**Symptoms**:
- Containers restarting frequently
- Health check failures in logs

**Solutions**:

1. Check container logs for errors:
   ```bash
   docker logs workspace-{id}
   ```

2. Verify container is responsive:
   ```bash
   docker exec workspace-{id} echo "test"
   ```

3. Increase resource limits:
   ```bash
   # Update workspace with higher limits
   # (requires workspace recreation)
   ```

4. Check HealthCheckService logs:
   ```bash
   grep "health_check" logs/vibe-repo.log
   ```

5. Adjust health check interval (in code):
   ```rust
   // backend/src/services/health_check_service.rs
   const HEALTH_CHECK_INTERVAL_SECONDS: u64 = 60; // Increase from 30
   ```

---

### Issue: Cannot Delete Image

**Symptoms**:
- Delete endpoint returns 409 Conflict
- Error message: "workspace(s) are using it"

**Solutions**:

1. List workspaces using the image:
   ```bash
   curl http://localhost:3000/api/settings/workspace/image
   ```

2. Stop or delete workspaces:
   ```bash
   curl -X DELETE http://localhost:3000/api/workspaces/1
   curl -X DELETE http://localhost:3000/api/workspaces/2
   ```

3. Retry image deletion:
   ```bash
   curl -X DELETE http://localhost:3000/api/settings/workspace/image
   ```

---

### Issue: Container Stats Not Available

**Symptoms**:
- Stats endpoint returns 409 Conflict
- Error message: "Container is not running"

**Solutions**:

1. Check container status:
   ```bash
   curl http://localhost:3000/api/workspaces/{id}
   ```

2. Start container if stopped:
   ```bash
   curl -X POST http://localhost:3000/api/workspaces/{id}/restart
   ```

3. Verify Docker daemon is running:
   ```bash
   docker ps
   ```

---

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run container service tests
cargo test container_service

# Run image management tests
cargo test image_management

# Run integration tests
cargo test --test '*'

# Run with output
cargo test -- --nocapture
```

### Test Coverage

**v0.3.0 Test Statistics**:
- Total tests: 249
- Unit tests: 50 (new container/image features)
- Integration tests: 14 (new API endpoints)
- All tests passing: ✅

### Adding New Features

When adding new container lifecycle features:

1. **Write tests first** (TDD approach):
   ```rust
   #[tokio::test]
   async fn test_new_feature() {
       // Arrange
       let service = ContainerService::new(db, docker);
       
       // Act
       let result = service.new_feature().await;
       
       // Assert
       assert!(result.is_ok());
   }
   ```

2. **Implement feature** in service layer

3. **Add API endpoint** if needed:
   ```rust
   #[utoipa::path(
       post,
       path = "/api/workspaces/{id}/new-feature",
       responses(...)
   )]
   pub async fn new_feature_handler(...) -> Result<Json<Response>> {
       // Implementation
   }
   ```

4. **Update OpenAPI docs** in handler annotations

5. **Add integration test**:
   ```rust
   #[tokio::test]
   async fn test_new_feature_endpoint() {
       let app = create_test_app().await;
       let response = app.oneshot(request).await.unwrap();
       assert_eq!(response.status(), StatusCode::OK);
   }
   ```

6. **Update documentation** (this file)

### Debugging Tips

**Enable debug logging**:

```bash
# In .env file
RUST_LOG=debug

# Or for specific modules
RUST_LOG=vibe_repo::services::container_service=debug
```

**Check Docker operations**:

```bash
# List all containers
docker ps -a

# Inspect container
docker inspect workspace-{id}

# View container logs
docker logs workspace-{id}

# Check container stats
docker stats workspace-{id}
```

**Database queries**:

```bash
# Connect to SQLite database
sqlite3 data/vibe-repo/db/vibe-repo.db

# Query containers
SELECT * FROM containers;

# Query workspaces
SELECT * FROM workspaces;
```

**Useful log queries**:

```bash
# Container creation logs
grep "Creating container" logs/vibe-repo.log

# Restart operations
grep "restart" logs/vibe-repo.log

# Health check failures
grep "health_check.*failed" logs/vibe-repo.log
```

---

## Related Documentation

- [Workspace Feature Guide](./workspace-feature-analysis.md)
- [Init Scripts Guide](./init-scripts-guide.md)
- [Docker Workspace Image](../docker/workspace/README.md)
- [AGENTS.md](../AGENTS.md) - Development Guidelines
- [CHANGELOG.md](../CHANGELOG.md) - Version History

---

## Version History

- **v0.3.0** (2026-01-20): Initial release of Container Lifecycle Management
  - ContainerService with CRUD and lifecycle operations
  - ImageManagementService for image management
  - 5 new API endpoints
  - Automatic health monitoring and recovery
  - 249 tests passing

---

## Support

For issues or questions:

- **GitHub Issues**: Report bugs and feature requests
- **API Documentation**: Access Swagger UI at `http://localhost:3000/swagger-ui`
- **Development Guide**: See [AGENTS.md](../AGENTS.md)

---

**Last Updated**: 2026-01-20  
**Version**: 0.3.0
