# Database Schema

**Version:** 0.4.0-mvp (Simplified MVP)

> **🎯 Simplified MVP**: The database schema has been significantly simplified. Several tables have been removed in favor of environment-based configuration and inline storage.

This document describes the complete database schema for VibeRepo.

## Overview

VibeRepo uses SeaORM 1.1 as the ORM layer, supporting both SQLite (development) and PostgreSQL (production). All migrations run automatically on application startup.

## Entity Relationships

```
Repository (entity) [self-contained with provider config]
└── Workspace (entity) [one-to-one]
    ├── Agent (entity) [one-to-one, unique constraint]
    └── Task (entity) [one-to-many]
```

**Key Simplifications:**
- No separate Provider entity (configuration stored in repository)
- No WebhookConfig entity (webhook_secret stored in repository)
- No InitScript entity (workspaces use default setup)
- No TaskExecution entity (logs stored in tasks.last_log field)
- Single agent per workspace (enforced by unique constraint)

## Tables

### repositories

Repository records with self-contained provider configuration.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `provider_type` (TEXT, NOT NULL) - Git provider type ('github', 'gitea', 'gitlab')
- `provider_base_url` (TEXT, NOT NULL) - Provider API base URL
- `access_token` (TEXT, NOT NULL) - Personal access token for this repository
- `webhook_secret` (TEXT, NOT NULL) - Unique webhook secret for this repository
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
- `webhook_status` (TEXT) - Webhook status ('active', 'inactive', 'failed')
- `deleted_at` (TIMESTAMP, NULLABLE) - Soft delete timestamp
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Relationships:**
- One-to-one with `workspaces` (CASCADE DELETE)

**New Fields (v0.4.0-mvp):**
- `provider_type` - Git provider type (replaces provider_id foreign key)
- `provider_base_url` - Provider API base URL (replaces provider_id foreign key)
- `access_token` - Per-repository access token (replaces provider-level token)
- `webhook_secret` - Per-repository webhook secret (replaces webhook_configs table)
- `webhook_status` - Webhook status tracking (replaces webhook_configs table)

**Removed Fields (from v0.3.0):**
- ~~`provider_id`~~ (provider configuration now stored in repository)
- ~~`polling_enabled`~~ (no issue polling service)
- ~~`polling_interval_seconds`~~ (no issue polling service)
- ~~`last_issue_poll_at`~~ (no issue polling service)

**Security Notes:**
- `access_token` is stored in plaintext (encryption planned for future)
- `webhook_secret` is stored in plaintext (used for webhook verification)
- Neither field is ever returned in API responses

---

### workspaces

Docker-based isolated development environments for repositories.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `repository_id` (INTEGER, FOREIGN KEY → repositories.id) - Associated repository
- `workspace_status` (TEXT) - Status ('creating', 'ready', 'error', 'failed')
- `container_id` (TEXT, NULLABLE) - Docker container ID
- `container_status` (TEXT, NULLABLE) - Container status
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Relationships:**
- One-to-one with `repositories` (CASCADE DELETE)
- One-to-one with `agents` (CASCADE DELETE, unique constraint)
- One-to-many with `tasks` (CASCADE DELETE)

**Workspace Status Values:**
- `creating` - Workspace is being created
- `ready` - Workspace is ready for use
- `error` - Temporary error, will retry
- `failed` - Permanent failure after max retries

**Removed Fields (from v0.3.0):**
- ~~`max_concurrent_tasks`~~ (simplified concurrency control)
- ~~`restart_count`~~ (simplified container management)
- ~~`max_restart_attempts`~~ (simplified container management)
- ~~`cpu_limit`~~ (use default Docker settings)
- ~~`memory_limit`~~ (use default Docker settings)
- ~~`disk_limit`~~ (use default Docker settings)

---

### agents

AI agent configurations for workspaces.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `workspace_id` (INTEGER, FOREIGN KEY → workspaces.id, UNIQUE) - Associated workspace
- `name` (TEXT, NOT NULL) - Agent name
- `tool_type` (TEXT, NOT NULL) - AI tool type ('opencode', 'aider', etc.)
- `command` (TEXT, NOT NULL) - Command to execute in container
- `env_vars` (TEXT, NULLABLE) - JSON environment variables
- `timeout` (INTEGER, DEFAULT 1800) - Execution timeout in seconds
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp

**Constraints:**
- UNIQUE (workspace_id) - One agent per workspace

**Relationships:**
- One-to-one with `workspaces` (CASCADE DELETE)
- One-to-many with `tasks` (via workspace relationship)

**Environment Variables (JSON):**
```json
{
  "ANTHROPIC_API_KEY": "sk-ant-...",
  "MODEL": "claude-3.5-sonnet"
}
```

**Removed Fields (from v0.3.0):**
- ~~`enabled`~~ (agents are always enabled)

**Key Changes:**
- Added UNIQUE constraint on `workspace_id` (one agent per workspace)
- Agents are now configured via environment variables (DEFAULT_AGENT_COMMAND, etc.)

---

### tasks

Automated development tasks created from issues.

**Fields:**
- `id` (INTEGER, PRIMARY KEY) - Unique identifier
- `workspace_id` (INTEGER, FOREIGN KEY → workspaces.id) - Associated workspace
- `issue_number` (INTEGER, NOT NULL) - Source issue number
- `issue_title` (TEXT, NOT NULL) - Issue title
- `issue_body` (TEXT, NULLABLE) - Issue description
- `issue_url` (TEXT, NULLABLE) - Issue URL
- `task_status` (TaskStatus enum, stored as TEXT) - Task status with state machine validation
- `priority` (TEXT, DEFAULT 'Medium') - Priority level ('Low', 'Medium', 'High')
- `branch_name` (TEXT, NULLABLE) - Git branch name
- `pr_number` (INTEGER, NULLABLE) - Pull request number
- `pr_url` (TEXT, NULLABLE) - Pull request URL
- `error_message` (TEXT, NULLABLE) - Error details for failed tasks
- `last_log` (TEXT, NULLABLE) - Last execution log (inline storage)
- `started_at` (TIMESTAMP, NULLABLE) - Execution start timestamp
- `completed_at` (TIMESTAMP, NULLABLE) - Completion timestamp
- `created_at` (TIMESTAMP) - Creation timestamp
- `updated_at` (TIMESTAMP) - Last update timestamp
- `deleted_at` (TIMESTAMP, NULLABLE) - Soft delete timestamp

**Constraints:**
- UNIQUE (workspace_id, issue_number) - Prevent duplicate tasks for same issue

**Relationships:**
- Many-to-one with `workspaces` (CASCADE DELETE)

**Task Status Values (TaskStatus Enum):**

The `task_status` field uses a type-safe enum with state machine validation. Values are stored as lowercase strings in the database.

- `pending` - Task created, waiting for execution
- `running` - Task execution in progress
- `completed` - Task successfully completed with PR created
- `failed` - Task failed
- `cancelled` - Task manually cancelled

**State Transition Rules:**
- `pending` → `running`, `cancelled`
- `running` → `completed`, `failed`, `cancelled`
- `completed`, `failed`, and `cancelled` are terminal states

**Removed States (from v0.3.0):**
- ~~`assigned`~~ (tasks go directly from pending to running)

**Removed Fields (from v0.3.0):**
- ~~`assigned_agent_id`~~ (automatic assignment via workspace)
- ~~`retry_count`~~ (no retry mechanism)
- ~~`max_retries`~~ (no retry mechanism)

**New Fields (v0.4.0-mvp):**
- `last_log` (TEXT, NULLABLE) - Inline log storage (replaces task_executions table)

**Priority Levels:**
- `High` - Critical tasks executed first
- `Medium` - Normal priority (default)
- `Low` - Tasks that can be deferred

---

## Removed Tables (from v0.3.0)

The following tables were removed in the simplified MVP:

### ~~repo_providers~~

**Reason:** Git provider configuration moved to per-repository storage. Each repository now contains its own `provider_type`, `provider_base_url`, and `access_token` fields.

**Migration:** When adding a repository via `POST /api/repositories`, provide provider configuration in the request body instead of referencing a provider_id.

**Benefits:**
- Eliminates foreign key dependency and JOIN queries
- Enables per-repository token management (principle of least privilege)
- Supports mixed providers (GitHub + Gitea + GitLab in same workspace)
- Simplifies code: no need to fetch provider before creating GitClient

---

### ~~webhook_configs~~

**Reason:** Webhook configuration moved to per-repository storage. Each repository now contains its own `webhook_secret` and `webhook_status` fields.

**Migration:** Webhook secrets are automatically generated when adding a repository via `POST /api/repositories`. Each repository has a unique webhook secret.

**Benefits:**
- Eliminates separate table and foreign key
- Simplifies webhook verification: single table lookup
- Reduces query count in webhook handler (hot path)

---

### ~~init_scripts~~

**Reason:** Workspaces now use default Docker setup without custom init scripts.

**Migration:** Use default workspace setup. Custom initialization can be done via agent commands.

---

### ~~task_executions~~

**Reason:** Execution history simplified to inline log storage in `tasks.last_log` field.

**Migration:** Task logs are now stored directly in the `tasks` table. Only the last log is kept.

---

### ~~task_logs~~

**Reason:** Real-time log streaming removed. Logs stored inline in `tasks.last_log`.

**Migration:** Use `tasks.last_log` field for task output. No WebSocket streaming available.

---

### ~~containers~~

**Reason:** Container management simplified. Container info stored in `workspaces` table.

**Migration:** Container ID and status are now fields in the `workspaces` table.

---

## Migrations

All migrations are located in `backend/src/migration/` and run automatically on application startup.

> **⚠️ Pre-1.0 Migration Policy**: Before the official v1.0.0 release, we do not need to consider database migration compatibility. Breaking schema changes are allowed and expected as we iterate on the MVP design.

### Migration Files

- `m20240101_000001_init.rs` - Initial database setup
- `m20250114_000001_create_repo_providers.rs` - RepoProvider table (removed in v0.4.0-mvp)
- `m20250114_000002_create_repositories.rs` - Repository table
- `m20250114_000003_add_provider_unique_constraint.rs` - Provider unique constraint (removed in v0.4.0-mvp)
- `m20250117_000001_add_repository_status_and_soft_delete.rs` - Repository status and soft delete
- `m20260117_000001_create_workspaces.rs` - Workspace table
- `m20260117_000002_create_agents.rs` - Agent table
- `m20260117_000003_create_webhook_configs.rs` - WebhookConfig table (removed in v0.4.0-mvp)
- `m20260117_000004_add_repository_webhook_status.rs` - Repository webhook status (removed in v0.4.0-mvp)
- `m20260117_000005_create_tasks.rs` - Task table
- `m20260117_000006_create_task_logs.rs` - TaskLog table (removed in v0.4.0-mvp)
- `m20260118_000001_add_webhook_retry_fields.rs` - Webhook retry configuration (removed in v0.4.0-mvp)
- `m20260119_000001_replace_dockerfile_with_init_script.rs` - InitScript table (removed in v0.4.0-mvp)
- `m20260120_000001_create_containers_table.rs` - Container table (removed in v0.4.0-mvp)
- `m20260120_000002_add_repository_polling_fields.rs` - Repository polling fields (removed in v0.4.0-mvp)
- `m20260120_000003_add_task_unique_constraint.rs` - Task unique constraint
- `m20260121_000001_create_task_executions.rs` - TaskExecution table (removed in v0.4.0-mvp)
- **`m20260206_000001_simplify_mvp_schema.rs`** - Simplified MVP schema (v0.4.0-mvp)

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

- `repositories.validation_status` - Fast filtering by validation status
- `workspaces.repository_id` - Fast workspace lookup (UNIQUE)
- `agents.workspace_id` - Fast agent lookup (UNIQUE)
- `tasks.workspace_id` - Fast task lookup by workspace
- `tasks.task_status` - Fast filtering by task status
- `tasks.priority` - Fast priority-based sorting

**Removed Indexes (from v0.3.0):**
- ~~`repo_providers.name`~~ (no repo_providers table)
- ~~`repositories.provider_id`~~ (no provider_id field)
- ~~`webhook_configs.repository_id`~~ (no webhook_configs table)
- ~~`tasks.assigned_agent_id`~~ (no assigned_agent_id field)
- ~~`task_executions.task_id`~~ (no task_executions table)

---

## Data Retention

### Soft Deletes

- **tasks**: Soft deleted with `deleted_at` timestamp
- Soft deleted tasks are excluded from normal queries
- Preserved for audit trail and historical analysis

### Log Storage

- **tasks.last_log**: Stores the last execution log inline (no separate table)
- No automatic cleanup (logs are part of task records)

**Removed Features (from v0.3.0):**
- ~~Log cleanup service~~ (no separate log tables)
- ~~Execution history~~ (no task_executions table)

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
5. **Index frequently queried fields** for performance
6. **Use JSON fields** for flexible configuration (env_vars, branches)
7. **Use environment variables** for provider and webhook configuration

---

## Schema Comparison: v0.3.0 vs v0.4.0-mvp

| Feature | v0.3.0 | v0.4.0-mvp |
|---------|--------|------------|
| **Tables** | 8 tables | 4 tables |
| **Provider Config** | Database (repo_providers) | Per-repository fields |
| **Webhook Config** | Database (webhook_configs) | Per-repository fields |
| **Init Scripts** | Database (init_scripts) | Default setup only |
| **Task Logs** | Separate tables (task_executions, task_logs) | Inline (tasks.last_log) |
| **Agents per Workspace** | Many | One (unique constraint) |
| **Task States** | 6 states (including Assigned) | 5 states (no Assigned) |
| **Task Retry** | Automatic retry with retry_count | No retry mechanism |
| **Repository Addition** | Auto-sync from provider | Manual addition with config |
| **Token Management** | One token per provider | One token per repository |
| **Database Queries** | 2 per operation (repo + provider) | 1 per operation (repo only) |

---

## Related Documentation

- **[User Guide](../api/user-guide.md)** - Complete usage guide
- **[API Reference](../api/api-reference.md)** - All API endpoints
- **[Migration Guide](../../MIGRATION.md)** - Migrating from v0.3.0 to v0.4.0-mvp
- **[Development Guide](../development/README.md)** - Development guidelines

---

**Last Updated:** 2026-02-06  
**Schema Version:** 0.4.0-mvp (Simplified MVP)
