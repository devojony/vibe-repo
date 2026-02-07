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

In v0.4.0-mvp, the architecture has changed from Provider-Centric to Repository-Centric. Instead of configuring a provider and auto-syncing all repositories, you now manually add only the repositories you want, providing provider configuration for each one.

**Key Changes:**
- No separate provider entity - configuration stored per repository
- No auto-discovery - explicitly add only desired repositories
- Self-contained repositories - each has its own provider credentials
- Single-step addition - repository, workspace, and webhook created atomically

### Adding a Repository

To add a repository to VibeRepo, use the `POST /api/repositories` endpoint with complete provider configuration:

```bash
curl -X POST http://localhost:3000/api/repositories \
  -H "Content-Type: application/json" \
  -d '{
    "provider_type": "github",
    "provider_base_url": "https://api.github.com",
    "access_token": "ghp_xxxxxxxxxxxx",
    "full_name": "owner/my-repo",
    "branch_name": "vibe-dev"
  }'
```

**Request Fields:**
- `provider_type` (required) - Git provider: "github", "gitea", or "gitlab"
- `provider_base_url` (required) - Provider API base URL
  - GitHub: `https://api.github.com`
  - Gitea: `https://gitea.example.com/api/v1`
  - GitLab: `https://gitlab.com/api/v4`
- `access_token` (required) - Personal access token with required permissions
- `full_name` (required) - Repository full name (owner/repo)
- `branch_name` (optional) - Branch for automation (default: "vibe-dev")

**What Happens:**
1. System validates your access token with the provider
2. Fetches repository information (name, clone URL, default branch)
3. Validates token permissions (branches, labels, PRs, issues, webhooks)
4. Generates a unique webhook secret for this repository
5. Creates repository record in database
6. Creates workspace and agent automatically
7. Initializes the vibe-dev branch (if it doesn't exist)
8. Creates required labels (vibe/pending-ack, vibe/todo-ai, etc.)
9. Creates webhook on the provider
10. Returns complete repository details

**Success Response (201 Created):**
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
  "created_at": "2026-02-06T10:00:00Z"
}
```

**Note:** The `access_token` and `webhook_secret` are never returned in API responses for security.

### Required Token Permissions

Your access token must have the following permissions:

**GitHub:**
- `repo` - Full repository access
- `admin:repo_hook` - Webhook management

**Gitea:**
- `read:repository` - Read repository information
- `write:repository` - Create branches and labels
- `write:issue` - Close issues
- `write:pull_request` - Create pull requests
- `write:webhook` - Create webhooks

**GitLab:**
- `api` - Full API access (includes all required permissions)

### Supported Providers

- **GitHub** - Fully supported (github.com and GitHub Enterprise)
- **Gitea** - Fully supported (self-hosted)
- **GitLab** - Fully supported (gitlab.com and self-hosted)

### Repository Operations

#### List Repositories

```bash
# List all repositories
curl http://localhost:3000/api/repositories

# Filter by validation status
curl "http://localhost:3000/api/repositories?validation_status=valid"

# Filter by repository status
curl "http://localhost:3000/api/repositories?status=idle"
```

**Note:** The `provider_id` filter has been removed. Each repository is now self-contained.

#### Get Repository Details

```bash
curl http://localhost:3000/api/repositories/:id
```

#### Re-initialize Repository

If you need to re-create branches or labels after manual changes:

```bash
curl -X POST http://localhost:3000/api/repositories/:id/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "branch_name": "vibe-dev",
    "create_labels": true
  }'
```

**Note:** This is typically not needed since `POST /api/repositories` automatically initializes everything.

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

Webhooks are automatically created when you add a repository. Each repository has its own unique webhook secret.

**Webhook URL Format:**
```
https://your-domain.com/api/webhooks/:repository_id
```

**Example:**
```
https://vibe-repo.example.com/api/webhooks/1
```

### Setting Up Webhooks Manually (If Needed)

If automatic webhook creation fails, you can set it up manually:

**GitHub:**
1. Go to repository settings → Webhooks → Add webhook
2. Set Payload URL: `https://your-domain.com/api/webhooks/1`
3. Set Content type: `application/json`
4. Set Secret: (contact admin for webhook secret)
5. Select events: `Issues`, `Pull requests`
6. Click "Add webhook"

**Gitea:**
1. Go to repository settings → Webhooks → Add webhook
2. Set Target URL: `https://your-domain.com/api/webhooks/1`
3. Set HTTP Method: `POST`
4. Set POST Content Type: `application/json`
5. Set Secret: (contact admin for webhook secret)
6. Select trigger events: `Issues`, `Pull requests`
7. Click "Add webhook"

**GitLab:**
1. Go to repository settings → Webhooks
2. Set URL: `https://your-domain.com/api/webhooks/1`
3. Set Secret token: (contact admin for webhook secret)
4. Select trigger events: `Issues events`, `Merge request events`
5. Click "Add webhook"

### Migration from v0.3.0

If you're upgrading from v0.3.0, the workflow has changed significantly:

**Before (v0.3.0):**
1. Configure provider via environment variables
2. System auto-syncs all repositories from provider
3. Archive unwanted repositories (e.g., 97 of 100)
4. Initialize desired repositories (e.g., 3)

**After (v0.4.0-mvp):**
1. Add only desired repositories via API (e.g., 3 API calls)
2. Each repository includes its own provider configuration
3. No auto-sync, no archiving needed

**Benefits:**
- 97% fewer API operations for selective usage (3 vs 102)
- Explicit intent - only add what you want
- Per-repository token management (principle of least privilege)
- Support for mixed providers (GitHub + Gitea + GitLab)

See [MIGRATION.md](../../MIGRATION.md) for detailed migration guide.

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
