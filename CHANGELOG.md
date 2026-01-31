# Changelog

All notable changes to VibeRepo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Version Policy

**Pre-1.0 (Current)**: Breaking changes allowed, version increments by 0.0.1 for each update.

**Post-1.0**: Follows strict semantic versioning (MAJOR.MINOR.PATCH).

---

## [Unreleased]

### Changed

#### Task State Management Refactor
- **BREAKING**: Task status changed from String to TaskStatus enum
  - **Old**: `task_status: String` with arbitrary string values
  - **New**: `task_status: TaskStatus` with type-safe enum and state machine validation
  - **API Impact**: API responses now return lowercase enum values (e.g., `"pending"`, `"running"`, `"completed"`)
  - **Validation**: Invalid state transitions now return `400 Bad Request` with detailed error message
  - **State Transitions**:
    - `pending` → `assigned`, `cancelled`
    - `assigned` → `running`, `cancelled`
    - `running` → `completed`, `failed`, `cancelled`
    - `failed` → `pending` (retry only, if retry_count < max_retries)
    - `completed` and `cancelled` are terminal states (no further transitions)
  - **Migration**: Existing string values automatically converted to enum during migration
  - **Error Handling**: New `InvalidStateTransition` error variant with current state, target state, and allowed transitions
  - **Benefits**: Compile-time type safety, prevents data corruption from illegal state changes, self-documenting state machine

### Planned
- Agent configuration management
- Task automation system
- GitHub provider implementation
- GitLab provider implementation

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
