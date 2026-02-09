# ACP Integration Guide

**Version:** 0.4.0-mvp  
**Last Updated:** 2026-02-08

This document provides a comprehensive guide to the Agent Client Protocol (ACP) integration in VibeRepo.

## Overview

VibeRepo uses the Agent Client Protocol (ACP) to communicate with AI agents in a structured, standardized way. This replaces the previous CLI-based approach with a more robust, real-time communication protocol.

### What is ACP?

Agent Client Protocol (ACP) is an industry-standard protocol for agent communication, supported by major IDEs including Zed, JetBrains, Neovim, and Emacs. It provides:

- **Structured Communication**: JSON-RPC over stdin/stdout
- **Real-time Events**: Streaming of plans, tool calls, and messages
- **Permission System**: Fine-grained control over agent actions
- **Session Management**: Isolated execution contexts
- **Capability Negotiation**: Dynamic feature discovery

### Architecture Changes

**Before (CLI-based):**
```
Task Executor → docker exec → CLI command → Parse output
```

**After (ACP-based):**
```
Task Executor → Agent Manager → ACP Client → Agent Process
                                    ↓
                            Real-time Events
                                    ↓
                            Database Storage
```

### Key Benefits

1. **Real-time Visibility**: Track agent progress with plans and events
2. **Structured Communication**: No more fragile output parsing
3. **Permission Control**: Policy-based security for agent actions
4. **Better Performance**: 10x faster startup with Bun runtime
5. **Standardization**: Compatible with multiple ACP agents

## Agent Configuration

### Environment Variables

Configure agents using environment variables in your `.env` file:

```bash
# Agent Type
AGENT_TYPE=opencode                    # Options: "opencode", "claude-code"

# API Key (optional, required for some agents)
AGENT_API_KEY=sk-xxx                   # Your LLM provider API key

# Model Selection
AGENT_DEFAULT_MODEL=claude-sonnet-4    # Default: "claude-sonnet-4"

# Timeout
AGENT_TIMEOUT_SECONDS=600              # Default: 600 (10 minutes)
```

### Supported Agent Types

#### OpenCode (Default)

OpenCode is the default agent with native ACP support.

**Features:**
- Native ACP support (`opencode acp` command)
- Multi-provider support (OpenAI, Anthropic, Gemini, etc.)
- Optimized for code execution tasks
- Fast startup with Bun runtime

**Configuration:**
```bash
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-xxx              # Anthropic API key
AGENT_DEFAULT_MODEL=claude-sonnet-4
```

**Supported Models:**
- `claude-sonnet-4` (recommended)
- `claude-opus-4`
- `gpt-4`
- `gpt-4-turbo`
- `gemini-pro`

#### Claude Code

Official Anthropic agent with ACP adapter.

**Features:**
- Official Anthropic agent
- Requires ACP adapter layer
- Anthropic API key required

**Configuration:**
```bash
AGENT_TYPE=claude-code
AGENT_API_KEY=sk-ant-xxx              # Anthropic API key required
AGENT_DEFAULT_MODEL=claude-sonnet-4
```

### Agent Lifecycle

1. **Spawn**: Agent Manager spawns agent subprocess with Bun runtime
2. **Initialize**: ACP Client sends `initialize` request with capabilities
3. **Create Session**: ACP Client creates session with working directory
4. **Execute**: Agent receives prompt and executes task
5. **Stream Events**: Agent streams plans, tool calls, and messages
6. **Complete**: Agent signals completion or error
7. **Shutdown**: Graceful shutdown with timeout, force kill if needed

## Permission System

### Default Permission Policy

VibeRepo uses a policy-based permission system to control agent actions:

**Allowed Operations:**
- ✅ Read any file in workspace
- ✅ Write files in workspace directory
- ✅ Execute safe commands (git, cargo, npm, etc.)
- ✅ Search files and content

**Denied Operations:**
- ❌ Write outside workspace directory
- ❌ Execute dangerous commands (rm -rf, dd, mkfs, etc.)
- ❌ Delete system files
- ❌ Modify system configuration

### Command Allowlist

Safe commands that agents can execute:

```
git, cargo, rustc, npm, node, bun, python, pip,
docker, kubectl, make, cmake, gcc, clang, go,
yarn, pnpm, deno, ruby, gem, java, mvn, gradle
```

### Command Denylist

Dangerous commands that are always blocked:

```
rm -rf, dd, mkfs, fdisk, parted, shutdown, reboot,
init, systemctl, service, chmod 777, chown root,
sudo, su, passwd, useradd, userdel, groupadd
```

### Path Validation

All file operations are validated to ensure they stay within the workspace:

```rust
// Example: Validate path is within workspace
fn is_within_workspace(path: &Path, workspace: &Path) -> bool {
    path.canonicalize()
        .ok()
        .and_then(|p| p.strip_prefix(workspace).ok())
        .is_some()
}
```

### Permission Logging

All permission requests and decisions are logged for audit:

```json
{
  "timestamp": "2026-02-08T10:00:00Z",
  "request": {
    "tool_kind": "write",
    "path": "/workspace/src/main.rs",
    "command": null
  },
  "decision": {
    "decision": "allow",
    "reason": "Write within workspace directory"
  },
  "task_id": 123,
  "agent_id": 456
}
```

### Custom Policies

Future versions will support repository-specific permission policies. For now, the default policy is applied to all agents.

## Event Structure

### Event Types

VibeRepo tracks four types of events from agents:

#### 1. Plan Events

Agent's execution plan with steps:

```json
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
```

**Plan Status:**
- `creating` - Plan is being created
- `active` - Plan is being executed
- `completed` - Plan is finished
- `modified` - Plan was updated

**Step Status:**
- `pending` - Not started yet
- `in_progress` - Currently executing
- `completed` - Finished successfully
- `skipped` - Skipped this step

#### 2. Tool Call Events

Agent tool executions:

```json
{
  "type": "tool_call",
  "tool_name": "write_file",
  "args": {
    "path": "src/auth.rs",
    "content": "// Authentication implementation..."
  },
  "result": "success",
  "timestamp": "2026-02-08T10:01:00Z"
}
```

**Common Tools:**
- `read_file` - Read file contents
- `write_file` - Write file contents
- `execute_command` - Run shell command
- `search_files` - Search for files
- `grep_content` - Search file contents

#### 3. Message Events

Agent messages and updates:

```json
{
  "type": "message",
  "content": "Implemented JWT authentication with refresh tokens",
  "level": "info",
  "timestamp": "2026-02-08T10:02:00Z"
}
```

**Message Levels:**
- `info` - Informational message
- `warning` - Warning message
- `error` - Error message
- `debug` - Debug information

#### 4. Completed Events

Task completion signal:

```json
{
  "type": "completed",
  "success": true,
  "summary": "Successfully implemented authentication feature",
  "timestamp": "2026-02-08T10:05:00Z"
}
```

### Database Storage

Events are stored in the `tasks` table as JSONB fields:

```sql
-- tasks table schema
CREATE TABLE tasks (
  id INTEGER PRIMARY KEY,
  -- ... other fields ...
  plans JSONB,           -- Array of PlanEvent
  events JSONB,          -- Array of AgentEvent
  -- ... other fields ...
);
```

**Storage Strategy:**
- Plans: Latest plan is stored (replaces previous)
- Events: Last 100 events are kept (automatic compaction)
- JSONB format allows efficient querying with PostgreSQL

### Event Compaction

To prevent unbounded growth, events are automatically compacted:

```rust
const MAX_EVENTS: usize = 100;

fn compact_events(events: &mut Vec<AgentEvent>) {
    if events.len() > MAX_EVENTS {
        // Keep most recent events
        events.drain(0..events.len() - MAX_EVENTS);
    }
}
```

## API Endpoints

### GET /api/tasks/:id/plans

Retrieve the current execution plan for a task.

**Response:**
```json
{
  "plans": [
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
    }
  ]
}
```

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

**Response:**
```json
{
  "events": [
    {
      "type": "plan",
      "steps": [...],
      "timestamp": "2026-02-08T10:00:00Z"
    },
    {
      "type": "tool_call",
      "tool_name": "read_file",
      "args": {"path": "src/main.rs"},
      "timestamp": "2026-02-08T10:00:30Z"
    },
    {
      "type": "message",
      "content": "Analyzing code structure",
      "level": "info",
      "timestamp": "2026-02-08T10:00:45Z"
    }
  ]
}
```

### GET /api/tasks/:id/progress

Get task progress percentage based on plan completion.

**Response:**
```json
{
  "task_id": 123,
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

### GET /api/tasks/:id/status

Get task status with progress information (enhanced).

**Response:**
```json
{
  "task_id": 123,
  "status": "running",
  "progress": 66.67,
  "started_at": "2026-02-08T10:00:00Z",
  "completed_at": null,
  "created_at": "2026-02-08T09:55:00Z"
}
```

## Usage Examples

### Example 1: Using OpenCode with Anthropic

```bash
# .env configuration
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-api03-xxx
AGENT_DEFAULT_MODEL=claude-sonnet-4
AGENT_TIMEOUT_SECONDS=600

# Start VibeRepo
cargo run

# Create a task (via webhook or API)
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 42,
    "issue_title": "Add authentication",
    "issue_body": "Implement JWT authentication",
    "issue_url": "https://github.com/owner/repo/issues/42"
  }'

# Monitor progress
curl http://localhost:3000/api/tasks/1/progress

# View events
curl http://localhost:3000/api/tasks/1/events
```

### Example 2: Using Claude Code

```bash
# .env configuration
AGENT_TYPE=claude-code
AGENT_API_KEY=sk-ant-api03-xxx
AGENT_DEFAULT_MODEL=claude-sonnet-4
AGENT_TIMEOUT_SECONDS=600

# Start VibeRepo
cargo run
```

### Example 3: Docker Workspace with ACP

```bash
# Build workspace image with Bun and OpenCode
docker build -t vibe-workspace -f docker/workspace/Dockerfile .

# Run workspace container
docker run -d \
  --name workspace-1 \
  -v /path/to/repo:/workspace \
  -e AGENT_TYPE=opencode \
  -e AGENT_API_KEY=sk-ant-xxx \
  vibe-workspace

# Agent runs inside container with ACP
docker exec workspace-1 bun opencode acp
```

### Example 4: Monitoring Agent Progress

```bash
# Get current plan
curl http://localhost:3000/api/tasks/123/plans | jq '.plans[0].steps'

# Get progress percentage
curl http://localhost:3000/api/tasks/123/progress | jq '.progress'

# Stream events (poll every 2 seconds)
while true; do
  curl -s http://localhost:3000/api/tasks/123/events?limit=5 | jq '.events[-1]'
  sleep 2
done

# Filter tool calls only
curl http://localhost:3000/api/tasks/123/events?event_type=tool_call | jq '.events'
```

## Troubleshooting

### Agent Startup Failures

**Problem:** Agent fails to start or times out during initialization.

**Possible Causes:**
1. Bun not installed in Docker image
2. OpenCode not installed globally
3. Invalid API key
4. Network connectivity issues

**Solutions:**

Check Bun installation:
```bash
docker exec workspace-1 bun --version
```

Check OpenCode installation:
```bash
docker exec workspace-1 bun opencode --version
```

Verify ACP support:
```bash
docker exec workspace-1 bun opencode acp --version
```

Check logs:
```bash
docker logs workspace-1
```

Rebuild Docker image:
```bash
docker build --no-cache -t vibe-workspace -f docker/workspace/Dockerfile .
```

### Permission Denied Errors

**Problem:** Agent operations fail with permission denied errors.

**Possible Causes:**
1. Attempting to write outside workspace
2. Executing blocked commands
3. Path validation failure

**Solutions:**

Check permission logs:
```bash
curl http://localhost:3000/api/tasks/123/events?event_type=message | \
  jq '.events[] | select(.content | contains("permission"))'
```

Verify workspace path:
```bash
docker exec workspace-1 pwd
docker exec workspace-1 ls -la /workspace
```

Check file permissions:
```bash
docker exec workspace-1 ls -la /workspace/src
```

### Timeout Errors

**Problem:** Agent operations timeout before completion.

**Possible Causes:**
1. Task too complex for timeout limit
2. Agent stuck in infinite loop
3. Network latency for API calls

**Solutions:**

Increase timeout:
```bash
# In .env
AGENT_TIMEOUT_SECONDS=1200  # 20 minutes
```

Check agent status:
```bash
curl http://localhost:3000/api/tasks/123/status
```

View last events:
```bash
curl http://localhost:3000/api/tasks/123/events?limit=10
```

Force cancel task:
```bash
curl -X POST http://localhost:3000/api/tasks/123/cancel
```

### Docker Image Issues

**Problem:** Docker image build fails or agent not found.

**Possible Causes:**
1. Bun installation failed
2. OpenCode installation failed
3. Network issues during build

**Solutions:**

Check Dockerfile:
```bash
cat docker/workspace/Dockerfile
```

Build with verbose output:
```bash
docker build --progress=plain -t vibe-workspace -f docker/workspace/Dockerfile .
```

Test Bun installation:
```bash
docker run --rm vibe-workspace bun --version
```

Test OpenCode installation:
```bash
docker run --rm vibe-workspace bun opencode --version
```

### API Key Issues

**Problem:** Agent fails with authentication errors.

**Possible Causes:**
1. Invalid API key
2. API key not set
3. Wrong provider for model

**Solutions:**

Verify API key is set:
```bash
echo $AGENT_API_KEY
```

Test API key manually:
```bash
curl https://api.anthropic.com/v1/messages \
  -H "x-api-key: $AGENT_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'
```

Check agent logs:
```bash
curl http://localhost:3000/api/tasks/123/logs
```

### Event Storage Issues

**Problem:** Events not appearing or incomplete.

**Possible Causes:**
1. Database connection issues
2. JSONB serialization errors
3. Event compaction too aggressive

**Solutions:**

Check database connection:
```bash
curl http://localhost:3000/health
```

Query database directly:
```sql
SELECT id, plans, events FROM tasks WHERE id = 123;
```

Check event count:
```bash
curl http://localhost:3000/api/tasks/123/events | jq '.events | length'
```

Verify JSONB format:
```bash
curl http://localhost:3000/api/tasks/123/events | jq '.events[0]'
```

## Migration from CLI Approach

### What Changed

**Before (v0.3.0 and earlier):**
- Task execution via `docker exec` CLI commands
- Output parsing with regex
- No real-time progress tracking
- No structured events
- No permission control

**After (v0.4.0-mvp):**
- Task execution via ACP protocol
- Structured JSON-RPC communication
- Real-time plans and events
- Permission-based security
- 10x faster startup with Bun

### Configuration Changes

**Old Configuration:**
```bash
DEFAULT_AGENT_COMMAND=opencode
DEFAULT_AGENT_TIMEOUT=600
```

**New Configuration:**
```bash
# Keep old config for backward compatibility
DEFAULT_AGENT_COMMAND=opencode
DEFAULT_AGENT_TIMEOUT=600

# Add new ACP config
AGENT_TYPE=opencode
AGENT_API_KEY=sk-xxx
AGENT_DEFAULT_MODEL=claude-sonnet-4
AGENT_TIMEOUT_SECONDS=600
```

### Database Changes

**New Fields in `tasks` Table:**
```sql
ALTER TABLE tasks
ADD COLUMN plans JSONB,
ADD COLUMN events JSONB;
```

**Migration:**
Migrations run automatically on startup. No manual intervention needed.

### API Changes

**New Endpoints:**
- `GET /api/tasks/:id/plans` - Get execution plan
- `GET /api/tasks/:id/events` - Get agent events
- `GET /api/tasks/:id/progress` - Get progress percentage

**Enhanced Endpoints:**
- `GET /api/tasks/:id/status` - Now includes progress percentage

### Code Changes

**Old Task Execution:**
```rust
// Execute CLI command
let output = Command::new("docker")
    .args(&["exec", container_id, "opencode", prompt])
    .output()
    .await?;

// Parse output
let result = parse_output(&output.stdout)?;
```

**New Task Execution:**
```rust
// Spawn agent with ACP
let agent = agent_manager.spawn_agent(config).await?;

// Create session
let session_id = agent.create_session(working_dir).await?;

// Send prompt and stream events
let mut events = agent.prompt(session_id, prompt).await?;

while let Some(event) = events.next().await {
    // Store event in database
    store_event(task_id, event).await?;
}
```

### Backward Compatibility

The old CLI-based execution code is preserved but deprecated:

```rust
// backend/src/services/task_executor_service.rs.backup
// Old implementation kept for reference
```

**Note:** The old approach is no longer supported. All new deployments should use ACP integration.

### Rollback Procedure

If you need to rollback to the old CLI approach:

1. Checkout previous version:
```bash
git checkout v0.3.0
```

2. Rebuild and redeploy:
```bash
cargo build --release
```

3. Database schema is backward compatible (new fields are nullable)

## Performance Improvements

### Startup Time

**Before (Node.js):**
- Cold start: ~100-200ms
- Warm start: ~50-100ms

**After (Bun):**
- Cold start: ~10-20ms (10x faster)
- Warm start: ~5-10ms (10x faster)

### Memory Usage

**Before (Node.js):**
- Base: ~50-100MB per agent
- Peak: ~150-200MB per agent

**After (Bun):**
- Base: ~30-50MB per agent (40% reduction)
- Peak: ~80-120MB per agent (40% reduction)

### Communication Overhead

**Before (CLI):**
- Parse overhead: ~10-50ms per output
- Fragile regex parsing
- No streaming

**After (ACP):**
- Parse overhead: ~1-5ms per event
- Structured JSON parsing
- Real-time streaming

## Best Practices

### 1. Choose the Right Agent

- **OpenCode**: Best for multi-provider support and flexibility
- **Claude Code**: Best for official Anthropic experience

### 2. Set Appropriate Timeouts

```bash
# Simple tasks (< 5 minutes)
AGENT_TIMEOUT_SECONDS=300

# Medium tasks (5-10 minutes)
AGENT_TIMEOUT_SECONDS=600

# Complex tasks (10-20 minutes)
AGENT_TIMEOUT_SECONDS=1200
```

### 3. Monitor Agent Progress

Poll the progress endpoint regularly:

```bash
# Check progress every 5 seconds
watch -n 5 'curl -s http://localhost:3000/api/tasks/123/progress | jq'
```

### 4. Review Permission Logs

Regularly audit permission decisions:

```bash
curl http://localhost:3000/api/tasks/123/events | \
  jq '.events[] | select(.type == "message" and (.content | contains("permission")))'
```

### 5. Handle Failures Gracefully

Implement retry logic for transient failures:

```rust
let max_retries = 3;
for attempt in 0..max_retries {
    match execute_task(task_id).await {
        Ok(_) => break,
        Err(e) if attempt < max_retries - 1 => {
            warn!("Task failed, retrying: {}", e);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        Err(e) => return Err(e),
    }
}
```

## Future Enhancements

### Planned Features

1. **Multi-Agent Orchestration**: Multiple agents working on same task
2. **WebSocket Streaming**: Real-time event streaming to frontend
3. **Custom Permission Policies**: Repository-specific policies
4. **Agent Pooling**: Reuse agent processes for faster execution
5. **Cost Tracking**: Monitor LLM API costs per task
6. **Interactive Approval**: User approval for sensitive operations

### Experimental Features

1. **Agent Collaboration**: Agents can request help from other agents
2. **Learning from History**: Agents learn from past task executions
3. **Automatic Optimization**: System tunes timeouts and policies based on usage

## References

- **ACP Specification**: https://github.com/zed-industries/acp
- **OpenCode Documentation**: https://opencode.ai/docs
- **Bun Documentation**: https://bun.sh/docs
- **VibeRepo API Reference**: [api-reference.md](./api-reference.md)
- **Task Management Guide**: [task-management-guide.md](./task-management-guide.md)

---

**Last Updated:** 2026-02-08  
**Version:** 0.4.0-mvp
