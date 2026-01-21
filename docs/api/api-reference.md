# API Reference

**Version:** 0.4.0  
**Last Updated:** 2026-01-21

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
  "version": "0.4.0"
}
```

---

## Settings Module

### RepoProvider Management

#### GET /api/settings/providers

List all Git provider configurations.

**Response:**
```json
[
  {
    "id": 1,
    "name": "My Gitea",
    "type": "gitea",
    "base_url": "https://git.example.com",
    "access_token": "12345678***",
    "locked": false,
    "created_at": "2026-01-21T10:00:00Z"
  }
]
```

#### POST /api/settings/providers

Create a new provider configuration.

**Request:**
```json
{
  "name": "My Gitea",
  "type": "gitea",
  "base_url": "https://git.example.com",
  "access_token": "your-token-here"
}
```

#### GET /api/settings/providers/:id

Get provider details.

#### PUT /api/settings/providers/:id

Update provider configuration.

#### DELETE /api/settings/providers/:id

Delete provider (if not locked).

#### POST /api/settings/providers/:id/validate

Validate provider token.

#### POST /api/settings/providers/:id/sync

Manually trigger repository sync.

---

## Repository Module

#### GET /api/repositories

List repositories with optional filters.

**Query Parameters:**
- `provider_id` (optional) - Filter by provider
- `validation_status` (optional) - Filter by status (valid/invalid/pending)

#### GET /api/repositories/:id

Get repository details.

#### POST /api/repositories/:id/refresh

Refresh repository validation status.

#### POST /api/repositories/:id/initialize

Initialize single repository.

**Request:**
```json
{
  "branch_name": "vibe-dev",
  "create_labels": true
}
```

#### POST /api/repositories/batch-initialize

Batch initialize repositories.

**Request:**
```json
{
  "repository_ids": [1, 2, 3],
  "branch_name": "vibe-dev",
  "create_labels": true
}
```

#### PATCH /api/repositories/:id/polling

Update issue polling configuration.

**Request:**
```json
{
  "polling_enabled": true,
  "polling_interval_seconds": 300,
  "polling_config": {
    "filter_labels": ["vibe/auto"],
    "filter_state": "open"
  }
}
```

#### POST /api/repositories/:id/poll-issues

Manually trigger issue polling.

---

## Webhook Module

#### POST /api/webhooks/:repository_id

Receive webhook events from Git providers.

**Headers:**
- `X-Gitea-Signature` - Webhook signature for verification

---

## Workspace Module

#### POST /api/workspaces

Create a new workspace with optional init script.

**Request:**
```json
{
  "repository_id": 1,
  "init_script": "#!/bin/bash\necho 'Setup'\n",
  "script_timeout_seconds": 600
}
```

#### GET /api/workspaces/:id

Get workspace details including init script status.

#### GET /api/workspaces

List all workspaces.

#### PUT /api/workspaces/:id/status

Update workspace status.

#### DELETE /api/workspaces/:id

Delete workspace.

#### POST /api/workspaces/:id/restart

Manually restart workspace container.

#### GET /api/workspaces/:id/stats

Get real-time container resource statistics.

**Response:**
```json
{
  "workspace_id": 1,
  "container_id": "abc123",
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

---

## Init Script Module

#### PUT /api/workspaces/:id/init-script

Create or update init script for workspace.

**Request:**
```json
{
  "script_content": "#!/bin/bash\necho 'Setup'\n",
  "timeout_seconds": 300,
  "execute_immediately": false
}
```

#### GET /api/workspaces/:id/init-script/logs

Get init script execution logs.

#### GET /api/workspaces/:id/init-script/logs/full

Download full log file.

#### POST /api/workspaces/:id/init-script/execute

Execute init script manually.

---

## Task Module

### CRUD Operations

#### POST /api/tasks

Create a new task.

**Request:**
```json
{
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "issue_body": "Implement JWT-based authentication...",
  "issue_url": "https://git.example.com/owner/repo/issues/42",
  "priority": "High",
  "max_retries": 3
}
```

#### GET /api/tasks

List tasks with optional filters.

**Query Parameters:**
- `workspace_id` (required) - Filter by workspace
- `status` (optional) - Filter by status
- `priority` (optional) - Filter by priority
- `assigned_agent_id` (optional) - Filter by agent

#### GET /api/tasks/:id

Get task details.

#### PATCH /api/tasks/:id

Update task (priority, assigned_agent_id).

#### DELETE /api/tasks/:id

Soft delete task.

### Status Management

#### PATCH /api/tasks/:id/status

Update task status directly.

#### POST /api/tasks/:id/assign

Assign agent to task.

**Request:**
```json
{
  "agent_id": 5
}
```

#### POST /api/tasks/:id/start

Start task execution.

#### POST /api/tasks/:id/complete

Mark task completed with PR information.

**Request:**
```json
{
  "pr_number": 123,
  "pr_url": "https://git.example.com/owner/repo/pulls/123",
  "branch_name": "feature/user-auth"
}
```

#### POST /api/tasks/:id/fail

Mark task failed (with automatic retry logic).

**Request:**
```json
{
  "error_message": "Build failed: missing dependency"
}
```

#### POST /api/tasks/:id/retry

Retry a failed task.

#### POST /api/tasks/:id/cancel

Cancel task execution.

### Task Execution

#### POST /api/tasks/:id/execute

Execute task in workspace container with assigned agent.

**Response (202 Accepted):**
```json
{
  "id": 1,
  "task_status": "Running",
  "started_at": "2026-01-21T12:00:00Z"
}
```

### Monitoring & Analysis

#### GET /api/tasks/:id/logs/stream

WebSocket endpoint for real-time log streaming.

**WebSocket URL:**
```
ws://localhost:3000/api/tasks/:id/logs/stream
```

**Message Format:**
```json
{
  "timestamp": "2026-01-21T12:34:56Z",
  "level": "info",
  "message": "Task execution started",
  "task_id": 123
}
```

#### GET /api/tasks/:id/failure-analysis

Get intelligent failure analysis with recommendations.

**Response:**
```json
{
  "task_id": 123,
  "failure_category": "GitError",
  "root_cause": "Git operation failed",
  "recommendations": [
    "Verify Git credentials and access token",
    "Check repository permissions",
    "Ensure Git is configured in the container"
  ],
  "similar_failures_count": 3,
  "is_recurring": false
}
```

---

## Workspace Image Management

#### GET /api/settings/workspace/image

Query workspace image information.

#### DELETE /api/settings/workspace/image

Delete workspace image (with conflict detection).

#### POST /api/settings/workspace/image/rebuild

Rebuild workspace image from Dockerfile.

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

## WebSocket Endpoints

### Real-time Log Streaming

**Endpoint:** `ws://localhost:3000/api/tasks/:id/logs/stream`

**Connection:**
```javascript
const ws = new WebSocket('ws://localhost:3000/api/tasks/123/logs/stream');

ws.onmessage = (event) => {
  const log = JSON.parse(event.data);
  console.log(log.message);
};
```

---

## Related Documentation

- **[Task API Design](./task-api-design.md)** - Detailed Task API specifications
- **[Issue Polling Feature](./issue-polling-feature.md)** - Issue polling documentation
- **[Container Lifecycle Management](./container-lifecycle-management.md)** - Container management
- **[Init Scripts Guide](./init-scripts-guide.md)** - Init scripts documentation

---

**Last Updated:** 2026-01-21  
**API Version:** 0.4.0
