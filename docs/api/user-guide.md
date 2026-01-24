# User Guide

**Version:** 0.3.0  
**Last Updated:** 2026-01-24

This guide provides comprehensive instructions for using VibeRepo features.

## Table of Contents

- [Getting Started](#getting-started)
- [Repository Management](#repository-management)
- [Workspace Management](#workspace-management)
- [Agent Management](#agent-management)
- [Task Management](#task-management)
- [Issue Polling](#issue-polling)
- [Container Management](#container-management)
- [Monitoring & Debugging](#monitoring--debugging)

---

## Getting Started

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- SQLite 3 or PostgreSQL
- Docker (for workspace features)
- Git provider account (Gitea/GitHub/GitLab)

### Installation

1. **Clone the repository:**
```bash
git clone https://github.com/yourusername/vibe-repo.git
cd vibe-repo/backend
```

2. **Create `.env` file:**
```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
```

3. **Build and run:**
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

---

## Repository Management

### Adding a Git Provider

1. **Create provider configuration:**
```bash
curl -X POST http://localhost:3000/api/settings/providers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Gitea",
    "type": "gitea",
    "base_url": "https://git.example.com",
    "access_token": "your-token-here"
  }'
```

2. **Validate provider token:**
```bash
curl -X POST http://localhost:3000/api/settings/providers/1/validate
```

3. **Sync repositories:**
```bash
curl -X POST http://localhost:3000/api/settings/providers/1/sync
```

### Initializing Repositories

**Single repository:**
```bash
curl -X POST http://localhost:3000/api/repositories/1/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "branch_name": "vibe-dev",
    "create_labels": true
  }'
```

**Batch initialization:**
```bash
curl -X POST http://localhost:3000/api/repositories/batch-initialize \
  -H "Content-Type: application/json" \
  -d '{
    "repository_ids": [1, 2, 3],
    "branch_name": "vibe-dev",
    "create_labels": true
  }'
```

---

## Workspace Management

### Creating a Workspace

**Basic workspace:**
```bash
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 1
  }'
```

**Workspace with init script:**
```bash
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 1,
    "init_script": "#!/bin/bash\necho \"Setting up workspace...\"\napt-get update\napt-get install -y git curl vim\necho \"Setup complete!\"",
    "script_timeout_seconds": 600
  }'
```

### Managing Init Scripts

**Update init script:**
```bash
curl -X PUT http://localhost:3000/api/workspaces/1/init-script \
  -H "Content-Type: application/json" \
  -d '{
    "script_content": "#!/bin/bash\necho \"Updated script\"\ndate\nuname -a",
    "timeout_seconds": 300,
    "execute_immediately": false
  }'
```

**Execute init script manually:**
```bash
curl -X POST http://localhost:3000/api/workspaces/1/init-script/execute \
  -H "Content-Type: application/json" \
  -d '{
    "force": false
  }'
```

**Check script logs:**
```bash
curl http://localhost:3000/api/workspaces/1/init-script/logs
```

---

## Agent Management

Agents are AI-powered automation tools that execute tasks in workspaces. Each workspace can have multiple agents configured with different tools and models.

### Creating an Agent

**Basic agent creation:**
```bash
curl -X POST http://localhost:3000/api/agents \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "name": "OpenCode Agent",
    "tool_type": "OpenCode",
    "command": "opencode --model glm-4-flash",
    "timeout": 600
  }'
```

**Agent with environment variables:**
```bash
curl -X POST http://localhost:3000/api/agents \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "name": "Aider GPT-4",
    "tool_type": "Aider",
    "command": "aider --model gpt-4 --yes",
    "timeout": 1800,
    "env_vars": {
      "OPENAI_API_KEY": "sk-...",
      "OPENAI_BASE_URL": "https://api.openai.com/v1"
    }
  }'
```

### Agent Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `workspace_id` | integer | Yes | ID of the workspace |
| `name` | string | Yes | Human-readable agent name |
| `tool_type` | string | Yes | Tool type (OpenCode, Aider, Custom) |
| `command` | string | Yes | Full command to execute |
| `timeout` | integer | No | Timeout in seconds (default: 1800) |
| `env_vars` | object | No | Environment variables |

### Supported Tool Types

#### OpenCode

OpenCode is an AI coding assistant that can work with various models.

**Example configuration:**
```json
{
  "workspace_id": 1,
  "name": "OpenCode GLM-4",
  "tool_type": "OpenCode",
  "command": "opencode --model glm-4-flash",
  "timeout": 600,
  "env_vars": {
    "OPENAI_API_KEY": "your-api-key"
  }
}
```

**Common OpenCode commands:**
- `opencode --model glm-4-flash` - Use GLM-4 Flash model
- `opencode --model gpt-4` - Use GPT-4 model
- `opencode --model claude-3-opus` - Use Claude 3 Opus

#### Aider

Aider is an AI pair programming tool in your terminal.

**Example configuration:**
```json
{
  "workspace_id": 1,
  "name": "Aider GPT-4",
  "tool_type": "Aider",
  "command": "aider --model gpt-4 --yes --no-git",
  "timeout": 1800,
  "env_vars": {
    "OPENAI_API_KEY": "your-api-key"
  }
}
```

**Common Aider commands:**
- `aider --model gpt-4 --yes` - Use GPT-4 with auto-confirm
- `aider --model claude-3-opus --yes` - Use Claude 3 Opus
- `aider --model gpt-3.5-turbo --yes --no-git` - Use GPT-3.5 without git

#### Custom Tools

You can configure any custom automation tool.

**Example configuration:**
```json
{
  "workspace_id": 1,
  "name": "Custom Script",
  "tool_type": "Custom",
  "command": "/usr/local/bin/my-automation-tool --config /etc/config.json",
  "timeout": 3600,
  "env_vars": {
    "CUSTOM_VAR": "value"
  }
}
```

### Listing Agents

**List all agents for a workspace:**
```bash
curl http://localhost:3000/api/workspaces/1/agents
```

**Response:**
```json
[
  {
    "id": 1,
    "workspace_id": 1,
    "name": "OpenCode Agent",
    "tool_type": "OpenCode",
    "enabled": true,
    "command": "opencode --model glm-4-flash",
    "env_vars": {},
    "timeout": 600,
    "created_at": "2026-01-24T10:30:00Z",
    "updated_at": "2026-01-24T10:30:00Z"
  }
]
```

### Getting Agent Details

**Get specific agent:**
```bash
curl http://localhost:3000/api/agents/1
```

### Enabling/Disabling Agents

**Disable an agent:**
```bash
curl -X PATCH http://localhost:3000/api/agents/1/enabled \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": false
  }'
```

**Enable an agent:**
```bash
curl -X PATCH http://localhost:3000/api/agents/1/enabled \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true
  }'
```

**Note:** Disabled agents cannot be assigned to new tasks, but existing task assignments are not affected.

### Deleting Agents

**Delete an agent:**
```bash
curl -X DELETE http://localhost:3000/api/agents/1
```

**Important:** Deleting an agent will not affect existing tasks that were assigned to it. Those tasks will continue to reference the deleted agent.

### Agent Best Practices

1. **Use Descriptive Names**: Name agents clearly to indicate their purpose
   - Good: "OpenCode GLM-4 for Python"
   - Bad: "Agent 1"

2. **Set Appropriate Timeouts**: Consider task complexity
   - Simple tasks: 600 seconds (10 minutes)
   - Complex tasks: 1800 seconds (30 minutes)
   - Very complex: 3600 seconds (1 hour)

3. **Secure Environment Variables**: Store sensitive data in `env_vars`
   - API keys
   - Access tokens
   - Configuration URLs

4. **Test Agent Configuration**: Create a test task to verify agent works correctly

5. **Use Multiple Agents**: Configure different agents for different types of tasks
   - Fast model for simple tasks
   - Powerful model for complex tasks
   - Specialized tools for specific domains

### Common Issues

#### Agent Command Not Found

**Problem:** Agent fails with "command not found" error

**Solution:** Use absolute path in command:
```json
{
  "command": "/usr/local/bin/opencode --model glm-4-flash"
}
```

#### Timeout Too Short

**Problem:** Tasks fail due to timeout

**Solution:** Increase timeout value:
```json
{
  "timeout": 3600
}
```

#### Missing Environment Variables

**Problem:** Agent fails due to missing API keys

**Solution:** Add required environment variables:
```json
{
  "env_vars": {
    "OPENAI_API_KEY": "sk-...",
    "OPENAI_BASE_URL": "https://api.openai.com/v1"
  }
}
```

---

## Task Management

### Creating Tasks

**Create task from issue:**
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

### Task Lifecycle

**1. Assign agent:**
```bash
curl -X POST http://localhost:3000/api/tasks/1/assign \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": 5
  }'
```

**2. Start execution:**
```bash
curl -X POST http://localhost:3000/api/tasks/1/start
```

**3. Complete task:**
```bash
curl -X POST http://localhost:3000/api/tasks/1/complete \
  -H "Content-Type: application/json" \
  -d '{
    "pr_number": 123,
    "pr_url": "https://git.example.com/owner/repo/pulls/123",
    "branch_name": "feature/user-auth"
  }'
```

### Pull Request Creation

VibeRepo automatically creates pull requests when tasks are completed successfully. The system extracts PR information from the agent's output and creates the PR via the Git provider API.

#### Automatic PR Creation

When a task completes successfully:
1. The agent creates a branch and commits changes
2. The agent outputs PR information in the format: `PR_NUMBER=123 PR_URL=https://...`
3. VibeRepo extracts this information and updates the task
4. The PR is automatically created with:
   - Title from the issue
   - Body including "Closes #N" to link the issue
   - Labels from the original issue
   - Assignment to the repository owner

#### Manual PR Creation

If automatic PR creation fails or you need to create a PR manually:

```bash
curl -X POST http://localhost:3000/api/tasks/1/create-pr
```

**Response:**
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "status": "Completed",
  "pr_number": 123,
  "pr_url": "https://git.example.com/owner/repo/pulls/123",
  "branch_name": "feature/user-auth"
}
```

**Requirements:**
- Task must be in "Completed" status
- Task must have branch_name set
- Repository must be accessible with current credentials

#### Manual Issue Closure

To manually close an issue after PR is merged:

```bash
curl -X POST http://localhost:3000/api/tasks/1/close-issue
```

**Response:**
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "status": "Completed",
  "pr_number": 123,
  "pr_url": "https://git.example.com/owner/repo/pulls/123"
}
```

**Requirements:**
- Task must have issue_number set
- Issue must exist in the repository
- Repository must be accessible with current credentials

#### PR Body Format

Pull requests created by VibeRepo include:
- Issue title as PR title
- Issue body as PR description
- "Closes #N" keyword to automatically close the issue when PR is merged
- Link to the original issue
- Task execution details (optional)

**Example PR Body:**
```markdown
Implement JWT-based authentication for user login and registration.

This PR addresses the requirements outlined in the issue.

Closes #42

---
Generated by VibeRepo
Task ID: 1
Branch: feature/user-auth
```

### Filtering Tasks

**List pending tasks:**
```bash
curl "http://localhost:3000/api/tasks?workspace_id=1&status=Pending"
```

**List high priority tasks:**
```bash
curl "http://localhost:3000/api/tasks?workspace_id=1&priority=High"
```

**List tasks by agent:**
```bash
curl "http://localhost:3000/api/tasks?workspace_id=1&assigned_agent_id=5"
```

### Task Execution

**Execute task:**
```bash
curl -X POST http://localhost:3000/api/tasks/1/execute
```

The task will be executed asynchronously in the workspace container.

---

## Issue Polling

### Enabling Issue Polling

**Configure polling for repository:**
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

### Manual Polling

**Trigger polling manually:**
```bash
curl -X POST http://localhost:3000/api/repositories/1/poll-issues
```

**Response:**
```json
{
  "repository_id": 1,
  "issues_found": 5,
  "tasks_created": 3,
  "tasks_updated": 2,
  "polling_duration_ms": 234
}
```

### Polling Configuration

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `polling_enabled` | boolean | Enable/disable polling | `false` |
| `polling_interval_seconds` | integer | Polling interval (60-86400) | `300` |
| `filter_labels` | array | Filter by labels (OR logic) | `[]` |
| `filter_mentions` | array | Filter by @mentions | `[]` |
| `filter_state` | string | Filter by state (open/closed/all) | `open` |
| `max_issue_age_days` | integer | Max issue age in days | `30` |

---

## Container Management

### Monitoring Container Health

**Get container statistics:**
```bash
curl http://localhost:3000/api/workspaces/1/stats
```

**Response:**
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

### Restarting Containers

**Manual restart:**
```bash
curl -X POST http://localhost:3000/api/workspaces/1/restart
```

**Automatic restart:**
- Containers are automatically restarted on failure
- Default: 3 restart attempts
- Configurable via `max_restart_attempts`

### Managing Workspace Images

**Query image information:**
```bash
curl http://localhost:3000/api/settings/workspace/image
```

**Rebuild image:**
```bash
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild
```

---

## Monitoring & Debugging

### Real-time Log Streaming

**JavaScript client:**
```javascript
const taskId = 123;
const ws = new WebSocket(`ws://localhost:3000/api/tasks/${taskId}/logs/stream`);

ws.onopen = () => {
  console.log('Connected to task logs');
};

ws.onmessage = (event) => {
  const log = JSON.parse(event.data);
  console.log(`[${log.timestamp}] ${log.level}: ${log.message}`);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected from task logs');
};
```

**Python client:**
```python
import websocket
import json

def on_message(ws, message):
    log = json.loads(message)
    print(f"[{log['timestamp']}] {log['level']}: {log['message']}")

def on_error(ws, error):
    print(f"Error: {error}")

def on_close(ws, close_status_code, close_msg):
    print("Connection closed")

def on_open(ws):
    print("Connected to task logs")

task_id = 123
ws = websocket.WebSocketApp(
    f"ws://localhost:3000/api/tasks/{task_id}/logs/stream",
    on_open=on_open,
    on_message=on_message,
    on_error=on_error,
    on_close=on_close
)

ws.run_forever()
```

### Failure Analysis

**Get failure analysis:**
```bash
curl http://localhost:3000/api/tasks/123/failure-analysis
```

**Response:**
```json
{
  "task_id": 123,
  "failure_category": "GitError",
  "root_cause": "Git operation failed",
  "recommendations": [
    "Verify Git credentials and access token",
    "Check repository permissions",
    "Ensure Git is configured in the container",
    "Verify branch names and remote URLs"
  ],
  "similar_failures_count": 3,
  "is_recurring": false
}
```

### Failure Categories

1. **ContainerError** - Docker/container issues
2. **AgentError** - Agent command or configuration problems
3. **GitError** - Git operations failures
4. **BuildError** - Build or compilation errors
5. **TestError** - Test failures
6. **Timeout** - Execution timeout exceeded
7. **PermissionError** - Access or permission denied
8. **NetworkError** - Network connectivity issues
9. **Unknown** - Unclassified errors

---

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```bash
# Database Configuration
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10

# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Logging
RUST_LOG=debug
LOG_FORMAT=json  # Optional: use JSON logs in production

# Issue Polling
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=300
ISSUE_POLLING_BATCH_SIZE=10
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=30

# Task Scheduler
TASK_SCHEDULER_INTERVAL_SECONDS=30

# Workspace Configuration
WORKSPACE_MAX_CONCURRENT_TASKS=3
WORKSPACE_MAX_RESTART_ATTEMPTS=3
```

### Database Options

**SQLite (Development):**
```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
```

**PostgreSQL (Production):**
```bash
DATABASE_URL=postgresql://user:password@localhost:5432/vibe_repo
DATABASE_MAX_CONNECTIONS=20
```

---

## Troubleshooting

### Common Issues

**1. Database connection failed**
- Check `DATABASE_URL` in `.env`
- Ensure database file directory exists
- Verify database permissions

**2. Container startup failed**
- Check Docker daemon is running
- Verify Docker image exists
- Check container logs

**3. Task execution timeout**
- Increase agent timeout setting
- Check container resource limits
- Review task complexity

**4. Webhook not receiving events**
- Verify webhook URL is accessible
- Check webhook secret matches
- Review Git provider webhook settings

### Getting Help

- **Documentation**: See [docs/README.md](../README.md)
- **API Reference**: See [api-reference.md](./api-reference.md)
- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Ask questions in GitHub Discussions

---

## Related Documentation

- **[API Reference](./api-reference.md)** - Complete API endpoint reference
- **[Task API Design](./task-api-design.md)** - Detailed Task API specifications
- **[Issue Polling Feature](./issue-polling-feature.md)** - Issue polling documentation
- **[Container Lifecycle Management](./container-lifecycle-management.md)** - Container management
- **[Init Scripts Guide](./init-scripts-guide.md)** - Init scripts documentation

---

**Last Updated:** 2026-01-24  
**Version:** 0.3.0
