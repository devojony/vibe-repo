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

### POST /api/repositories

Manually add a repository with complete provider configuration.

**Request:**
```json
{
  "provider_type": "github",
  "provider_base_url": "https://api.github.com",
  "access_token": "ghp_xxxxxxxxxxxx",
  "full_name": "owner/my-repo",
  "branch_name": "vibe-dev"
}
```

**Request Fields:**
- `provider_type` (required) - Git provider type: "github", "gitea", or "gitlab"
- `provider_base_url` (required) - Provider API base URL
- `access_token` (required) - Personal access token with required permissions
- `full_name` (required) - Repository full name (owner/repo)
- `branch_name` (optional) - Branch name for automation (default: "vibe-dev")

**Response:** `201 Created`
```json
{
  "id": 1,
  "name": "my-repo",
  "full_name": "owner/my-repo",
  "clone_url": "https://github.com/owner/my-repo.git",
  "default_branch": "main",
  "branches": ["main", "vibe-dev"],
  "provider_type": "github",
  "provider_base_url": "https://api.github.com",
  "validation_status": "valid",
  "has_required_branches": true,
  "has_required_labels": true,
  "can_manage_prs": true,
  "can_manage_issues": true,
  "webhook_status": "active",
  "created_at": "2026-02-06T10:00:00Z",
  "updated_at": "2026-02-06T10:00:00Z"
}
```

**Error Responses:**
- `400 Bad Request` - Invalid provider_type or missing required fields
- `401 Unauthorized` - Invalid access token
- `403 Forbidden` - Token lacks required permissions
- `404 Not Found` - Repository not found on provider
- `500 Internal Server Error` - Failed to create workspace or webhook

**What This Endpoint Does:**
1. Validates the access token with the provider
2. Fetches repository information from the provider
3. Validates token permissions (branches, labels, PRs, issues, webhooks)
4. Generates a unique webhook secret
5. Creates repository record in database
6. Creates workspace and agent automatically
7. Initializes the vibe-dev branch (if it doesn't exist)
8. Creates required labels (vibe/pending-ack, vibe/todo-ai, etc.)
9. Creates webhook on the provider
10. Returns complete repository details

**Note:** This is an atomic operation - if any step fails, the entire operation is rolled back.

### GET /api/repositories

List all repositories.

**Query Parameters:**
- `validation_status` (optional) - Filter by validation status (valid/invalid/pending)
- `status` (optional) - Filter by repository status (idle/busy/error)

**Response:**
```json
[
  {
    "id": 1,
    "name": "my-repo",
    "full_name": "owner/my-repo",
    "clone_url": "https://github.com/owner/my-repo.git",
    "default_branch": "main",
    "provider_type": "github",
    "provider_base_url": "https://api.github.com",
    "validation_status": "valid",
    "webhook_status": "active",
    "created_at": "2026-02-06T10:00:00Z"
  }
]
```

**Note:** The `provider_id` filter has been removed. Each repository now contains its own provider configuration.

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
  "provider_type": "github",
  "provider_base_url": "https://api.github.com",
  "validation_status": "valid",
  "has_required_branches": true,
  "has_required_labels": true,
  "can_manage_prs": true,
  "can_manage_issues": true,
  "webhook_status": "active",
  "created_at": "2026-02-06T10:00:00Z",
  "updated_at": "2026-02-06T10:00:00Z"
}
```

**Security Note:** The `access_token` and `webhook_secret` fields are never included in API responses for security reasons.

### POST /api/repositories/:id/initialize

Re-initialize repository branch and labels (if needed after manual changes).

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

**Note:** This endpoint is typically not needed since `POST /api/repositories` automatically initializes the repository. Use this only if you need to re-create branches or labels after manual changes.

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

**Note:** This endpoint operates on repository IDs, not provider IDs. Provider-level batch operations have been removed.

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

### Settings Module (Removed - Use Environment Variables)
- ~~GET /api/settings/providers~~ - Provider configuration now per-repository
- ~~POST /api/settings/providers~~ - Use `POST /api/repositories` with provider config
- ~~GET /api/settings/providers/:id~~ - Provider info included in repository response
- ~~PUT /api/settings/providers/:id~~ - Update repository record directly
- ~~DELETE /api/settings/providers/:id~~ - No separate provider entity
- ~~POST /api/settings/providers/:id/validate~~ - Validation happens during repository creation
- ~~POST /api/settings/providers/:id/sync~~ - No auto-discovery, use `POST /api/repositories`
- ~~GET /api/settings/workspace/image~~ - Use default Docker images
- ~~DELETE /api/settings/workspace/image~~ - Use default Docker images
- ~~POST /api/settings/workspace/image/rebuild~~ - Use default Docker images

### Repository Module
- ~~POST /api/repositories/:id/refresh~~ - Validation happens automatically
- ~~PATCH /api/repositories/:id/polling~~ - No issue polling service
- ~~POST /api/repositories/:id/poll-issues~~ - No issue polling service
- ~~GET /api/repositories?provider_id=X~~ - Filter by provider_id removed (no provider entity)

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
