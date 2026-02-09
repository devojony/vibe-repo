# API Reference

**Version:** 0.4.0-mvp (Simplified MVP)  
**Last Updated:** 2026-02-09

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
  "event_type": "issue_comment",
  "action": "created"
}
```

**Supported Event Types:**
- `issue_comment` - Comment on issue (checks for bot mention)
- `pull_request_comment` - Comment on PR (checks for bot mention)
- `pull_request` - PR events (merge triggers issue closure)

**Note:** The system uses a mention-based workflow. Only comments that mention the bot (e.g., `@vibe-repo-bot`) will trigger task creation.

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

### GET /api/tasks/:id/status

Get task status with progress information.

**Response:** `200 OK`
```json
{
  "task_id": 1,
  "status": "running",
  "progress": 66.67,
  "started_at": "2026-02-08T10:00:00Z",
  "completed_at": null,
  "created_at": "2026-02-08T09:55:00Z"
}
```

**Progress Calculation:**
- Progress is calculated from plan completion: `(completed_steps / total_steps) * 100`
- Returns `null` if no plan is available

### GET /api/tasks/:id/plans

Retrieve the current execution plan for a task.

**Response:** `200 OK`
```json
{
  "plans": [
    {
      "type": "plan",
      "steps": [
        {
          "description": "Analyze issue requirements",
          "status": "completed",
          "index": 0
        },
        {
          "description": "Implement authentication logic",
          "status": "in_progress",
          "index": 1
        },
        {
          "description": "Write tests",
          "status": "pending",
          "index": 2
        }
      ],
      "current_step": 1,
      "status": "active",
      "timestamp": "2026-02-08T10:00:00Z"
    }
  ]
}
```

**Plan Status Values:**
- `creating` - Plan is being created
- `active` - Plan is being executed
- `completed` - Plan is finished
- `modified` - Plan was updated

**Step Status Values:**
- `pending` - Not started yet
- `in_progress` - Currently executing
- `completed` - Finished successfully
- `skipped` - Skipped this step

**Usage Examples:**

Get current plan:
```bash
curl http://localhost:3000/api/tasks/123/plans
```

Extract steps only:
```bash
curl http://localhost:3000/api/tasks/123/plans | jq '.plans[0].steps'
```

Check current step:
```bash
curl http://localhost:3000/api/tasks/123/plans | jq '.plans[0].current_step'
```

**Error Responses:**
- `404 Not Found` - Task not found
- `500 Internal Server Error` - Failed to retrieve plans

### GET /api/tasks/:id/events

Retrieve events for a task with optional filtering.

**Query Parameters:**
- `event_type` (optional) - Filter by event type: "plan", "tool_call", "message", "completed"
- `since` (optional) - Filter events since timestamp (ISO 8601 format)
- `limit` (optional) - Limit number of events returned

**Examples:**

Get all events:
```bash
curl http://localhost:3000/api/tasks/123/events
```

Get only tool calls:
```bash
curl http://localhost:3000/api/tasks/123/events?event_type=tool_call
```

Get recent events (last 10):
```bash
curl http://localhost:3000/api/tasks/123/events?limit=10
```

Get events since timestamp:
```bash
curl "http://localhost:3000/api/tasks/123/events?since=2026-02-08T10:00:00Z"
```

**Response:** `200 OK`
```json
{
  "events": [
    {
      "type": "plan",
      "steps": [
        {
          "description": "Analyze issue",
          "status": "completed",
          "index": 0
        }
      ],
      "current_step": 0,
      "status": "active",
      "timestamp": "2026-02-08T10:00:00Z"
    },
    {
      "type": "tool_call",
      "tool_name": "read_file",
      "args": {
        "path": "src/main.rs"
      },
      "result": "success",
      "timestamp": "2026-02-08T10:00:30Z"
    },
    {
      "type": "message",
      "content": "Analyzing code structure",
      "level": "info",
      "timestamp": "2026-02-08T10:00:45Z"
    },
    {
      "type": "completed",
      "success": true,
      "summary": "Task completed successfully",
      "timestamp": "2026-02-08T10:05:00Z"
    }
  ]
}
```

**Event Types:**

1. **Plan Event** - Agent's execution plan
   ```json
   {
     "type": "plan",
     "steps": [...],
     "current_step": 1,
     "status": "active",
     "timestamp": "2026-02-08T10:00:00Z"
   }
   ```

2. **Tool Call Event** - Agent tool execution
   ```json
   {
     "type": "tool_call",
     "tool_name": "write_file",
     "args": {"path": "src/auth.rs", "content": "..."},
     "result": "success",
     "timestamp": "2026-02-08T10:01:00Z"
   }
   ```

3. **Message Event** - Agent message
   ```json
   {
     "type": "message",
     "content": "Implemented authentication",
     "level": "info",
     "timestamp": "2026-02-08T10:02:00Z"
   }
   ```

4. **Completed Event** - Task completion
   ```json
   {
     "type": "completed",
     "success": true,
     "summary": "Task completed successfully",
     "timestamp": "2026-02-08T10:05:00Z"
   }
   ```

**Usage Examples:**

Monitor recent activity:
```bash
# Poll every 2 seconds
while true; do
  curl -s http://localhost:3000/api/tasks/123/events?limit=5 | jq '.events[-1]'
  sleep 2
done
```

Filter by event type:
```bash
# Only tool calls
curl http://localhost:3000/api/tasks/123/events?event_type=tool_call | jq '.events'

# Only messages
curl http://localhost:3000/api/tasks/123/events?event_type=message | jq '.events'
```

Check for errors:
```bash
curl http://localhost:3000/api/tasks/123/events | \
  jq '.events[] | select(.type == "message" and .level == "error")'
```

**Error Responses:**
- `404 Not Found` - Task not found
- `400 Bad Request` - Invalid query parameters
- `500 Internal Server Error` - Failed to retrieve events

### GET /api/tasks/:id/progress

Get task progress percentage based on plan completion.

**Response:** `200 OK`
```json
{
  "task_id": 1,
  "progress": 66.67,
  "total_steps": 3,
  "completed_steps": 2,
  "current_step": {
    "description": "Write tests",
    "status": "in_progress",
    "index": 2
  }
}
```

**Progress Calculation:**
```
progress = (completed_steps / total_steps) * 100
```

Returns `0.0` if no plan is available.

**Usage Examples:**

Get progress percentage:
```bash
curl http://localhost:3000/api/tasks/123/progress | jq '.progress'
```

Watch progress (updates every 2 seconds):
```bash
watch -n 2 'curl -s http://localhost:3000/api/tasks/123/progress | jq'
```

Get current step:
```bash
curl http://localhost:3000/api/tasks/123/progress | jq '.current_step'
```

Check if task is complete:
```bash
progress=$(curl -s http://localhost:3000/api/tasks/123/progress | jq '.progress')
if [ "$progress" == "100" ]; then
  echo "Task complete!"
fi
```

**Error Responses:**
- `404 Not Found` - Task not found
- `500 Internal Server Error` - Failed to calculate progress

### GET /api/tasks/:id/logs

Get task execution logs.

**Response:** `200 OK`
```json
{
  "task_id": 1,
  "logs": "Task started...\nExecuting step 1...\nCompleted successfully."
}
```

**Note:** This endpoint returns the `last_log` field from the task. For real-time progress tracking, use the `/events` endpoint instead.

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

**Last Updated:** 2026-02-09  
**API Version:** 0.4.0-mvp (Simplified MVP)
