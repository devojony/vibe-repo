# Migration Guide: v0.3.0 → v0.4.0-mvp

This document explains how to migrate from the full-featured v0.3.0 to the simplified MVP v0.4.0.

## ⚠️ Breaking Changes

### Removed Features

The following features have been completely removed in v0.4.0-mvp:

1. **Issue Polling Service** - Automatic issue polling is no longer supported. Use webhooks instead.
2. **Webhook Retry Service** - Failed webhooks are not automatically retried.
3. **Init Script Service** - Custom initialization scripts are not supported.
4. **WebSocket Real-time Logs** - Real-time log streaming via WebSocket has been removed. Use the REST API to fetch logs.
5. **Task Failure Analyzer** - Automatic failure analysis is not available.
6. **Task Execution History** - Historical execution records are not stored. Only the last log is kept.
7. **Health Check Service** - Dedicated health check endpoints have been removed.
8. **Image Management Service** - Docker image management is simplified.
9. **Task Retry Mechanism** - Failed tasks cannot be automatically retried.
10. **Assigned State** - Tasks go directly from Pending to Running (no intermediate Assigned state).

### Removed API Endpoints

The following API endpoints have been removed:

- `POST /api/settings/providers` - Provider management
- `GET /api/settings/providers` - List providers
- `DELETE /api/settings/providers/:id` - Delete provider
- `POST /api/workspaces` - Workspace management
- `GET /api/workspaces` - List workspaces
- `DELETE /api/workspaces/:id` - Delete workspace
- `POST /api/agents` - Agent management
- `GET /api/agents` - List agents
- `DELETE /api/agents/:id` - Delete agent
- `POST /api/webhooks/config` - Webhook configuration
- `GET /api/webhooks/config` - Get webhook config
- `WS /api/tasks/:id/logs/stream` - WebSocket log streaming

### Remaining API Endpoints (8 Core Endpoints)

1. `POST /api/repositories` - Create repository
2. `POST /api/webhooks/github` - Receive GitHub webhook
3. `GET /api/tasks` - List tasks with filters
4. `POST /api/tasks` - Create task manually
5. `GET /api/tasks/:id` - Get task details
6. `POST /api/tasks/:id/execute` - Execute task
7. `GET /api/tasks/:id/logs` - Get task logs (last_log field)
8. `DELETE /api/tasks/:id` - Delete task

## 📋 Migration Steps

### 1. Update Configuration

**Before (v0.3.0):**
- Providers configured via API
- Webhooks configured via API
- Agents configured via API

**After (v0.4.0-mvp):**
All configuration is done via environment variables:

```bash
# .env file
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Git Provider Configuration
GITHUB_TOKEN=your_github_token_here
GITHUB_BASE_URL=https://api.github.com
WEBHOOK_SECRET=your_webhook_secret_here

# Agent Configuration
DEFAULT_AGENT_COMMAND=opencode
DEFAULT_AGENT_TIMEOUT=600
DEFAULT_DOCKER_IMAGE=ubuntu:22.04

# Workspace Configuration
WORKSPACE_BASE_DIR=./data/vibe-repo/workspaces

# Logging
RUST_LOG=info
LOG_FORMAT=human
```

### 2. Database Migration

The database schema has been simplified. Run the migration:

```bash
cd backend
cargo run
```

The migration will automatically:
- Drop `repo_providers` table (use environment variables)
- Drop `webhook_configs` table (use environment variables)
- Drop `init_scripts` table (use default setup)
- Drop `task_executions` table (use tasks.last_log)
- Add `last_log` field to `tasks` table
- Remove `retry_count` and `max_retries` from `tasks` table
- Add agent configuration fields to `repositories` table
- Add unique constraint on `agents.workspace_id`

### 3. Update Client Code

If you have client code that uses the removed API endpoints, you'll need to update it:

**Provider Management:**
```diff
- POST /api/settings/providers
+ Configure via GITHUB_TOKEN environment variable
```

**Workspace Management:**
```diff
- POST /api/workspaces
+ Workspaces are automatically created when repository is initialized
```

**Agent Management:**
```diff
- POST /api/agents
+ Agents are automatically created with workspace (one per workspace)
```

**Real-time Logs:**
```diff
- WS /api/tasks/:id/logs/stream
+ GET /api/tasks/:id/logs (returns last_log field)
```

**Task Retry:**
```diff
- POST /api/tasks/:id/retry
+ Manually create a new task or re-execute the existing one
```

### 4. Update Workflow

**Before (v0.3.0):**
1. Create provider via API
2. Create repository via API
3. Create workspace via API
4. Create agent via API
5. Configure webhook via API
6. Receive webhook → Create task → Execute task

**After (v0.4.0-mvp):**
1. Configure environment variables (GITHUB_TOKEN, WEBHOOK_SECRET, etc.)
2. Create repository via API (workspace and agent are auto-created)
3. Configure webhook in GitHub (use WEBHOOK_SECRET)
4. Receive webhook → Create task → Execute task

## 🔄 Rollback

If you need to rollback to v0.3.0:

1. Stop the v0.4.0-mvp server
2. Restore your database backup (if you made one)
3. Checkout the v0.3.0 branch
4. Restart the server

**Note:** Database migrations are not reversible. Make sure to backup your database before upgrading.

## 📚 Additional Resources

- [README.md](./README.md) - Overview of v0.4.0-mvp
- [CHANGELOG.md](./CHANGELOG.md) - Detailed changelog
- [docs/api/user-guide.md](./docs/api/user-guide.md) - Updated user guide
- [docs/api/api-reference.md](./docs/api/api-reference.md) - Updated API reference

## 🆘 Support

If you encounter issues during migration:

1. Check the [CHANGELOG.md](./CHANGELOG.md) for detailed changes
2. Review the [docs/api/user-guide.md](./docs/api/user-guide.md) for updated workflows
3. Open an issue on GitHub with the `migration` label

---

**Last Updated:** 2026-02-06  
**Version:** 0.4.0-mvp
