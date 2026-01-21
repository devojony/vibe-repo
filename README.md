# VibeRepo

**Version:** 0.3.0 (Pre-1.0 - Breaking changes allowed)

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

## Features

- **Multi-Provider Support**: Unified interface for Gitea, GitHub, and GitLab
- **Automated Repository Management**: Automatic sync and validation of repositories
- **Repository Initialization**: Automated branch and label setup for new repositories
- **Webhook Integration**: Real-time event processing from Git providers
- **Issue Polling**: Automatic issue synchronization with intelligent filtering (labels, mentions, state, age)
- **Dual-Mode Issue Tracking**: Webhook-first with automatic polling fallback on webhook failures
- **Task Management**: Complete task lifecycle management with automatic retry, priority scheduling, and agent assignment
- **Workspace Management**: Docker-based isolated development environments
- **Container Lifecycle Management**: Automated Docker container management with health monitoring
- **Init Scripts**: Automated container setup with custom shell scripts
- **Background Services**: Scheduled repository synchronization, issue polling, and log cleanup
- **RESTful API**: Comprehensive API with OpenAPI documentation
- **Database Flexibility**: Support for both SQLite (development) and PostgreSQL (production)

## Technology Stack

- **Language**: Rust (Edition 2021)
- **Framework**: Axum 0.7 with WebSocket support
- **Async Runtime**: Tokio with full features
- **Database ORM**: SeaORM 1.1 (supports SQLite and PostgreSQL)
- **HTTP Client**: Reqwest 0.11 for Git provider APIs
- **API Documentation**: utoipa 4.x with Swagger UI
- **Testing**: Comprehensive TDD approach with 500+ tests

## Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- SQLite 3 or PostgreSQL
- Git provider account (Gitea/GitHub/GitLab)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/vibe-repo.git
cd vibe-repo/backend
```

2. Create `.env` file in project root:
```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
```

3. Build and run:
```bash
cargo build
cargo run
```

The server will start at `http://localhost:3000`

### Verify Installation

Check the health endpoint:
```bash
curl http://localhost:3000/health
```

Access the API documentation:
```
http://localhost:3000/swagger-ui
```

## API Endpoints

### Health Check
- `GET /health` - Service health status with database connectivity check

### Settings Module

#### RepoProvider Management
- `GET /api/settings/providers` - List all Git provider configurations
- `POST /api/settings/providers` - Create a new provider configuration
- `GET /api/settings/providers/:id` - Get provider details
- `PUT /api/settings/providers/:id` - Update provider configuration
- `DELETE /api/settings/providers/:id` - Delete provider (if not locked)
- `POST /api/settings/providers/:id/validate` - Validate provider token
- `POST /api/settings/providers/:id/sync` - Manually trigger repository sync

### Repository Module
- `GET /api/repositories` - List repositories with optional filters
- `GET /api/repositories/:id` - Get repository details
- `POST /api/repositories/:id/refresh` - Refresh repository validation status
- `POST /api/repositories/:id/initialize` - Initialize single repository
- `POST /api/repositories/batch-initialize` - Batch initialize repositories
- `PATCH /api/repositories/:id/polling` - Update issue polling configuration
- `POST /api/repositories/:id/poll-issues` - Manually trigger issue polling

### Webhook Module
- `POST /api/webhooks/:repository_id` - Receive webhook events from Git providers

### Workspace Module
- `POST /api/workspaces` - Create a new workspace with optional init script
- `GET /api/workspaces/:id` - Get workspace details including init script status
- `GET /api/workspaces` - List all workspaces
- `PUT /api/workspaces/:id/status` - Update workspace status
- `DELETE /api/workspaces/:id` - Delete workspace

### Init Script Module
- `PUT /api/workspaces/:id/init-script` - Create or update init script for workspace
- `GET /api/workspaces/:id/init-script/logs` - Get init script execution logs
- `GET /api/workspaces/:id/init-script/logs/full` - Download full log file
- `POST /api/workspaces/:id/init-script/execute` - Execute init script manually

### Task Module
#### CRUD Operations
- `POST /api/tasks` - Create a new task
- `GET /api/tasks` - List tasks with optional filters (status, priority, assigned_agent_id)
- `GET /api/tasks/:id` - Get task details
- `PATCH /api/tasks/:id` - Update task (priority, assigned_agent_id)
- `DELETE /api/tasks/:id` - Soft delete task

#### Status Management
- `PATCH /api/tasks/:id/status` - Update task status directly
- `POST /api/tasks/:id/assign` - Assign agent to task
- `POST /api/tasks/:id/start` - Start task execution
- `POST /api/tasks/:id/complete` - Mark task completed with PR information
- `POST /api/tasks/:id/fail` - Mark task failed (with automatic retry logic)
- `POST /api/tasks/:id/retry` - Retry a failed task
- `POST /api/tasks/:id/cancel` - Cancel task execution

## Init Scripts

Init scripts allow you to automatically configure workspace containers after they start. This replaces the previous `custom_dockerfile_path` approach with a more flexible shell script solution.

### Features

- **Automatic Execution**: Scripts run automatically when a workspace container starts
- **Hybrid Storage**: Small outputs (≤4KB) stored in database, larger outputs in files
- **Timeout Control**: Configurable timeout (default: 300 seconds)
- **Status Tracking**: Monitor script execution status (Pending/Running/Success/Failed)
- **Log Management**: Automatic cleanup of logs older than 30 days
- **Concurrency Control**: Prevents multiple simultaneous executions

### Usage Examples

#### 1. Create a workspace with an init script

```bash
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 1,
    "init_script": "#!/bin/bash\necho \"Setting up workspace...\"\napt-get update\napt-get install -y git curl vim\necho \"Setup complete!\"",
    "script_timeout_seconds": 600
  }'
```

Response includes the created init script:
```json
{
  "id": 1,
  "repository_id": 1,
  "workspace_status": "Initializing",
  "init_script": {
    "id": 1,
    "workspace_id": 1,
    "script_content": "#!/bin/bash\necho \"Setting up workspace...\"\n...",
    "timeout_seconds": 600,
    "status": "Pending",
    "created_at": "2026-01-19T14:20:54Z"
  }
}
```

#### 2. Create a workspace without init script

```bash
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 2
  }'
```

The `init_script` field will be `null` in the response.

#### 3. Update an existing init script

```bash
curl -X PUT http://localhost:3000/api/workspaces/1/init-script \
  -H "Content-Type: application/json" \
  -d '{
    "script_content": "#!/bin/bash\necho \"Updated script\"\ndate\nuname -a",
    "timeout_seconds": 300,
    "execute_immediately": false
  }'
```

#### 4. Execute init script manually

```bash
curl -X POST http://localhost:3000/api/workspaces/1/init-script/execute \
  -H "Content-Type: application/json" \
  -d '{
    "force": false
  }'
```

Note: Returns 409 Conflict if script is already running.

#### 5. Check script execution logs

```bash
curl http://localhost:3000/api/workspaces/1/init-script/logs
```

Response:
```json
{
  "status": "Success",
  "output_summary": "Setup complete!\n",
  "has_full_log": false,
  "executed_at": "2026-01-19T14:25:30Z"
}
```

#### 6. Download full log file (for large outputs)

```bash
curl http://localhost:3000/api/workspaces/1/init-script/logs/full -o script.log
```

### Migration from custom_dockerfile_path

If you were using `custom_dockerfile_path`, see [docs/migration-guide-init-scripts.md](./docs/migration-guide-init-scripts.md) for migration instructions.

## Container Lifecycle Management

VibeRepo automatically manages Docker containers for isolated development environments:

- **Automatic Container Creation**: Containers are created and started automatically when workspaces are initialized
- **Health Monitoring**: Continuous health checks with automatic restart on failure (every 30 seconds)
- **Resource Monitoring**: Real-time CPU, memory, and network usage statistics via API
- **Image Management**: Build, rebuild, and manage workspace Docker images
- **Manual Control**: API endpoints for manual restart and monitoring
- **Restart Policies**: Configurable restart limits (default: 3 attempts) with automatic failure detection
- **Graceful Degradation**: Containers marked as failed after exceeding restart limits

### Container Management Endpoints

- `POST /api/workspaces/:id/restart` - Manually restart workspace container
- `GET /api/workspaces/:id/stats` - Get real-time container resource statistics
- `GET /api/settings/workspace/image` - Query workspace image information
- `DELETE /api/settings/workspace/image` - Delete workspace image (with conflict detection)
- `POST /api/settings/workspace/image/rebuild` - Rebuild workspace image from Dockerfile

### Example: Get Container Statistics

```bash
curl http://localhost:3000/api/workspaces/1/stats
```

Response:
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

See [Container Lifecycle Management Documentation](docs/container-lifecycle-management.md) for complete details.

## Issue Polling

VibeRepo provides automatic issue synchronization from Git providers with intelligent filtering and dual-mode operation (webhook + polling).

### Features

- **Automatic Synchronization**: Periodic polling of issues from Git providers
- **Intelligent Filtering**: Filter by labels, mentions, state, and age
- **Dual-Mode Operation**: Webhook-first with automatic polling fallback
- **Automatic Failover**: Enables polling after 5 consecutive webhook failures
- **Per-Repository Configuration**: Customize polling settings for each repository
- **Concurrent Processing**: Poll multiple repositories in parallel (10x performance)
- **Workspace Mapping Cache**: 99% cache hit rate for workspace lookups
- **Rate Limiting Protection**: Exponential backoff retry mechanism

### Quick Start

#### 1. Enable polling for a repository

```bash
curl -X PATCH http://localhost:3000/api/repositories/1/polling \
  -H "Content-Type: application/json" \
  -d '{
    "polling_enabled": true,
    "polling_interval_seconds": 300,
    "polling_config": {
      "filter_labels": ["vibe/auto", "bug"],
      "filter_mentions": ["@vibe-bot"],
      "filter_state": "open",
      "max_issue_age_days": 30
    }
  }'
```

#### 2. Manually trigger polling

```bash
curl -X POST http://localhost:3000/api/repositories/1/poll-issues
```

Response:
```json
{
  "repository_id": 1,
  "issues_found": 5,
  "tasks_created": 3,
  "tasks_updated": 2,
  "polling_duration_ms": 234
}
```

#### 3. Check polling status

```bash
curl http://localhost:3000/api/repositories/1
```

Response includes polling configuration:
```json
{
  "id": 1,
  "name": "my-repo",
  "polling_enabled": true,
  "polling_interval_seconds": 300,
  "last_polled_at": "2026-01-21T10:30:00Z",
  "polling_config": {
    "filter_labels": ["vibe/auto"],
    "filter_state": "open"
  }
}
```

### Configuration Options

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `polling_enabled` | boolean | Enable/disable polling | `false` |
| `polling_interval_seconds` | integer | Polling interval (60-86400) | `300` |
| `filter_labels` | array | Filter by labels (OR logic) | `[]` |
| `filter_mentions` | array | Filter by @mentions | `[]` |
| `filter_state` | string | Filter by state (open/closed/all) | `open` |
| `max_issue_age_days` | integer | Max issue age in days | `30` |

### Automatic Failover

When webhooks fail repeatedly, polling is automatically enabled:

1. Webhook fails 5 times consecutively
2. System automatically enables polling for the repository
3. Polling continues until webhooks are restored
4. Manual re-enable of webhooks disables automatic polling

See [Issue Polling Documentation](docs/issue-polling-feature.md) for complete details (Chinese).

## Task Management

VibeRepo provides comprehensive task management capabilities for automated development workflows. Tasks are created from issues and executed by AI agents in isolated workspace containers.

### Features

- **Complete Lifecycle Management**: From creation to completion with clear state transitions
- **Automatic Retry Mechanism**: Configurable retry logic for failed tasks
- **Agent Assignment**: Assign specific AI agents to tasks
- **Priority Management**: High/Medium/Low priority levels
- **Advanced Filtering**: Filter tasks by status, priority, and assigned agent
- **Soft Delete**: Preserve task history with soft deletion
- **PR Integration**: Track pull request information for completed tasks

### Task Lifecycle

```
Create (Pending) 
  → Assign (Assigned) 
  → Start (Running) 
  → Complete (Completed) OR Fail (Pending/Failed)
  → Retry (Pending) OR Cancel (Cancelled)
```

### Task Status Values

- **Pending**: Task created, waiting to be assigned
- **Assigned**: Agent assigned, ready to start
- **Running**: Task execution in progress
- **Completed**: Task successfully completed with PR created
- **Failed**: Task failed after exhausting retries
- **Cancelled**: Task manually cancelled

### Quick Start

#### 1. Create a task from an issue

```bash
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 42,
    "issue_title": "Add user authentication",
    "issue_body": "Implement JWT-based authentication...",
    "issue_url": "https://git.example.com/owner/repo/issues/42",
    "priority": "High",
    "max_retries": 3
  }'
```

Response:
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "task_status": "Pending",
  "priority": "High",
  "retry_count": 0,
  "max_retries": 3,
  "created_at": "2026-01-21T10:00:00Z"
}
```

#### 2. Assign an agent to the task

```bash
curl -X POST http://localhost:3000/api/tasks/1/assign \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": 5
  }'
```

Response:
```json
{
  "id": 1,
  "task_status": "Assigned",
  "assigned_agent_id": 5,
  "updated_at": "2026-01-21T10:01:00Z"
}
```

#### 3. Start task execution

```bash
curl -X POST http://localhost:3000/api/tasks/1/start
```

Response:
```json
{
  "id": 1,
  "task_status": "Running",
  "started_at": "2026-01-21T10:02:00Z"
}
```

#### 4. Complete the task with PR information

```bash
curl -X POST http://localhost:3000/api/tasks/1/complete \
  -H "Content-Type: application/json" \
  -d '{
    "pr_number": 123,
    "pr_url": "https://git.example.com/owner/repo/pulls/123",
    "branch_name": "feature/user-auth"
  }'
```

Response:
```json
{
  "id": 1,
  "task_status": "Completed",
  "pr_number": 123,
  "pr_url": "https://git.example.com/owner/repo/pulls/123",
  "branch_name": "feature/user-auth",
  "completed_at": "2026-01-21T10:15:00Z"
}
```

#### 5. Handle task failure (with automatic retry)

```bash
curl -X POST http://localhost:3000/api/tasks/1/fail \
  -H "Content-Type: application/json" \
  -d '{
    "error_message": "Build failed: missing dependency"
  }'
```

Response (if retry_count < max_retries):
```json
{
  "id": 1,
  "task_status": "Pending",
  "retry_count": 1,
  "max_retries": 3,
  "error_message": "Build failed: missing dependency"
}
```

Response (if retry_count >= max_retries):
```json
{
  "id": 1,
  "task_status": "Failed",
  "retry_count": 3,
  "max_retries": 3,
  "error_message": "Build failed: missing dependency"
}
```

#### 6. List tasks with filters

```bash
# List all pending tasks
curl "http://localhost:3000/api/tasks?workspace_id=1&status=Pending"

# List high priority tasks
curl "http://localhost:3000/api/tasks?workspace_id=1&priority=High"

# List tasks assigned to specific agent
curl "http://localhost:3000/api/tasks?workspace_id=1&assigned_agent_id=5"

# Combine multiple filters
curl "http://localhost:3000/api/tasks?workspace_id=1&status=Running&priority=High"
```

#### 7. Update task properties

```bash
curl -X PATCH http://localhost:3000/api/tasks/1 \
  -H "Content-Type: application/json" \
  -d '{
    "priority": "High",
    "assigned_agent_id": 7
  }'
```

#### 8. Retry a failed task

```bash
curl -X POST http://localhost:3000/api/tasks/1/retry
```

Response:
```json
{
  "id": 1,
  "task_status": "Pending",
  "retry_count": 0,
  "error_message": null
}
```

#### 9. Cancel a task

```bash
curl -X POST http://localhost:3000/api/tasks/1/cancel
```

Response:
```json
{
  "id": 1,
  "task_status": "Cancelled",
  "updated_at": "2026-01-21T10:20:00Z"
}
```

#### 10. Soft delete a task

```bash
curl -X DELETE http://localhost:3000/api/tasks/1
```

Response:
```json
{
  "message": "Task soft deleted successfully",
  "task_id": 1,
  "deleted_at": "2026-01-21T10:25:00Z"
}
```

### Task Priority Levels

- **High**: Critical tasks that should be executed first
- **Medium**: Normal priority tasks (default)
- **Low**: Tasks that can be deferred

### Automatic Retry Logic

When a task fails:
1. If `retry_count < max_retries`: Status → "Pending" (automatic retry)
2. If `retry_count >= max_retries`: Status → "Failed" (no more retries)

Default `max_retries` is 3, but can be customized per task.

### Integration with Issue Polling

Tasks are automatically created when the Issue Polling service detects new issues:

1. Issue Polling service finds a new issue matching filters
2. System checks if workspace exists for the repository
3. If workspace exists, creates a new task with status "Pending"
4. Task is ready to be assigned to an agent and executed

### Best Practices

1. **Set appropriate max_retries**: Balance between resilience and resource usage
2. **Use priority levels**: Ensure critical tasks are executed first
3. **Monitor failed tasks**: Review error messages to identify systemic issues
4. **Clean up completed tasks**: Periodically soft delete old completed tasks
5. **Use filters effectively**: Query specific task subsets for better performance

See [Task API Design Document](docs/task-api-design.md) for complete API specifications.

## Development

### Build Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run
```

### Testing

```bash
# Run all tests (500+ tests)
cargo test

# Run specific test
cargo test test_name

# Run with output visible
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

### Code Quality

```bash
# Check for warnings and style issues
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --check
```

## Project Structure

```
backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library root
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types
│   ├── state.rs             # Application state
│   ├── api/                 # HTTP API layer
│   │   ├── health/          # Health check module
│   │   ├── settings/        # Settings namespace
│   │   │   └── providers/   # RepoProvider API
│   │   ├── repositories/    # Repository API
│   │   └── webhooks/        # Webhook receiver
│   ├── services/            # Background services
│   ├── git_provider/        # Git provider abstraction
│   │   ├── traits.rs        # GitProvider trait
│   │   ├── factory.rs       # GitClientFactory
│   │   └── gitea/           # Gitea implementation
│   ├── db/                  # Database connection
│   ├── entities/            # SeaORM entities
│   ├── migration/           # Database migrations
│   └── test_utils/          # Test utilities
└── tests/                   # Integration tests
```

## Configuration

Configuration is loaded from `.env` file:

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | Database connection URL | `sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc` |
| `DATABASE_MAX_CONNECTIONS` | Max database connections | `10` |
| `SERVER_HOST` | Server bind address | `0.0.0.0` |
| `SERVER_PORT` | Server port | `3000` |
| `RUST_LOG` | Log level | `info` |
| `LOG_FORMAT` | Log format (text/json) | `text` |
| `ISSUE_POLLING_ENABLED` | Enable issue polling service | `true` |
| `ISSUE_POLLING_INTERVAL_SECONDS` | Global polling interval | `300` (5 minutes) |
| `ISSUE_POLLING_BATCH_SIZE` | Max repositories per batch | `10` |
| `ISSUE_POLLING_MAX_ISSUE_AGE_DAYS` | Max issue age to poll | `30` |

## Database Schema

### repo_providers
Git provider configurations with authentication credentials.

**Key Fields:**
- `name`, `type`, `base_url`, `access_token`
- `locked` - Prevents deletion when true
- Unique constraint on (name, base_url, access_token)

### repositories
Repository records with validation status and polling configuration.

**Key Fields:**
- `provider_id` (FK to repo_providers)
- `name`, `full_name`, `clone_url`, `default_branch`
- `validation_status` - 'valid', 'invalid', 'pending'
- Validation flags: `has_required_branches`, `has_required_labels`, etc.
- Polling fields: `polling_enabled`, `polling_interval_seconds`, `polling_config`, `last_polled_at`

### webhook_configs
Webhook configurations for repository event monitoring.

**Key Fields:**
- `repository_id` (FK to repositories, one-to-one)
- `provider_id` (FK to repo_providers, redundant for performance)
- `webhook_id`, `webhook_secret`, `webhook_url`
- `events` - JSON array of subscribed events
- Retry mechanism: `retry_count`, `last_retry_at`, `next_retry_at`

### workspaces
Docker-based isolated development environments for repositories.

**Key Fields:**
- `repository_id` (FK to repositories, one-to-one)
- `workspace_status` - 'creating', 'ready', 'error', etc.
- `container_id`, `container_status`
- Resource limits: `cpu_limit`, `memory_limit`, `disk_limit`

### init_scripts
Custom initialization scripts for workspace containers.

**Key Fields:**
- `workspace_id` (FK to workspaces, one-to-one)
- `script_content` - Shell script to execute
- `timeout_seconds` - Execution timeout (default: 300)
- `status` - 'Pending', 'Running', 'Success', 'Failed'
- `output_summary` - Last 4KB of output (stored in DB)
- `output_file_path` - Path to full log file (for outputs >4KB)

### tasks
Automated development tasks created from issues.

**Key Fields:**
- `workspace_id` (FK to workspaces)
- `issue_number` - Source issue number
- `issue_title`, `issue_body` - Issue content
- `issue_url` - Full URL to the issue
- `task_status` - 'Pending', 'Assigned', 'Running', 'Completed', 'Failed', 'Cancelled'
- `priority` - 'Low', 'Medium', 'High'
- `assigned_agent_id` (FK to agents, nullable) - Assigned AI agent
- `branch_name`, `pr_number`, `pr_url` (nullable) - Pull request information
- `error_message` (nullable) - Error details for failed tasks
- `retry_count`, `max_retries` - Automatic retry configuration (default: 0, 3)
- `started_at`, `completed_at` (nullable) - Execution timestamps
- `created_at`, `updated_at`, `deleted_at` (nullable) - Lifecycle timestamps
- Unique constraint on (workspace_id, issue_number) to prevent duplicates

## Architecture

### Module Hierarchy

```
Settings (namespace)
└── RepoProvider (entity)
    └── Repository (entity) [many-to-one]
        └── Workspace (entity) [one-to-one]
            ├── InitScript (entity) [one-to-one]
            ├── Agent (entity) [one-to-many]
            └── Task (entity) [one-to-many]
```

### Git Provider Abstraction

Unified interface for different Git platforms:

```rust
use vibe_repo::git_provider::{GitProvider, GitClientFactory};

// Create a client from a RepoProvider entity
let client = GitClientFactory::from_provider(&provider)?;

// Use the unified interface
let repos = client.list_repositories(None).await?;
let branches = client.list_branches("owner", "repo").await?;
```

**Supported Operations:**
- Repository operations (list, get)
- Branch operations (list, get, create, delete)
- Issue operations (list, get, create, update, add/remove labels)
- Pull request operations (list, get, create, update, merge)
- Label operations (list, create, delete)
- Webhook operations (create, delete, list)

## Testing Philosophy

This project follows **Test-Driven Development (TDD)**:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Refactor code while keeping tests passing

**Test Coverage (v0.3.0):**
- Total tests: 398
- Passing: 100%
- Unit tests: 379 (including issue polling service tests)
- Integration tests: 19 (including polling API tests)
- Property tests: 14

## Contributing

### Development Standards

- **Language**: English for all code, comments, and documentation
- **Commit Messages**: Follow Conventional Commits specification
- **Code Style**: Run `cargo fmt` before committing
- **Testing**: Write tests first (TDD approach)
- **Breaking Changes**: Allowed before v1.0.0

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

**Scopes:** `api`, `db`, `deps`, `test`, `docs`

**Examples:**
```bash
feat(api): Add repository initialization feature
fix(db): Fix unique constraint validation logic
test(api): Add webhook integration tests
```

## Roadmap

### Current Status (v0.3.0)

**Completed:**
- ✅ Backend Foundation
- ✅ RepoProvider API
- ✅ Repository API
- ✅ Git Provider Abstraction (Gitea)
- ✅ Repository Initialization
- ✅ Webhook Integration
- ✅ Workspace API
- ✅ Init Script Feature
- ✅ Container Lifecycle Management
- ✅ Agent Management
- ✅ Task Automation (Complete Task API with 12 endpoints)
- ✅ Issue Polling with Intelligent Filtering
- ✅ Dual-Mode Issue Tracking (Webhook + Polling)

**In Progress:**
- 🟡 GitHub/GitLab provider implementations
- 🟡 Issue-to-PR Workflow

**Planned:**
- 📋 Advanced Task Scheduling
- 📋 Multi-Agent Coordination

## License

[Add your license here]

## Support

- **Documentation**: See [AGENTS.md](./AGENTS.md) for detailed development guidelines
- **Issues**: Report bugs and feature requests on GitHub Issues
- **API Docs**: Access Swagger UI at `http://localhost:3000/swagger-ui`

## Acknowledgments

Built with Rust and powered by:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SeaORM](https://www.sea-ql.org/SeaORM/) - Database ORM
- [Tokio](https://tokio.rs/) - Async runtime
- [utoipa](https://github.com/juhaku/utoipa) - OpenAPI documentation
