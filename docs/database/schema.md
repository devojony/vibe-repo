# Database Schema

**Version:** 0.3.0

This document describes the complete database schema for VibeRepo.

## Overview

VibeRepo uses SeaORM 1.1 as the ORM layer, supporting both SQLite (development) and PostgreSQL (production). All migrations run automatically on application startup.

## Entity Relationships

```
Settings (namespace)
└── RepoProvider (entity)
    └── Repository (entity) [many-to-one]
        ├── WebhookConfig (entity) [one-to-one]
        └── Workspace (entity) [one-to-one]
            ├── InitScript (entity) [one-to-one]
            ├── Agent (entity) [one-to-many]
            └── Task (entity) [one-to-many]
                └── TaskExecution (entity) [one-to-many]
```

## Tables

### repo_providers

Git provider configurations with authentication credentials.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `name` (TEXT, NOT NULL) - Provider name
- `type` (TEXT, NOT NULL) - Provider type ('gitea', 'github', 'gitlab')
- `base_url` (TEXT, NOT NULL) - Provider base URL
- `access_token` (TEXT, NOT NULL) - Authentication token (masked in API responses)
- `locked` (BOOLEAN, DEFAULT false) - Prevents deletion when true
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Constraints:**
- UNIQUE (name, base_url, access_token) - Prevents duplicate providers

**Relationships:**
- One-to-many with `repositories` (CASCADE DELETE)
- One-to-many with `webhook_configs` (CASCADE DELETE)

**Notes:**
- Access tokens are masked in API responses (first 8 chars + `***`)
- Locked providers cannot be deleted
- Currently only 'gitea' type is fully implemented

---

### repositories

Repository records with validation status and polling configuration.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `provider_id` (INTEGER, FOREIGN KEY → repo_providers.id) - Associated provider
- `name` (TEXT, NOT NULL) - Repository name
- `full_name` (TEXT, NOT NULL) - Full repository name (owner/repo)
- `clone_url` (TEXT, NOT NULL) - Git clone URL
- `default_branch` (TEXT, NOT NULL) - Default branch name
- `branches` (JSON) - Array of branch names
- `validation_status` (TEXT) - Validation status ('valid', 'invalid', 'pending')
- `validation_message` (TEXT, NULLABLE) - Validation error message
- `has_required_branches` (BOOLEAN) - vibe-dev branch exists
- `has_required_labels` (BOOLEAN) - vibe/* labels exist
- `can_manage_prs` (BOOLEAN) - Token has PR permissions
- `can_manage_issues` (BOOLEAN) - Token has Issue permissions
- `validation_message` (TEXT, NULLABLE) - Validation error message
- `webhook_status` (TEXT, DEFAULT 'not_configured') - Webhook configuration status
- `polling_enabled` (BOOLEAN, DEFAULT false) - Enable issue polling
- `polling_interval_seconds` (INTEGER, NULLABLE) - Polling interval (60-86400 seconds)
- `last_issue_poll_at` (TIMESTAMP, NULLABLE) - Last issue poll timestamp
- `deleted_at` (TIMESTAMP, NULLABLE) - Soft delete timestamp
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Relationships:**
- Many-to-one with `repo_providers` (CASCADE DELETE)
- One-to-one with `workspaces` (CASCADE DELETE)
- One-to-one with `webhook_configs` (CASCADE DELETE)

---

### webhook_configs

Webhook configurations for repository event monitoring.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `provider_id` (INTEGER, FOREIGN KEY → repo_providers.id) - Associated provider (redundant for performance)
- `repository_id` (INTEGER, FOREIGN KEY → repositories.id) - Associated repository
- `webhook_id` (TEXT, NOT NULL) - Provider's webhook ID
- `webhook_secret` (TEXT, NOT NULL) - Secret for signature verification
- `webhook_url` (TEXT, NOT NULL) - Full webhook URL
- `events` (TEXT, NOT NULL) - JSON array of subscribed events
- `enabled` (BOOLEAN, DEFAULT true) - Webhook enabled status
- `retry_count` (INTEGER, DEFAULT 0) - Failed delivery retry count
- `last_retry_at` (TIMESTAMP, NULLABLE) - Last retry timestamp
- `next_retry_at` (TIMESTAMP, NULLABLE) - Next scheduled retry
- `last_error` (TEXT, NULLABLE) - Last error message
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Constraints:**
- UNIQUE (repository_id) - One webhook per repository

**Relationships:**
- **Primary**: One-to-one with `repositories` (CASCADE DELETE)
- **Secondary**: Many-to-one with `repo_providers` (CASCADE DELETE, redundant for performance)

**Webhook URL Format:**
```
https://vibe-repo.example.com/api/webhooks/{repository_id}
```

**Design Rationale:**
- Webhooks are per-repository in Git providers (Gitea/GitHub/GitLab)
- `repository_id` in URL enables direct lookup without database queries
- `provider_id` is redundant but kept for performance (cascade delete, fast queries)

---

### workspaces

Docker-based isolated development environments for repositories.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `repository_id` (INTEGER, FOREIGN KEY → repositories.id) - Associated repository
- `workspace_status` (TEXT) - Status ('creating', 'ready', 'error', 'failed')
- `container_id` (TEXT, NULLABLE) - Docker container ID
- `container_status` (TEXT, NULLABLE) - Container status
- `max_concurrent_tasks` (INTEGER, DEFAULT 3) - Maximum concurrent task executions
- `restart_count` (INTEGER, DEFAULT 0) - Container restart count
- `max_restart_attempts` (INTEGER, DEFAULT 3) - Maximum restart attempts
- `cpu_limit` (TEXT, NULLABLE) - CPU limit (e.g., "2.0")
- `memory_limit` (TEXT, NULLABLE) - Memory limit (e.g., "2g")
- `disk_limit` (TEXT, NULLABLE) - Disk limit (e.g., "10g")
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Relationships:**
- One-to-one with `repositories` (CASCADE DELETE)
- One-to-one with `init_scripts` (CASCADE DELETE)
- One-to-many with `agents` (CASCADE DELETE)
- One-to-many with `tasks` (CASCADE DELETE)

**Workspace Status Values:**
- `creating` - Workspace is being created
- `ready` - Workspace is ready for use
- `error` - Temporary error, will retry
- `failed` - Permanent failure after max retries

---

### init_scripts

Custom initialization scripts for workspace containers.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `workspace_id` (INTEGER, FOREIGN KEY → workspaces.id) - Associated workspace
- `script_content` (TEXT, NOT NULL) - Shell script to execute
- `timeout_seconds` (INTEGER, DEFAULT 300) - Execution timeout
- `status` (TEXT) - Execution status ('Pending', 'Running', 'Success', 'Failed')
- `output_summary` (TEXT, NULLABLE) - Last 4KB of output (stored in DB)
- `output_file_path` (TEXT, NULLABLE) - Path to full log file (for outputs >4KB)
- `executed_at` (TIMESTAMP, NULLABLE) - Execution timestamp
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Relationships:**
- One-to-one with `workspaces` (CASCADE DELETE)

**Storage Strategy:**
- Small outputs (≤4KB): Stored in `output_summary`
- Large outputs (>4KB): Summary in `output_summary`, full content in file at `output_file_path`
- Log files location: `./data/vibe-repo/init-logs/`

---

### containers

Docker container instances for workspaces.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `workspace_id` (INTEGER, FOREIGN KEY → workspaces.id, UNIQUE) - Associated workspace
- `container_id` (TEXT, NOT NULL, UNIQUE) - Docker container ID
- `container_name` (TEXT, NOT NULL) - Container name (format: `workspace-{workspace_id}`)
- `image_name` (TEXT, NOT NULL) - Docker image name
- `image_id` (TEXT, NULLABLE) - Docker image ID
- `status` (TEXT, NOT NULL, DEFAULT 'creating') - Container status
- `health_status` (TEXT, NULLABLE) - Health check status
- `exit_code` (INTEGER, NULLABLE) - Container exit code
- `error_message` (TEXT, NULLABLE) - Error details for failed containers
- `restart_count` (INTEGER, NOT NULL, DEFAULT 0) - Number of restart attempts
- `max_restart_attempts` (INTEGER, NOT NULL, DEFAULT 3) - Maximum restart attempts
- `last_restart_at` (TIMESTAMP, NULLABLE) - Last restart timestamp
- `last_health_check` (TIMESTAMP, NULLABLE) - Last health check timestamp
- `health_check_failures` (INTEGER, NOT NULL, DEFAULT 0) - Consecutive health check failures
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp
- `started_at` (TIMESTAMP, NULLABLE) - Container start timestamp
- `stopped_at` (TIMESTAMP, NULLABLE) - Container stop timestamp

**Relationships:**
- One-to-one with `workspaces` (CASCADE DELETE)

**Container Status Values:**
- `creating` - Container is being created
- `running` - Container is running normally
- `stopped` - Container was stopped manually
- `exited` - Container exited (may be restarted)
- `failed` - Container failed and exceeded restart limits

**Health Status Values:**
- `Healthy` - Container is healthy
- `Unhealthy` - Container failed health checks
- `Unknown` - Health status unknown

**Indices:**
- `idx_containers_workspace_id` - Fast workspace lookups
- `idx_containers_status` - Filter by status
- `idx_containers_container_id` - Fast Docker ID lookups

---

### agents

AI agent configurations for workspaces.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `workspace_id` (INTEGER, FOREIGN KEY → workspaces.id) - Associated workspace
- `name` (TEXT, NOT NULL) - Agent name
- `tool_type` (TEXT, NOT NULL) - AI tool type ('opencode', 'aider', etc.)
- `command` (TEXT, NOT NULL) - Command to execute in container
- `env_vars` (TEXT, NULLABLE) - JSON environment variables
- `timeout` (INTEGER, DEFAULT 1800) - Execution timeout in seconds
- `enabled` (BOOLEAN, DEFAULT true) - Agent enabled status
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Relationships:**
- Many-to-one with `workspaces` (CASCADE DELETE)
- One-to-many with `tasks` (via `assigned_agent_id`)
- One-to-many with `task_executions` (via `agent_id`)

**Environment Variables (JSON):**
```json
{
  "ANTHROPIC_API_KEY": "sk-ant-...",
  "MODEL": "claude-3.5-sonnet"
}
```

---

### tasks

Automated development tasks created from issues.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `workspace_id` (INTEGER, FOREIGN KEY → workspaces.id) - Associated workspace
- `issue_number` (INTEGER, NOT NULL) - Source issue number
- `issue_title` (TEXT, NOT NULL) - Issue title
- `issue_body` (TEXT, NULLABLE) - Issue description
- `task_status` (TaskStatus enum, stored as TEXT) - Task status with state machine validation
- `priority` (TEXT, DEFAULT 'Medium') - Priority level ('Low', 'Medium', 'High')
- `assigned_agent_id` (INTEGER, FOREIGN KEY → agents.id, NULLABLE) - Assigned AI agent
- `branch_name` (TEXT, NULLABLE) - Git branch name
- `pr_number` (INTEGER, NULLABLE) - Pull request number
- `pr_url` (TEXT, NULLABLE) - Pull request URL
- `error_message` (TEXT, NULLABLE) - Error details for failed tasks
- `retry_count` (INTEGER, DEFAULT 0) - Current retry count
- `max_retries` (INTEGER, DEFAULT 3) - Maximum retry attempts
- `started_at` (TIMESTAMP, NULLABLE) - Execution start timestamp
- `completed_at` (TIMESTAMP, NULLABLE) - Completion timestamp
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp
- `deleted_at` (TIMESTAMP, NULLABLE) - Soft delete timestamp

**Constraints:**
- UNIQUE (workspace_id, issue_number) - Prevent duplicate tasks for same issue

**Relationships:**
- Many-to-one with `workspaces` (CASCADE DELETE)
- Many-to-one with `agents` (via `assigned_agent_id`, SET NULL on delete)
- One-to-many with `task_executions` (CASCADE DELETE)

**Task Status Values (TaskStatus Enum):**

The `task_status` field uses a type-safe enum with state machine validation. Values are stored as lowercase strings in the database.

- `pending` - Task created, waiting to be assigned
- `assigned` - Agent assigned, ready to start
- `running` - Task execution in progress
- `completed` - Task successfully completed with PR created
- `failed` - Task failed after exhausting retries
- `cancelled` - Task manually cancelled

**State Transition Rules:**
- `pending` → `assigned`, `cancelled`
- `assigned` → `running`, `cancelled`
- `running` → `completed`, `failed`, `cancelled`
- `failed` → `pending` (retry only, if retry_count < max_retries)
- `completed` and `cancelled` are terminal states (no further transitions)

**Priority Levels:**
- `High` - Critical tasks executed first
- `Medium` - Normal priority (default)
- `Low` - Tasks that can be deferred

---

### task_executions

Complete history of task execution attempts.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `task_id` (INTEGER, FOREIGN KEY → tasks.id) - Associated task
- `agent_id` (INTEGER, FOREIGN KEY → agents.id, NULLABLE) - Agent that executed the task
- `status` (TEXT) - Execution status ('running', 'completed', 'failed')
- `command` (TEXT, NOT NULL) - Full command executed in container
- `exit_code` (INTEGER, NULLABLE) - Process exit code
- `stdout_summary` (TEXT, NULLABLE) - stdout summary (≤4KB)
- `stderr_summary` (TEXT, NULLABLE) - stderr summary (≤4KB)
- `stdout_file_path` (TEXT, NULLABLE) - Full stdout log file path (>4KB)
- `stderr_file_path` (TEXT, NULLABLE) - Full stderr log file path (>4KB)
- `error_message` (TEXT, NULLABLE) - Error details
- `pr_number` (INTEGER, NULLABLE) - Pull request number
- `pr_url` (TEXT, NULLABLE) - Pull request URL
- `branch_name` (TEXT, NULLABLE) - Git branch name
- `duration_ms` (INTEGER, NULLABLE) - Execution duration in milliseconds
- `started_at` (TIMESTAMP, NULLABLE) - Execution start timestamp
- `completed_at` (TIMESTAMP, NULLABLE) - Execution completion timestamp
- `created_at` (TIMESTAMP) - Record creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Relationships:**
- Many-to-one with `tasks` (CASCADE DELETE)
- Many-to-one with `agents` (SET NULL on delete)

**Storage Strategy:**
- Small outputs (≤4KB): Stored in `stdout_summary` / `stderr_summary`
- Large outputs (>4KB): Summary in DB, full content in files
- Log files location: `./data/vibe-repo/task-logs/execution_{id}_{type}.log`

**Execution Status Values:**
- `running` - Execution in progress
- `completed` - Execution completed successfully
- `failed` - Execution failed

---

### task_logs

Structured logs for task execution with different log levels.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `task_id` (INTEGER, FOREIGN KEY → tasks.id) - Associated task
- `log_level` (TEXT, NOT NULL, DEFAULT 'Info') - Log level
- `message` (TEXT, NOT NULL) - Log message
- `metadata` (JSON, NULLABLE) - Additional structured data
- `created_at` (TIMESTAMP) - Log entry timestamp

**Relationships:**
- Many-to-one with `tasks` (CASCADE DELETE)

**Log Level Values:**
- `Debug` - Detailed debugging information
- `Info` - General informational messages
- `Warning` - Warning messages
- `Error` - Error messages
- `Critical` - Critical error messages

**Metadata (JSON):**
```json
{
  "execution_id": 123,
  "agent_id": 5,
  "step": "git_commit",
  "duration_ms": 1500
}
```

**Indices:**
- `idx_task_logs_task_id` - Fast task lookups
- `idx_task_logs_level` - Filter by log level
- `idx_task_logs_created_at` - Time-based queries

**Usage:**
- Real-time log streaming via WebSocket
- Historical log analysis
- Debugging task execution issues
- Performance monitoring

---

## Migrations

All migrations are located in `backend/src/migration/` and run automatically on application startup.

### Migration Files

- `m20240101_000001_init.rs` - Initial database setup
- `m20250114_000001_create_repo_providers.rs` - RepoProvider table
- `m20250114_000002_create_repositories.rs` - Repository table
- `m20250114_000003_add_provider_unique_constraint.rs` - Provider unique constraint
- `m20250117_000001_add_repository_status_and_soft_delete.rs` - Repository status and soft delete
- `m20260117_000001_create_workspaces.rs` - Workspace table
- `m20260117_000002_create_agents.rs` - Agent table
- `m20260117_000003_create_webhook_configs.rs` - WebhookConfig table
- `m20260117_000004_add_repository_webhook_status.rs` - Repository webhook status
- `m20260117_000005_create_tasks.rs` - Task table
- `m20260117_000006_create_task_logs.rs` - TaskLog table
- `m20260118_000001_add_webhook_retry_fields.rs` - Webhook retry configuration
- `m20260119_000001_replace_dockerfile_with_init_script.rs` - InitScript table (replaces Dockerfile)
- `m20260120_000001_create_containers_table.rs` - Container table
- `m20260120_000002_add_repository_polling_fields.rs` - Repository polling fields
- `m20260120_000003_add_task_unique_constraint.rs` - Task unique constraint
- `m20260121_000001_create_task_executions.rs` - TaskExecution table

### Running Migrations

Migrations run automatically on application startup. To run manually:

```bash
cd backend
cargo run -- migrate
```

### Creating New Migrations

```bash
cd backend
sea-orm-cli migrate generate <migration_name>
```

---

## Indexes

### Performance Indexes

- `repo_providers.name` - Fast provider lookup by name
- `repositories.provider_id` - Fast repository lookup by provider
- `repositories.validation_status` - Fast filtering by validation status
- `webhook_configs.repository_id` - Fast webhook lookup (UNIQUE)
- `workspaces.repository_id` - Fast workspace lookup (UNIQUE)
- `tasks.workspace_id` - Fast task lookup by workspace
- `tasks.task_status` - Fast filtering by task status
- `tasks.priority` - Fast priority-based sorting
- `tasks.assigned_agent_id` - Fast agent assignment queries
- `task_executions.task_id` - Fast execution history lookup

---

## Data Retention

### Soft Deletes

- **tasks**: Soft deleted with `deleted_at` timestamp
- Soft deleted tasks are excluded from normal queries
- Preserved for audit trail and historical analysis

### Log Cleanup

- **init_scripts**: Logs older than 30 days automatically cleaned up
- **task_executions**: Logs older than 30 days automatically cleaned up
- Cleanup runs daily via background service

---

## Database Configuration

### SQLite (Development)

```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
```

### PostgreSQL (Production)

```bash
DATABASE_URL=postgresql://user:password@localhost:5432/vibe_repo
DATABASE_MAX_CONNECTIONS=20
```

---

## Best Practices

1. **Always use SeaORM** for database operations (no raw SQL)
2. **Use transactions** for multi-table operations
3. **Respect CASCADE DELETE** relationships
4. **Use soft deletes** for audit trails (tasks)
5. **Clean up old logs** regularly (init_scripts, task_executions)
6. **Index frequently queried fields** for performance
7. **Use JSON fields** for flexible configuration (polling_config, env_vars)

---

**Last Updated:** 2026-01-21  
**Schema Version:** 0.4.0
