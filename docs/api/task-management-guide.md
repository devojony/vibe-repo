# Task Management Guide

**Version:** 0.3.0  
**Last Updated:** 2026-01-24

This guide provides comprehensive documentation for managing tasks in VibeRepo, including task lifecycle, API endpoints, WebSocket streaming, and best practices.

## Table of Contents

- [Overview](#overview)
- [Task Lifecycle](#task-lifecycle)
- [Task API Endpoints](#task-api-endpoints)
- [WebSocket Real-time Logs](#websocket-real-time-logs)
- [Task Execution History](#task-execution-history)
- [Failure Analysis](#failure-analysis)
- [Best Practices](#best-practices)
- [Examples](#examples)

---

## Overview

Tasks in VibeRepo represent automated development work triggered by GitHub/GitLab/Gitea issues. Each task:

- Is created from a repository issue
- Gets assigned to an AI agent
- Executes in an isolated Docker container
- Produces a pull request upon completion
- Maintains complete execution history

**Key Features:**
- Automatic task creation from issues
- Priority-based scheduling
- Retry mechanism with failure analysis
- Real-time log streaming via WebSocket
- Complete execution history tracking
- Automatic PR creation and issue closure

---

## Task Lifecycle

### Task States

Tasks progress through the following states with **state machine validation**:

```
pending → assigned → running → completed
                              ↓
                           failed → (retry) → pending
            ↓                 ↓
        cancelled         cancelled
```

**State Descriptions:**

| State | Description | Next States | API Value |
|-------|-------------|-------------|-----------|
| `Pending` | Task created, waiting for agent assignment | `Assigned`, `Cancelled` | `"pending"` |
| `Assigned` | Agent assigned, ready to execute | `Running`, `Cancelled` | `"assigned"` |
| `Running` | Task execution in progress | `Completed`, `Failed`, `Cancelled` | `"running"` |
| `Completed` | Task successfully completed, PR created | None (terminal) | `"completed"` |
| `Failed` | Task failed after exhausting retries | `Pending` (retry only) | `"failed"` |
| `Cancelled` | Task manually cancelled | None (terminal) | `"cancelled"` |

**State Machine Validation:**
- All state transitions are validated before execution
- Invalid transitions return `400 Bad Request` with detailed error message
- Terminal states (`Completed`, `Cancelled`) cannot transition to any other state
- Retry from `Failed` to `Pending` only allowed if `retry_count < max_retries`

### State Transitions

**1. Task Creation (Pending)**
```
Issue created/labeled → Task created → State: Pending
```

**2. Agent Assignment (Assigned)**
```
Scheduler picks task → Assigns agent → State: Assigned
```

**3. Execution Start (Running)**
```
Agent starts execution → State: Running → Logs streaming
```

**4. Success Path (Completed)**
```
Execution succeeds → PR created → Issue closed → State: Completed
```

**5. Failure Path (Failed)**
```
Execution fails → Retry count < max_retries → State: Running (retry)
                ↓
         Retry count >= max_retries → State: Failed
```

**6. Manual Cancellation (Cancelled)**
```
User cancels task → Stop execution → State: Cancelled
```

### Retry Mechanism

- **Default max retries:** 3
- **Retry delay:** Exponential backoff
- **Failure analysis:** Automatic categorization
- **Smart retry:** Skips retries for certain failure types

---

## Task API Endpoints

### List Tasks

Get all tasks with optional filtering.

**Endpoint:** `GET /api/tasks`

**Query Parameters:**
- `workspace_id` (optional): Filter by workspace
- `status` (optional): Filter by status
- `priority` (optional): Filter by priority

**Example:**
```bash
curl http://localhost:3000/api/tasks?workspace_id=1&status=Running
```

**Response:**
```json
{
  "tasks": [
    {
      "id": 1,
      "workspace_id": 1,
      "issue_number": 42,
      "issue_title": "Add user authentication",
      "task_status": "running",
      "priority": "High",
      "assigned_agent_id": 3,
      "retry_count": 0,
      "max_retries": 3,
      "created_at": "2026-01-24T10:00:00Z",
      "started_at": "2026-01-24T10:05:00Z"
    }
  ]
}
```

### Get Task Details

Get detailed information about a specific task.

**Endpoint:** `GET /api/tasks/:id`

**Example:**
```bash
curl http://localhost:3000/api/tasks/1
```

**Response:**
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "issue_body": "Implement JWT-based authentication...",
  "task_status": "Running",
  "priority": "High",
  "assigned_agent_id": 3,
  "branch_name": "feature/issue-42",
  "retry_count": 0,
  "max_retries": 3,
  "created_at": "2026-01-24T10:00:00Z",
  "updated_at": "2026-01-24T10:05:00Z",
  "started_at": "2026-01-24T10:05:00Z"
}
```

### Create Task

Manually create a task from an issue.

**Endpoint:** `POST /api/tasks`

**Request Body:**
```json
{
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "issue_body": "Implement JWT-based authentication...",
  "priority": "High",
  "assigned_agent_id": 3
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 42,
    "issue_title": "Add user authentication",
    "priority": "High"
  }'
```

**Response:**
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "task_status": "pending",
  "created_at": "2026-01-24T10:00:00Z"
}
```

### Update Task

Update task properties.

**Endpoint:** `PUT /api/tasks/:id`

**Request Body:**
```json
{
  "priority": "High",
  "assigned_agent_id": 3
}
```

**Example:**
```bash
curl -X PUT http://localhost:3000/api/tasks/1 \
  -H "Content-Type: application/json" \
  -d '{"priority": "High"}'
```

### Cancel Task

Cancel a running or pending task.

**Endpoint:** `POST /api/tasks/:id/cancel`

**Example:**
```bash
curl -X POST http://localhost:3000/api/tasks/1/cancel
```

**Response:**
```json
{
  "id": 1,
  "task_status": "Cancelled",
  "updated_at": "2026-01-24T10:30:00Z"
}
```

### Retry Failed Task

Manually retry a failed task.

**Endpoint:** `POST /api/tasks/:id/retry`

**Example:**
```bash
curl -X POST http://localhost:3000/api/tasks/1/retry
```

**Response:**
```json
{
  "id": 1,
  "task_status": "pending",
  "retry_count": 1,
  "updated_at": "2026-01-24T10:35:00Z"
}
```

---

## WebSocket Real-time Logs

### Connection

Connect to WebSocket for real-time task execution logs.

**Endpoint:** `ws://localhost:3000/api/tasks/:id/logs`

**With Authentication:**
```
ws://localhost:3000/api/tasks/:id/logs?token=your-auth-token
```

### JavaScript Example

```javascript
// Connect to WebSocket
const taskId = 1;
const token = 'your-auth-token'; // Optional, if WEBSOCKET_AUTH_TOKEN is set
const ws = new WebSocket(`ws://localhost:3000/api/tasks/${taskId}/logs?token=${token}`);

// Handle connection open
ws.onopen = () => {
  console.log('Connected to task logs');
};

// Handle incoming messages
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  
  switch(data.type) {
    case 'log':
      console.log(`[${data.timestamp}] ${data.message}`);
      break;
    case 'status':
      console.log(`Task status: ${data.status}`);
      break;
    case 'error':
      console.error(`Error: ${data.message}`);
      break;
    case 'complete':
      console.log('Task completed');
      ws.close();
      break;
  }
};

// Handle errors
ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

// Handle connection close
ws.onclose = () => {
  console.log('Disconnected from task logs');
};
```

### Python Example

```python
import asyncio
import websockets
import json

async def stream_task_logs(task_id, token=None):
    url = f"ws://localhost:3000/api/tasks/{task_id}/logs"
    if token:
        url += f"?token={token}"
    
    async with websockets.connect(url) as websocket:
        print(f"Connected to task {task_id} logs")
        
        async for message in websocket:
            data = json.loads(message)
            
            if data['type'] == 'log':
                print(f"[{data['timestamp']}] {data['message']}")
            elif data['type'] == 'status':
                print(f"Task status: {data['status']}")
            elif data['type'] == 'complete':
                print("Task completed")
                break
            elif data['type'] == 'error':
                print(f"Error: {data['message']}")

# Run
asyncio.run(stream_task_logs(1, 'your-auth-token'))
```

### Message Types

**Log Message:**
```json
{
  "type": "log",
  "timestamp": "2026-01-24T10:05:30Z",
  "level": "info",
  "message": "Running git clone..."
}
```

**Status Update:**
```json
{
  "type": "status",
  "status": "running",
  "timestamp": "2026-01-24T10:05:00Z"
}
```

**Error Message:**
```json
{
  "type": "error",
  "message": "Failed to connect to repository",
  "timestamp": "2026-01-24T10:06:00Z"
}
```

**Completion Message:**
```json
{
  "type": "complete",
  "status": "completed",
  "pr_url": "https://github.com/owner/repo/pull/123",
  "timestamp": "2026-01-24T10:30:00Z"
}
```

### Authentication

If `WEBSOCKET_AUTH_TOKEN` is set, include it in the query string:

```javascript
const ws = new WebSocket(
  `ws://localhost:3000/api/tasks/${taskId}/logs?token=${authToken}`
);
```

**Security Notes:**
- Always use WSS (WebSocket Secure) in production
- Set `WEBSOCKET_AUTH_TOKEN` in production
- Rotate tokens periodically
- Use HTTPS/WSS to prevent token interception

---

## Task Execution History

### Get Execution History

Get all execution attempts for a task.

**Endpoint:** `GET /api/tasks/:id/executions`

**Example:**
```bash
curl http://localhost:3000/api/tasks/1/executions
```

**Response:**
```json
{
  "executions": [
    {
      "id": 1,
      "task_id": 1,
      "agent_id": 3,
      "status": "completed",
      "exit_code": 0,
      "pr_number": 123,
      "pr_url": "https://github.com/owner/repo/pull/123",
      "branch_name": "feature/issue-42",
      "duration_ms": 45000,
      "started_at": "2026-01-24T10:05:00Z",
      "completed_at": "2026-01-24T10:05:45Z"
    }
  ]
}
```

### Get Execution Details

Get detailed information about a specific execution.

**Endpoint:** `GET /api/tasks/:task_id/executions/:execution_id`

**Example:**
```bash
curl http://localhost:3000/api/tasks/1/executions/1
```

**Response:**
```json
{
  "id": 1,
  "task_id": 1,
  "agent_id": 3,
  "status": "completed",
  "command": "opencode --model claude-3.5-sonnet",
  "exit_code": 0,
  "stdout_summary": "Task completed successfully...",
  "pr_number": 123,
  "pr_url": "https://github.com/owner/repo/pull/123",
  "branch_name": "feature/issue-42",
  "duration_ms": 45000,
  "started_at": "2026-01-24T10:05:00Z",
  "completed_at": "2026-01-24T10:05:45Z"
}
```

### Get Execution Logs

Get full execution logs (stdout/stderr).

**Endpoint:** `GET /api/tasks/:task_id/executions/:execution_id/logs`

**Query Parameters:**
- `type`: `stdout` or `stderr` (default: `stdout`)

**Example:**
```bash
curl http://localhost:3000/api/tasks/1/executions/1/logs?type=stdout
```

---

## Failure Analysis

### Get Failure Analysis

Get automatic failure analysis for a failed task.

**Endpoint:** `GET /api/tasks/:id/failure-analysis`

**Example:**
```bash
curl http://localhost:3000/api/tasks/1/failure-analysis
```

**Response:**
```json
{
  "task_id": 1,
  "failure_category": "dependency_error",
  "root_cause": "Missing npm package 'express'",
  "recommendations": [
    "Add 'express' to package.json dependencies",
    "Run 'npm install express --save'",
    "Check package.json for typos"
  ],
  "similar_failures": [
    {
      "task_id": 5,
      "similarity": 0.85,
      "resolution": "Added missing dependency to package.json"
    }
  ],
  "should_retry": false,
  "analyzed_at": "2026-01-24T10:30:00Z"
}
```

### Failure Categories

| Category | Description | Auto-Retry |
|----------|-------------|------------|
| `dependency_error` | Missing or incompatible dependencies | No |
| `syntax_error` | Code syntax errors | No |
| `test_failure` | Test suite failures | No |
| `timeout` | Execution timeout | Yes |
| `network_error` | Network connectivity issues | Yes |
| `git_error` | Git operation failures | Yes |
| `container_error` | Docker container issues | Yes |
| `permission_error` | File/directory permission issues | No |
| `unknown` | Unclassified failure | Yes |

### Recommendations

The failure analyzer provides:
- **Root cause identification**
- **Actionable recommendations**
- **Similar failure detection**
- **Smart retry decisions**

---

## Best Practices

### Task Priority

Set appropriate priorities based on urgency:

```bash
# High priority for critical bugs
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 42,
    "priority": "High"
  }'

# Medium priority for features (default)
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 43,
    "priority": "Medium"
  }'

# Low priority for refactoring
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 44,
    "priority": "Low"
  }'
```

### Monitoring Tasks

Monitor task progress with WebSocket:

```javascript
function monitorTask(taskId) {
  const ws = new WebSocket(`ws://localhost:3000/api/tasks/${taskId}/logs`);
  
  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    
    // Update UI with progress
    if (data.type === 'log') {
      appendLog(data.message);
    } else if (data.type === 'status') {
      updateStatus(data.status);
    } else if (data.type === 'complete') {
      showSuccess(data.pr_url);
      ws.close();
    }
  };
}
```

### Error Handling

Handle task failures gracefully:

```javascript
async function executeTask(taskId) {
  try {
    // Start monitoring
    const ws = new WebSocket(`ws://localhost:3000/api/tasks/${taskId}/logs`);
    
    // Wait for completion
    await new Promise((resolve, reject) => {
      ws.onmessage = (event) => {
        const data = JSON.parse(event.data);
        
        if (data.type === 'complete') {
          resolve(data);
        } else if (data.type === 'error') {
          reject(new Error(data.message));
        }
      };
    });
    
  } catch (error) {
    // Get failure analysis
    const analysis = await fetch(
      `http://localhost:3000/api/tasks/${taskId}/failure-analysis`
    ).then(r => r.json());
    
    console.error('Task failed:', analysis.root_cause);
    console.log('Recommendations:', analysis.recommendations);
    
    // Retry if recommended
    if (analysis.should_retry) {
      await fetch(`http://localhost:3000/api/tasks/${taskId}/retry`, {
        method: 'POST'
      });
    }
  }
}
```

### Retry Strategy

Configure retry behavior:

```json
{
  "max_retries": 3,
  "retry_delay": "exponential",
  "skip_retry_on": [
    "dependency_error",
    "syntax_error",
    "permission_error"
  ]
}
```

---

## Examples

### Example 1: Create and Monitor Task

```bash
# 1. Create task
TASK_ID=$(curl -s -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 42,
    "issue_title": "Add user authentication",
    "priority": "High"
  }' | jq -r '.id')

echo "Created task: $TASK_ID"

# 2. Monitor with WebSocket (in another terminal)
# See JavaScript/Python examples above

# 3. Check status
curl http://localhost:3000/api/tasks/$TASK_ID

# 4. Get execution history
curl http://localhost:3000/api/tasks/$TASK_ID/executions
```

### Example 2: Handle Failed Task

```bash
# 1. Get task details
curl http://localhost:3000/api/tasks/1

# 2. Get failure analysis
curl http://localhost:3000/api/tasks/1/failure-analysis

# 3. Review recommendations
# ... fix issues based on recommendations ...

# 4. Retry task
curl -X POST http://localhost:3000/api/tasks/1/retry

# 5. Monitor retry
# ... use WebSocket to monitor ...
```

### Example 3: Bulk Task Management

```bash
# Get all pending tasks
curl 'http://localhost:3000/api/tasks?status=Pending'

# Get all high priority tasks
curl 'http://localhost:3000/api/tasks?priority=High'

# Get tasks for specific workspace
curl 'http://localhost:3000/api/tasks?workspace_id=1'

# Cancel multiple tasks
for task_id in 1 2 3; do
  curl -X POST http://localhost:3000/api/tasks/$task_id/cancel
done
```

### Example 4: Task Dashboard

```javascript
// Fetch task statistics
async function getTaskStats() {
  const tasks = await fetch('http://localhost:3000/api/tasks')
    .then(r => r.json());
  
  const stats = {
    total: tasks.length,
    pending: tasks.filter(t => t.task_status === 'pending').length,
    running: tasks.filter(t => t.task_status === 'running').length,
    completed: tasks.filter(t => t.task_status === 'completed').length,
    failed: tasks.filter(t => t.task_status === 'failed').length
  };
  
  return stats;
}

// Display dashboard
getTaskStats().then(stats => {
  console.log('Task Statistics:');
  console.log(`Total: ${stats.total}`);
  console.log(`Pending: ${stats.pending}`);
  console.log(`Running: ${stats.running}`);
  console.log(`Completed: ${stats.completed}`);
  console.log(`Failed: ${stats.failed}`);
});
```

---

## Related Documentation

- [API Reference](./api-reference.md) - Complete API endpoint documentation
- [User Guide](./user-guide.md) - General usage guide
- [Environment Variables](./environment-variables.md) - Configuration reference
- [WebSocket Testing](../testing/websocket-testing.md) - WebSocket testing guide

---

**Last Updated:** 2026-01-24  
**Version:** 0.3.0
