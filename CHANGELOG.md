# Changelog

All notable changes to VibeRepo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Version Policy

**Pre-1.0 (Current)**: Breaking changes allowed, version increments by 0.0.1 for each update.

**Post-1.0**: Follows strict semantic versioning (MAJOR.MINOR.PATCH).

---

## [Unreleased]

---

## [0.4.0-mvp] - 2026-02-06

### 🎯 Simplified MVP Release

This release represents a major simplification of VibeRepo, focusing on core Issue-to-PR automation functionality while removing complex features that are not essential for the MVP.

### ⚠️ Breaking Changes

#### Removed Features
- **Issue Polling Service**: Removed automatic issue polling. Use webhooks only.
- **Webhook Retry Service**: Removed automatic webhook retry mechanism.
- **Webhook Cleanup Service**: Removed automatic webhook log cleanup.
- **Log Cleanup Service**: Removed automatic log file cleanup.
- **Init Script Service**: Removed container initialization script support.
- **Task Failure Analyzer**: Removed intelligent failure analysis.
- **Health Check Service**: Removed background health monitoring.
- **Image Management Service**: Removed Docker image management features.
- **WebSocket Support**: Removed real-time log streaming via WebSocket.
- **Task Execution History**: Removed execution history tracking (task_executions table).
- **Multi-Agent Support**: Simplified to single agent per repository.
- **Task Retry Mechanism**: Removed automatic task retry on failure.

#### Removed API Endpoints
- `POST /providers` - Provider management (use environment variables)
- `GET /providers` - List providers
- `GET /providers/:id` - Get provider details
- `PUT /providers/:id` - Update provider
- `DELETE /providers/:id` - Delete provider
- `POST /workspaces` - Workspace management (auto-created)
- `GET /workspaces` - List workspaces
- `GET /workspaces/:id` - Get workspace details
- `DELETE /workspaces/:id` - Delete workspace
- `POST /agents` - Agent management (use environment variables)
- `GET /agents` - List agents
- `GET /agents/:id` - Get agent details
- `PUT /agents/:id` - Update agent
- `DELETE /agents/:id` - Delete agent
- `GET /stats` - Statistics endpoints
- `GET /health` - Health check endpoint
- `GET /tasks/:id/logs/stream` - WebSocket log streaming

#### Database Schema Changes
- **Removed Tables**: `webhook_configs`, `init_scripts`, `task_executions`, `workspaces`
- **Modified Tables**:
  - `tasks`: Added `last_log` field (TEXT), removed `retry_count` and `max_retries`
  - `repositories`: Added `agent_command`, `agent_timeout`, `agent_env_vars`, `docker_image`
  - `agents`: Removed `enabled` field, added UNIQUE constraint per workspace
- **Table Count**: Reduced from 10 tables to 7 tables

#### Task Status Machine Simplification
- **Removed State**: `Assigned` (tasks go directly from Pending to Running)
- **Simplified Transitions**:
  - `Pending` → `Running`, `Cancelled`
  - `Running` → `Completed`, `Failed`, `Cancelled`
  - `Completed`, `Failed`, `Cancelled` are terminal states (no retry)
- **Removed**: Retry mechanism and state transitions

#### Configuration Changes
- **Provider Configuration**: Moved from database to environment variables
  - `GITHUB_TOKEN` - GitHub personal access token
  - `GITHUB_BASE_URL` - GitHub API base URL
  - `WEBHOOK_SECRET` - Webhook signature verification secret
- **Agent Configuration**: Moved from database to environment variables
  - `DEFAULT_AGENT_COMMAND` - Default agent command
  - `DEFAULT_AGENT_TIMEOUT` - Default agent timeout (seconds)
  - `DEFAULT_DOCKER_IMAGE` - Default Docker image
- **Removed Configuration**: WebSocket, polling, retry, cleanup settings

### ✨ Added

#### Core Features
- **Environment Variable Configuration**: All configuration via environment variables
- **Single Agent Mode**: One agent per repository, configured at repository creation
- **Simplified Log Storage**: Logs stored in `tasks.last_log` field (10MB limit)
- **Webhook-Only Task Creation**: Tasks created only via webhook events

#### API Endpoints (10 Core Endpoints)
- `POST /repositories` - Create repository with agent configuration
- `GET /repositories` - List repositories
- `GET /repositories/:id` - Get repository details
- `DELETE /repositories/:id` - Delete repository
- `POST /webhooks/github` - GitHub webhook handler
- `GET /tasks` - List tasks with filtering
- `POST /tasks/:id/execute` - Execute task manually
- `GET /tasks/:id/logs` - Get task logs (from last_log field)
- `GET /tasks/:id/status` - Get task status and timestamps
- `DELETE /tasks/:id` - Delete task

#### Documentation
- **Deployment Guide**: New simplified deployment documentation
- **Migration Guide**: Guide for migrating from full version
- **API Reference**: Updated to reflect simplified endpoints
- **Environment Variables**: Complete environment variable documentation

### 🔧 Changed

#### Task Execution
- Logs now stored directly in `tasks.last_log` field (max 10MB)
- Log truncation when exceeding size limit
- No execution history tracking
- Simplified error handling (error message only)

#### Repository Management
- Agent configuration embedded in repository entity
- Automatic workspace creation with repository settings
- Single agent per workspace (UNIQUE constraint)

#### Webhook Integration
- Webhook configuration via environment variables only
- Simplified webhook handler (no retry, no cleanup)
- Direct task creation from webhook events

### 🗑️ Removed

#### Dependencies
- Removed `axum` WebSocket feature
- Removed unused `futures-util` (if only used by WebSocket)

#### Code Reduction
- **Total Lines**: Reduced by ~23% (from ~30,000 to ~23,000 lines)
- **Services**: Reduced from 15+ to 8 core services
- **API Endpoints**: Reduced from 40+ to 10 core endpoints
- **Database Tables**: Reduced from 10 to 7 tables

### 📊 Test Results

- **Unit Tests**: 280 passed, 0 failed, 5 ignored
- **Integration Tests**: 56 passed, 2 failed, 4 ignored
- **Test Pass Rate**: 99.4% (336/338)
- **Compilation**: Clean build with 0 errors

### 🚀 Migration Notes

**From v0.3.0 to v0.4.0-mvp:**

1. **Database Migration Required**: The schema has changed significantly
   - Export data from v0.3.0
   - Transform to match simplified schema
   - Import into v0.4.0-mvp

2. **Configuration Migration**:
   - Move provider settings from database to environment variables
   - Move agent settings from database to environment variables
   - Update webhook configuration to use environment variables

3. **API Client Updates**:
   - Remove calls to deleted endpoints
   - Update task creation to use webhook-only approach
   - Update log retrieval to use new `/tasks/:id/logs` endpoint

4. **Feature Adjustments**:
   - Replace WebSocket log streaming with polling
   - Remove retry logic from client code
   - Remove execution history queries

### 📝 Documentation Updates

- Updated README.md with simplified MVP description
- Created deployment guide for simplified version
- Updated API reference to reflect 10 core endpoints
- Added migration guide from full version
- Updated database schema documentation

### 🎯 Core Functionality Retained

- ✅ Repository management
- ✅ Webhook integration (GitHub)
- ✅ Task creation and execution
- ✅ Docker container management
- ✅ PR creation from task results
- ✅ Issue closure after PR merge
- ✅ Log query API
- ✅ Complete Issue-to-PR workflow

---

## [0.3.0] - 2026-01-20

---

## [0.3.0] - 2026-01-20

### Added

#### Container Lifecycle Management
- **ContainerService**: New service for container CRUD and lifecycle management
  - Automatic container creation and startup
  - Manual and automatic restart capabilities
  - Restart count tracking with configurable limits (default: 3 attempts)
  - Container status management (creating, running, stopped, exited, failed)
  - Container naming convention: `workspace-{workspace_id}`
  - Default workspace mount at `/workspace`
- **ImageManagementService**: New service for workspace image management
  - Image information queries (size, creation time, usage)
  - Safe image deletion with conflict detection
  - Image rebuild with force option
  - Workspace usage tracking per image
- **DockerService Enhancements**: 7 new methods for image and container operations
  - `image_exists()` - Check if image exists
  - `build_image()` - Build Docker image from Dockerfile
  - `remove_image()` - Remove Docker image
  - `inspect_image()` - Get image metadata (size, creation time, ID)
  - `list_containers_using_image()` - List containers by image
  - `restart_container()` - Restart container with timeout (default: 10s)
  - `get_container_stats()` - Get real-time resource usage (CPU, memory, network)
- **WorkspaceService Updates**: Container integration
  - `create_workspace_with_container()` - Create workspace with container
  - `ensure_image_exists()` - Auto-build images when needed
  - Returns tuple `(workspace, Option<container>)` from creation
- **HealthCheckService Enhancements**: Automatic container recovery
  - Auto-restart unhealthy containers
  - Respect max restart attempts (default: 3)
  - Mark containers as failed after limit exceeded
  - Health check interval: 30 seconds

#### API Endpoints
- `POST /api/workspaces/:id/restart` - Manually restart workspace container
  - Returns restart count and last restart timestamp
  - Increments restart counter
- `GET /api/workspaces/:id/stats` - Get container resource statistics
  - CPU usage percentage
  - Memory usage and limits
  - Network RX/TX bytes
  - Real-time data collection
- `GET /api/settings/workspace/image` - Query workspace image information
  - Image existence check
  - Size and creation time
  - List of workspaces using the image
- `DELETE /api/settings/workspace/image` - Delete workspace image
  - Conflict detection (prevents deletion if in use)
  - Helpful error messages with workspace IDs
- `POST /api/settings/workspace/image/rebuild` - Rebuild workspace image
  - Force option to rebuild even if in use
  - Build time tracking
  - Warning when workspaces need restart

#### Database
- New `containers` table for container metadata
  - Fields: workspace_id, container_id, container_name, image_name, status
  - Restart tracking: restart_count, max_restart_attempts, last_restart_at
  - Health monitoring: health_check_failures, last_health_check_at
  - Timestamps: created_at, updated_at
- Migration from workspace.container_id to separate containers table
- Cascade delete support (deleting workspace deletes container)
- One-to-one relationship: workspace ↔ container

#### Infrastructure
- Default workspace Dockerfile (Ubuntu 22.04 based)
  - Pre-installed tools: git, curl, wget, vim, nano, build-essential, jq
  - Size: ~200MB compressed
  - Location: `docker/workspace/Dockerfile`
- Workspace image build system
  - Automatic build on first workspace creation
  - Manual rebuild via API
  - Force rebuild option for updates
- Container resource monitoring
  - Real-time CPU and memory tracking
  - Network usage statistics
  - Resource limit enforcement

### Changed
- **BREAKING**: `WorkspaceService::create_workspace_with_container()` return type changed
  - Old: `Result<workspace::Model>`
  - New: `Result<(workspace::Model, Option<container::Model>)>`
- Workspace status management improved with "Active" and "Failed" states
  - "Active" - Container running normally
  - "Failed" - Container exceeded restart limits
- Enhanced error messages with workspace IDs and suggestions
  - Conflict errors include affected workspace IDs
  - Suggestions for resolution (e.g., "stop these workspaces first")
- Container lifecycle now fully managed by ContainerService
  - Centralized restart logic (manual and automatic)
  - Consistent status tracking
  - Unified error handling

### Technical Details
- **Tests**: 249 total (100% passing)
  - 50 new unit tests for container and image services
  - 14 new integration tests for API endpoints
  - Property-based tests for edge cases
- **Documentation**: Full OpenAPI documentation for all endpoints
  - Request/response schemas
  - Error codes and descriptions
  - Example requests and responses
- **Error Handling**: Comprehensive error handling with conflict detection
  - `ServiceUnavailable` - Docker not available
  - `NotFound` - Workspace/container not found
  - `Conflict` - Image in use, container not running
- **Logging**: Structured logging with tracing
  - Container lifecycle events
  - Restart operations (manual and automatic)
  - Health check results
  - Image build operations
- **TDD Approach**: Test-first implementation throughout
  - Red-Green-Refactor cycle
  - High test coverage
  - Integration tests for all endpoints

### Breaking Changes
1. **WorkspaceService API Change**:
   ```rust
   // Old (v0.2.0)
   let workspace = workspace_service.create_workspace_with_container(...).await?;
   
   // New (v0.3.0)
   let (workspace, container) = workspace_service.create_workspace_with_container(...).await?;
   ```

2. **Database Schema**:
   - New required `containers` table (migration runs automatically)
   - Workspace model no longer has direct container_id field
   - Container information accessed via ContainerService

### Migration Guide

**For API Users**:
- No changes required - API endpoints remain backward compatible
- New endpoints available for container management

**For Service Layer Users**:
- Update calls to `create_workspace_with_container()` to handle tuple return
- Use `ContainerService` for container operations instead of direct Docker calls

**For Database**:
- Migration runs automatically on startup
- Existing workspace data preserved
- New containers table created

### Documentation
- Added `docs/container-lifecycle-management.md` (comprehensive guide)
  - Architecture overview
  - Component descriptions
  - API endpoint documentation with examples
  - Configuration reference
  - 7 usage examples
  - Troubleshooting guide
  - Development guide
- Updated `docker/workspace/README.md` with API integration details
- Updated `README.md` with Container Lifecycle Management section
- All documentation includes version numbers and timestamps

### Performance
- Efficient container status queries (indexed by workspace_id)
- Minimal Docker API calls (cached image checks)
- Background health checks run every 30 seconds
- Restart operations complete in <5 seconds

### Security
- Container isolation via Docker
- Resource limits enforced (CPU, memory)
- No privileged containers
- Workspace mount at `/workspace` only

---

## [0.2.0] - 2026-01-20

### Added
- **Init Scripts Feature**: Automated container setup with custom shell scripts
  - Create init scripts when creating workspaces
  - Update and execute scripts via API
  - Hybrid storage strategy (≤4KB in DB, >4KB in files)
  - Automatic log cleanup (30-day retention)
  - Concurrency control with database locking
  - 6 execution states: Pending, Running, Success, Failed, Timeout, Cancelled
  - 4 new API endpoints for script management
  - Comprehensive integration tests (12 new tests)
  - Complete documentation and migration guide
- Docker exec integration with timeout support
- LogCleanupService for automatic log file management
- Comprehensive usage guide with 7 common use cases
- Best practices and troubleshooting documentation

### Changed
- **BREAKING**: Removed `custom_dockerfile_path` field from workspaces
  - Replaced with more flexible init_script functionality
  - See `docs/migration-guide-init-scripts.md` for migration instructions
- Updated workspace API to include init_script in responses
- Enhanced OpenAPI documentation with init script endpoints
- Increased test coverage to 500+ tests

### Technical Details
- Database: New init_scripts table with 1:1 relationship to workspaces
- Services: InitScriptService with CRUD and execution logic
- API: 4 new endpoints (create/update, execute, logs, download)
- Testing: 500+ tests including unit, integration, and API tests
- Documentation: 3 comprehensive guides totaling 1000+ lines

### Development
- Implemented using Subagent-Driven Development methodology
- 15 commits with systematic code reviews
- TDD approach with test-first implementation
- Complete OpenAPI documentation

---

## [0.1.1] - 2026-01-16

### Added
- Project skill system in `.opencode/skills/`
- `vibe-repo-development.md` skill for development workflow guidance
- Development progress tracking document (CHANGELOG.md)

### Changed
- Updated AGENTS.md with comprehensive development guidelines
- Enhanced documentation structure

---

## [0.1.0] - 2026-01-14

### Added
- **Backend Foundation**
  - Configuration management with environment variables
  - Database connection pooling (SQLite/PostgreSQL)
  - Unified error handling with `VibeRepoError`
  - Health check endpoint with database connectivity check
  - Structured logging with tracing
  - Graceful shutdown support

- **RepoProvider API** (Settings Module)
  - CRUD operations for Git provider configurations
  - Token validation endpoint
  - Manual repository sync trigger
  - Access token masking in responses
  - Unique constraint on (name, base_url, access_token)
  - Locked provider protection

- **Repository API**
  - List repositories with filtering (provider_id, validation_status)
  - Get repository details
  - Refresh validation status
  - Single repository initialization
  - Batch repository initialization
  - Automatic repository sync on provider create/update

- **Git Provider Abstraction**
  - Unified `GitProvider` trait interface
  - Gitea client implementation (full support)
  - GitHub client placeholder
  - GitLab client placeholder
  - Static dispatch with compile-time polymorphism
  - Repository, branch, issue, PR, and label operations

- **Repository Initialization**
  - Configurable branch creation
  - Label management (vibe/* prefix)
  - Validation checks (branches, labels, permissions)
  - Background sync service (hourly)

- **Database Schema**
  - `repo_providers` table with authentication
  - `repositories` table with validation status
  - Cascade delete relationships
  - SeaORM entity definitions
  - Automatic migrations on startup

- **API Documentation**
  - OpenAPI 3.0 specification
  - Swagger UI at `/swagger-ui`
  - Comprehensive endpoint documentation

- **Testing Infrastructure**
  - 181+ total tests (100% passing)
  - Unit tests in source files
  - Integration tests in `tests/` directory
  - Property-based tests with proptest
  - Test utilities for database and state management

### Technical Details
- **Language**: Rust 2021
- **Framework**: Axum 0.7
- **Database**: SeaORM 0.12
- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest 0.11
- **API Docs**: utoipa 4.x

### Development Standards
- Test-Driven Development (TDD) methodology
- Conventional Commits for commit messages
- English for all code and documentation
- Comprehensive code style guidelines
- Module documentation requirements

---

## Version History Summary

| Version | Date | Key Features | Status |
|---------|------|--------------|--------|
| 0.1.1 | 2026-01-16 | Skill system, documentation | Current |
| 0.1.0 | 2026-01-14 | Initial release, core APIs | Stable |

---

## How to Update Version

When making changes:

1. **Increment version** in `backend/Cargo.toml`:
   ```toml
   version = "0.1.2"  # Increment by 0.0.1
   ```

2. **Update CHANGELOG.md**:
   - Move items from `[Unreleased]` to new version section
   - Add date in format `YYYY-MM-DD`
   - Categorize changes: Added, Changed, Deprecated, Removed, Fixed, Security

3. **Update AGENTS.md**:
   - Update version number in header
   - Update "Current Status" section if needed

4. **Commit with conventional commit**:
   ```bash
   git add backend/Cargo.toml CHANGELOG.md AGENTS.md
   git commit -m "chore: bump version to 0.1.2"
   ```

---

## Categories

- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security vulnerability fixes

---

## Links

- [Project Repository](https://github.com/yourusername/vibe-repo)
- [Issue Tracker](https://github.com/yourusername/vibe-repo/issues)
- [Documentation](./docs/)
- [AGENTS.md](./AGENTS.md) - Development Guidelines

---

**Note**: This project is in pre-1.0 development. Breaking changes may occur between versions.
