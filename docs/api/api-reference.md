# API Reference

**Version:** 0.4.0-mvp (Simplified MVP)  
**Last Updated:** 2026-02-06

> **🎯 Simplified MVP**: This version includes only 10 core API endpoints. Many management APIs have been removed in favor of environment-based configuration.

This document provides a complete reference for all VibeRepo API endpoints.

## Base URL

```
http://localhost:3000
```

## Authentication

Currently, VibeRepo does not require authentication for API access. Authentication will be added in future versions.

## API Documentation

Interactive API documentation is available at:
- **Swagger UI**: `http://localhost:3000/swagger-ui`
- **OpenAPI Spec**: `http://localhost:3000/api-docs/openapi.json`

---

## Health Check

### GET /health

Service health status with database connectivity check.

**Response:**
```json
{
  "status": "healthy",
  "database": "connected",
  "version": "0.4.0-mvp"
}
```

---

## Repository Module

### GET /api/repositories

List all repositories.

**Response:**
```json
[
  {
    "id": 1,
    "name": "my-repo",
    "full_name": "owner/my-repo",
    "clone_url": "https://github.com/owner/my-repo.git",
    "default_branch": "main",
    "validation_status": "valid",
    "created_at": "2026-02-06T10:00:00Z"
  }
]
```

### GET /api/repositories/:id

Get repository details.

**Response:**
```json
{
  "id": 1,
  "name": "my-repo",
  "full_name": "owner/my-repo",
  "clone_url": "https://github.com/owner/my-repo.git",
  "default_branch": "main",
  "branches": ["main", "vibe-dev"],
  "validation_status": "valid",
  "has_required_branches": true,
  "has_required_labels": true,
  "can_manage_prs": true,
  "can_manage_issues": true,
  "created_at": "2026-02-06T10:00:00Z",
  "updated_at": "2026-02-06T10:00:00Z"
}
```

### POST /api/repositories/:id/initialize

Initialize repository with vibe-dev branch and required labels.

**Request:**
```json
{
  "branch_name": "vibe-dev",
  "create_labels": true
}
```

**Response:** `200 OK`
```json
{
  "id": 1,
  "name": "my-repo",
  "validation_status": "valid",
  "has_required_branches": true,
  "has_required_labels": true
}
```

### POST /api/repositories/batch-initialize

Batch initialize multiple repositories.

**Request:**
```json
{
  "repository_ids": [1, 2, 3],
  "branch_name": "vibe-dev",
  "create_labels": true
}
```

**Response:** `200 OK`
```json
{
  "total": 3,
  "succeeded": 3,
  "failed": 0,
  "results": [
    {
      "repository_id": 1,
      "status": "success"
    },
    {
      "repository_id": 2,
      "status": "success"
    },
    {
      "repository_id": 3,
      "status": "success"
    }
  ]
}
```

---

## Webhook Module

### POST /api/webhooks/:repository_id

Receive webhook events from Git providers.

**Headers:**
- `X-Hub-Signature-256` - GitHub webhook signature for verification
- `X-Gitea-Signature` - Gitea webhook signature for verification

**Request Body:**
GitHub/Gitea webhook payload (varies by event type)

**Response:** `200 OK`
```json
{
  "status": "processed",
  "event_type": "issues",
  "action": "opened"
}
```

---

## Task Module

### POST /api/tasks

Create a new task from an issue.

**Request:**
```json
{
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "issue_body": "Implement JWT-based authentication...",
  "issue_url": "https://github.com/owner/repo/issues/42",
  "priority": "High"
}
```

**Response:** `201 Created`
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "issue_body": "Implement JWT-based authentication...",
  "issue_url": "https://github.com/owner/repo/issues/42",
  "task_status": "pending",
  "priority": "High",
  "created_at": "2026-02-06T10:00:00Z"
}
```

**Note:** Tasks are automatically assigned to the workspace's agent. The `assigned_agent_id` field has been removed.

### GET /api/tasks

List tasks with optional filters.

**Query Parameters:**
- `workspace_id` (required) - Filter by workspace
- `status` (optional) - Filter by status (pending/running/completed/failed/cancelled)
- `priority` (optional) - Filter by priority (Low/Medium/High)

**Response:**
```json
[
  {
    "id": 1,
    "workspace_id": 1,
    "issue_number": 42,
    "issue_title": "Add user authentication",
    "task_status": "pending",
    "priority": "High",
    "last_log": null,
    "pr_number": null,
    "pr_url": null,
    "created_at": "2026-02-06T10:00:00Z"
  }
]
```

### GET /api/tasks/:id

Get task details including last log.

**Response:**
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "issue_body": "Implement JWT-based authentication...",
  "issue_url": "https://github.com/owner/repo/issues/42",
  "task_status": "completed",
  "priority": "High",
  "last_log": "Task completed successfully. PR #123 created.",
  "branch_name": "feature/user-auth",
  "pr_number": 123,
  "pr_url": "https://github.com/owner/repo/pull/123",
  "error_message": null,
  "started_at": "2026-02-06T10:05:00Z",
  "completed_at": "2026-02-06T10:10:00Z",
  "created_at": "2026-02-06T10:00:00Z",
  "updated_at": "2026-02-06T10:10:00Z"
}
```

### PATCH /api/tasks/:id

Update task priority.

**Request:**
```json
{
  "priority": "High"
}
```

**Response:** `200 OK`
```json
{
  "id": 1,
  "priority": "High",
  "updated_at": "2026-02-06T10:15:00Z"
}
```

**Note:** The `assigned_agent_id` field has been removed from update requests.

### DELETE /api/tasks/:id

Soft delete a task.

**Response:** `204 No Content`

### POST /api/tasks/:id/execute

Execute task in workspace container with assigned agent.

**Response:** `202 Accepted`
```json
{
  "id": 1,
  "task_status": "running",
  "started_at": "2026-02-06T10:05:00Z"
}
```

### POST /api/tasks/:id/create-pr

Manually create a pull request for a completed task.

**Requirements:**
- Task must be in "Completed" status
- Task must have `branch_name` set
- Repository must be accessible with current credentials

**Response:** `200 OK`
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "task_status": "completed",
  "pr_number": 123,
  "pr_url": "https://github.com/owner/repo/pull/123",
  "branch_name": "feature/user-auth"
}
```

### POST /api/tasks/:id/close-issue

Manually close the issue associated with a task.

**Requirements:**
- Task must have `issue_number` set
- Issue must exist in the repository
- Repository must be accessible with current credentials

**Response:** `200 OK`
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "task_status": "completed"
}
```

---

## Removed Endpoints (from v0.3.0)

The following endpoints were removed in the simplified MVP:

### Settings Module
- ~~GET /api/settings/providers~~ (configured via environment variables)
- ~~POST /api/settings/providers~~ (configured via environment variables)
- ~~GET /api/settings/providers/:id~~ (configured via environment variables)
- ~~PUT /api/settings/providers/:id~~ (configured via environment variables)
- ~~DELETE /api/settings/providers/:id~~ (configured via environment variables)
- ~~POST /api/settings/providers/:id/validate~~ (configured via environment variables)
- ~~POST /api/settings/providers/:id/sync~~ (configured via environment variables)
- ~~GET /api/settings/workspace/image~~ (use default Docker images)
- ~~DELETE /api/settings/workspace/image~~ (use default Docker images)
- ~~POST /api/settings/workspace/image/rebuild~~ (use default Docker images)

### Repository Module
- ~~POST /api/repositories/:id/refresh~~ (validation happens automatically)
- ~~PATCH /api/repositories/:id/polling~~ (no issue polling service)
- ~~POST /api/repositories/:id/poll-issues~~ (no issue polling service)

### Workspace Module
- ~~POST /api/workspaces~~ (workspaces created automatically)
- ~~GET /api/workspaces~~ (workspaces created automatically)
- ~~GET /api/workspaces/:id~~ (workspaces created automatically)
- ~~PUT /api/workspaces/:id/status~~ (workspaces created automatically)
- ~~DELETE /api/workspaces/:id~~ (workspaces created automatically)
- ~~POST /api/workspaces/:id/restart~~ (simplified container management)
- ~~GET /api/workspaces/:id/stats~~ (simplified container management)

### Init Script Module
- ~~PUT /api/workspaces/:id/init-script~~ (no custom init scripts)
- ~~GET /api/workspaces/:id/init-script/logs~~ (no custom init scripts)
- ~~GET /api/workspaces/:id/init-script/logs/full~~ (no custom init scripts)
- ~~POST /api/workspaces/:id/init-script/execute~~ (no custom init scripts)

### Agent Module
- ~~POST /api/agents~~ (agents configured via environment variables)
- ~~GET /api/agents/:id~~ (agents configured via environment variables)
- ~~GET /api/workspaces/:workspace_id/agents~~ (agents configured via environment variables)
- ~~PATCH /api/agents/:id/enabled~~ (agents configured via environment variables)
- ~~DELETE /api/agents/:id~~ (agents configured via environment variables)

### Task Module
- ~~PATCH /api/tasks/:id/status~~ (status managed automatically)
- ~~POST /api/tasks/:id/assign~~ (automatic assignment)
- ~~POST /api/tasks/:id/start~~ (use /execute instead)
- ~~POST /api/tasks/:id/complete~~ (completed automatically)
- ~~POST /api/tasks/:id/fail~~ (failed automatically)
- ~~POST /api/tasks/:id/retry~~ (no retry mechanism)
- ~~POST /api/tasks/:id/cancel~~ (simplified cancellation)
- ~~GET /api/tasks/:id/logs/stream~~ (no WebSocket streaming)
- ~~GET /api/tasks/:id/failure-analysis~~ (no failure analyzer)

---

## Task Status Enum

Task status is represented as a type-safe enum with state machine validation:

**Possible Values:**
- `pending` - Task created, waiting for execution
- `running` - Task execution in progress
- `completed` - Task successfully completed with PR created
- `failed` - Task failed
- `cancelled` - Task manually cancelled

**State Transitions:**
- `pending` → `running`, `cancelled`
- `running` → `completed`, `failed`, `cancelled`
- `completed`, `failed`, and `cancelled` are terminal states

**Removed States:**
- ~~`assigned`~~ (tasks go directly from pending to running)

---

## Error Responses

All endpoints return errors in the following format:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": {
    "field": "Additional information"
  }
}
```

### HTTP Status Codes

- `200 OK` - Request succeeded
- `201 Created` - Resource created
- `202 Accepted` - Request accepted (async operation)
- `204 No Content` - Request succeeded with no content
- `400 Bad Request` - Invalid request
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource conflict
- `500 Internal Server Error` - Server error
- `503 Service Unavailable` - Service temporarily unavailable

---

## Rate Limiting

Currently, there is no rate limiting. This will be added in future versions.

---

## Pagination

List endpoints support pagination:

**Query Parameters:**
- `page` (optional) - Page number (default: 1)
- `per_page` (optional) - Items per page (default: 20, max: 100)

**Response Headers:**
- `X-Total-Count` - Total number of items
- `X-Page` - Current page
- `X-Per-Page` - Items per page

---

## Related Documentation

- **[User Guide](./user-guide.md)** - Complete usage guide
- **[Database Schema](../database/schema.md)** - Simplified database schema
- **[Migration Guide](../../MIGRATION.md)** - Migrating from v0.3.0 to v0.4.0-mvp
- **[Development Guide](../development/README.md)** - Development guidelines

---

**Last Updated:** 2026-02-06  
**API Version:** 0.4.0-mvp (Simplified MVP)
