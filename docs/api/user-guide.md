# User Guide

**Version:** 0.4.0-mvp (Simplified MVP)  
**Last Updated:** 2026-02-06

> **🎯 Simplified MVP**: This version focuses on core Issue-to-PR automation with a streamlined architecture. Many advanced features have been removed to create a solid foundation for future development.

This guide provides comprehensive instructions for using VibeRepo's core features.

## Table of Contents

- [Getting Started](#getting-started)
- [Repository Management](#repository-management)
- [Task Management](#task-management)
- [Webhook Integration](#webhook-integration)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)

---

## Getting Started

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- SQLite 3 or PostgreSQL
- Docker (for workspace features)
- Git provider account (GitHub)

### Installation

1. **Clone the repository:**
```bash
git clone https://github.com/yourusername/vibe-repo.git
cd vibe-repo/backend
```

2. **Create `.env` file:**
```bash
# Database Configuration
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10

# Server Configuration
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

### Overview

In the simplified MVP, Git provider configuration is managed through environment variables instead of a database-backed API. This simplifies deployment and reduces complexity.

### Environment-based Configuration

Configure your Git provider in the `.env` file:

```bash
# GitHub Configuration
GITHUB_TOKEN=ghp_your_token_here
GITHUB_BASE_URL=https://api.github.com
WEBHOOK_SECRET=your_webhook_secret_here
```

**Supported Providers:**
- GitHub (fully supported)
- Gitea (planned)
- GitLab (planned)

### Repository Operations

#### List Repositories

```bash
curl http://localhost:3000/api/repositories
```

#### Get Repository Details

```bash
curl http://localhost:3000/api/repositories/:id
```

#### Initialize Repository

Initialize a repository with the vibe-dev branch and required labels:

```bash
curl -X POST http://localhost:3000/api/repositories/:id/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "branch_name": "vibe-dev",
    "create_labels": true
  }'
```

---

## Task Management

### Overview

Tasks are the core unit of work in VibeRepo. Each task represents an issue that needs to be converted into a pull request.

### Simplified Task State Machine

The MVP uses a simplified state machine:

```
Pending → Running → Completed
                  ↘ Failed
                  ↘ Cancelled
```

**Removed States:**
- ~~Assigned~~ (tasks go directly from Pending to Running)

### Single Agent per Repository

Each repository has exactly one agent configured. Tasks are automatically assigned to the repository's agent when created.

**Key Changes:**
- No manual agent assignment needed
- Agent configuration via environment variables
- One agent per workspace (enforced by unique constraint)

### Creating Tasks

Tasks are typically created automatically via webhooks when issues are opened or labeled. You can also create tasks manually:

```bash
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 42,
    "issue_title": "Add user authentication",
    "issue_body": "Implement JWT-based authentication...",
    "issue_url": "https://github.com/owner/repo/issues/42",
    "priority": "High"
  }'
```

**Note:** The `assigned_agent_id` field has been removed. Tasks are automatically assigned to the workspace's agent.

### Task Lifecycle

**1. Task Created (Pending)**
- Task is created from an issue
- Automatically assigned to workspace's agent

**2. Task Execution (Running)**
```bash
curl -X POST http://localhost:3000/api/tasks/:id/execute
```

**3. Task Completion (Completed)**
- PR is automatically created
- Issue is automatically closed when PR is merged

### Listing Tasks

```bash
# List all tasks for a workspace
curl "http://localhost:3000/api/tasks?workspace_id=1"

# Filter by status
curl "http://localhost:3000/api/tasks?workspace_id=1&status=Pending"

# Filter by priority
curl "http://localhost:3000/api/tasks?workspace_id=1&priority=High"
```

### Task Logs

Task logs are stored inline in the `tasks.last_log` field (no separate task_executions table):

```bash
# Get task details including last log
curl http://localhost:3000/api/tasks/:id
```

**Response:**
```json
{
  "id": 1,
  "workspace_id": 1,
  "issue_number": 42,
  "issue_title": "Add user authentication",
  "task_status": "completed",
  "last_log": "Task completed successfully. PR #123 created.",
  "pr_number": 123,
  "pr_url": "https://github.com/owner/repo/pull/123"
}
```

### Pull Request Creation

VibeRepo automatically creates pull requests when tasks complete successfully:

1. Agent creates a branch and commits changes
2. Agent outputs PR information
3. VibeRepo extracts PR info and creates the PR
4. PR includes "Closes #N" to link the issue

#### Manual PR Creation

If automatic PR creation fails:

```bash
curl -X POST http://localhost:3000/api/tasks/:id/create-pr
```

#### Manual Issue Closure

To manually close an issue after PR is merged:

```bash
curl -X POST http://localhost:3000/api/tasks/:id/close-issue
```

---

## Webhook Integration

### Overview

Webhooks enable real-time issue-to-PR automation. When an issue is opened or labeled, a webhook event triggers task creation and execution.

### Webhook Configuration

Webhooks are configured via environment variables:

```bash
WEBHOOK_SECRET=your_webhook_secret_here
```

### Webhook URL Format

```
https://your-domain.com/api/webhooks/:repository_id
```

### Setting Up GitHub Webhooks

1. Go to your repository settings
2. Navigate to Webhooks → Add webhook
3. Set Payload URL: `https://your-domain.com/api/webhooks/1`
4. Set Content type: `application/json`
5. Set Secret: (same as `WEBHOOK_SECRET` in `.env`)
6. Select events: `Issues`, `Pull requests`
7. Click "Add webhook"

### Webhook Events

**Supported Events:**
- `issues` - Issue opened, edited, labeled
- `pull_request` - PR opened, merged, closed

**Removed Features:**
- ~~Webhook retry mechanism~~ (simplified error handling)
- ~~Webhook status tracking~~ (no webhook_configs table)

---

## Configuration

### Environment Variables

All configuration is done via environment variables in the `.env` file:

```bash
# Database Configuration
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10

# Server Configuration
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

### Agent Configuration

Agents are configured via environment variables:

- `DEFAULT_AGENT_COMMAND` - Command to execute (e.g., `opencode`, `aider`)
- `DEFAULT_AGENT_TIMEOUT` - Timeout in seconds (default: 600)
- `DEFAULT_DOCKER_IMAGE` - Docker image for workspaces (default: `ubuntu:22.04`)

**Example Commands:**
- OpenCode: `opencode --model glm-4-flash`
- Aider: `aider --model gpt-4 --yes`

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
- Increase `DEFAULT_AGENT_TIMEOUT` setting
- Check container resource limits
- Review task complexity

**4. Webhook not receiving events**
- Verify webhook URL is accessible
- Check `WEBHOOK_SECRET` matches
- Review Git provider webhook settings

**5. Missing environment variables**
- Ensure all required variables are set in `.env`
- Check for typos in variable names
- Restart the application after changes

### Getting Help

- **Documentation**: See [docs/README.md](../README.md)
- **API Reference**: See [api-reference.md](./api-reference.md)
- **Migration Guide**: See [MIGRATION.md](../../MIGRATION.md)
- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Ask questions in GitHub Discussions

---

## Removed Features (from v0.3.0)

The following features were removed in the simplified MVP:

**Background Services:**
- ~~Issue Polling Service~~ (use webhooks only)
- ~~Webhook Retry Service~~ (simplified error handling)
- ~~Init Script Service~~ (workspaces use default setup)
- ~~Task Failure Analyzer~~ (basic error messages only)
- ~~Health Check Service~~ (basic health endpoint only)
- ~~Image Management Service~~ (use default Docker images)

**API Endpoints:**
- ~~Provider Management API~~ (configured via environment variables)
- ~~Workspace Management API~~ (workspaces created automatically)
- ~~Agent Management API~~ (agents configured via environment variables)
- ~~Init Script API~~ (no custom init scripts)
- ~~Task Retry Endpoint~~ (no manual retry)
- ~~Task Assignment Endpoint~~ (automatic assignment)

**Features:**
- ~~WebSocket Real-time Logs~~ (logs stored in tasks.last_log)
- ~~Task Execution History~~ (no task_executions table)
- ~~Multiple Agents per Workspace~~ (one agent per workspace)
- ~~Manual Agent Assignment~~ (automatic assignment)
- ~~Task Retry Mechanism~~ (no automatic retry)
- ~~Assigned Task State~~ (simplified state machine)

**Database Tables:**
- ~~repo_providers~~ (configured via environment variables)
- ~~webhook_configs~~ (configured via environment variables)
- ~~init_scripts~~ (no custom init scripts)
- ~~task_executions~~ (logs in tasks.last_log)

---

## Related Documentation

- **[API Reference](./api-reference.md)** - Complete API endpoint reference
- **[Database Schema](../database/schema.md)** - Simplified database schema
- **[Migration Guide](../../MIGRATION.md)** - Migrating from v0.3.0 to v0.4.0-mvp
- **[Development Guide](../development/README.md)** - Development guidelines

---

**Last Updated:** 2026-02-06  
**Version:** 0.4.0-mvp (Simplified MVP)
