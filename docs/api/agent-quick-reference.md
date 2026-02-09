# Agent Configuration Quick Reference

**Version:** 0.4.0-mvp  
**Last Updated:** 2026-02-08

Quick reference for configuring and using AI agents in VibeRepo.

## Environment Variables

```bash
# Agent Type (required)
AGENT_TYPE=opencode                    # Options: "opencode", "claude-code"

# API Key (optional, required for some agents)
AGENT_API_KEY=sk-ant-api03-xxx         # Your LLM provider API key

# Model Selection (optional)
AGENT_DEFAULT_MODEL=claude-sonnet-4    # Default: "claude-sonnet-4"

# Timeout (optional)
AGENT_TIMEOUT_SECONDS=600              # Default: 600 (10 minutes)
```

## Supported Agents

### OpenCode (Recommended)

**Best for:** Multi-provider support, flexibility, fast startup

```bash
# Configuration
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-api03-xxx         # Anthropic API key
AGENT_DEFAULT_MODEL=claude-sonnet-4

# Supported Models
# - claude-sonnet-4 (recommended)
# - claude-opus-4
# - gpt-4
# - gpt-4-turbo
# - gemini-pro
```

**Features:**
- ✅ Native ACP support
- ✅ Multi-provider (OpenAI, Anthropic, Gemini)
- ✅ Fast startup with Bun (~10-20ms)
- ✅ Optimized for code tasks

### Claude Code

**Best for:** Official Anthropic experience

```bash
# Configuration
AGENT_TYPE=claude-code
AGENT_API_KEY=sk-ant-api03-xxx         # Anthropic API key (required)
AGENT_DEFAULT_MODEL=claude-sonnet-4

# Supported Models
# - claude-sonnet-4
# - claude-opus-4
```

**Features:**
- ✅ Official Anthropic agent
- ✅ ACP adapter included
- ⚠️ Requires Anthropic API key

## Quick Start

### 1. Configure Agent

Create `.env` file:

```bash
# Minimal configuration
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-api03-xxx

# Full configuration
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-api03-xxx
AGENT_DEFAULT_MODEL=claude-sonnet-4
AGENT_TIMEOUT_SECONDS=600
```

### 2. Start VibeRepo

```bash
cargo run
```

### 3. Create Task

Via API:
```bash
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 42,
    "issue_title": "Add authentication",
    "issue_body": "Implement JWT authentication",
    "issue_url": "https://github.com/owner/repo/issues/42"
  }'
```

Via Webhook:
```bash
# Comment on issue with bot mention
@vibe-repo-bot please implement this
```

### 4. Monitor Progress

```bash
# Get progress percentage
curl http://localhost:3000/api/tasks/1/progress

# View execution plan
curl http://localhost:3000/api/tasks/1/plans

# Stream events
curl http://localhost:3000/api/tasks/1/events?limit=10
```

## Common Configurations

### Development (Fast Iteration)

```bash
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-xxx
AGENT_DEFAULT_MODEL=claude-sonnet-4
AGENT_TIMEOUT_SECONDS=300              # 5 minutes
```

### Production (Reliable)

```bash
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-xxx
AGENT_DEFAULT_MODEL=claude-sonnet-4
AGENT_TIMEOUT_SECONDS=1200             # 20 minutes
```

### Cost-Optimized

```bash
AGENT_TYPE=opencode
AGENT_API_KEY=sk-xxx                   # OpenAI key
AGENT_DEFAULT_MODEL=gpt-4-turbo
AGENT_TIMEOUT_SECONDS=600
```

### High-Quality Output

```bash
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-xxx
AGENT_DEFAULT_MODEL=claude-opus-4      # Most capable
AGENT_TIMEOUT_SECONDS=1200
```

## Docker Configuration

### Dockerfile

```dockerfile
FROM ubuntu:22.04

# Install Bun
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

# Install OpenCode
RUN bun install -g opencode-ai

# Verify installation
RUN bun --version && bun opencode --version

# Set working directory
WORKDIR /workspace
```

### Docker Run

```bash
docker run -d \
  --name workspace-1 \
  -v /path/to/repo:/workspace \
  -e AGENT_TYPE=opencode \
  -e AGENT_API_KEY=sk-ant-xxx \
  -e AGENT_DEFAULT_MODEL=claude-sonnet-4 \
  vibe-workspace
```

## Monitoring

### Check Agent Status

```bash
# Task status with progress
curl http://localhost:3000/api/tasks/123/status

# Current plan
curl http://localhost:3000/api/tasks/123/plans | jq '.plans[0].steps'

# Recent events
curl http://localhost:3000/api/tasks/123/events?limit=5
```

### Filter Events

```bash
# Only tool calls
curl http://localhost:3000/api/tasks/123/events?event_type=tool_call

# Only messages
curl http://localhost:3000/api/tasks/123/events?event_type=message

# Recent events (last 10)
curl http://localhost:3000/api/tasks/123/events?limit=10

# Events since timestamp
curl "http://localhost:3000/api/tasks/123/events?since=2026-02-08T10:00:00Z"
```

### Progress Tracking

```bash
# Get progress percentage
curl http://localhost:3000/api/tasks/123/progress | jq '.progress'

# Watch progress (updates every 2 seconds)
watch -n 2 'curl -s http://localhost:3000/api/tasks/123/progress | jq'

# Get current step
curl http://localhost:3000/api/tasks/123/progress | jq '.current_step'
```

## Troubleshooting

### Agent Not Starting

```bash
# Check Bun installation
docker exec workspace-1 bun --version

# Check OpenCode installation
docker exec workspace-1 bun opencode --version

# Verify ACP support
docker exec workspace-1 bun opencode acp --version

# Check logs
docker logs workspace-1
curl http://localhost:3000/api/tasks/123/logs
```

### Permission Errors

```bash
# Check permission logs
curl http://localhost:3000/api/tasks/123/events | \
  jq '.events[] | select(.content | contains("permission"))'

# Verify workspace path
docker exec workspace-1 pwd
docker exec workspace-1 ls -la /workspace
```

### Timeout Issues

```bash
# Increase timeout
AGENT_TIMEOUT_SECONDS=1200             # 20 minutes

# Check last events
curl http://localhost:3000/api/tasks/123/events?limit=10

# Force cancel
curl -X POST http://localhost:3000/api/tasks/123/cancel
```

### API Key Issues

```bash
# Verify API key is set
echo $AGENT_API_KEY

# Test API key manually
curl https://api.anthropic.com/v1/messages \
  -H "x-api-key: $AGENT_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-4","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'
```

## Performance Tips

### Optimize Startup Time

1. **Use Bun instead of Node.js** (10x faster)
   ```bash
   docker exec workspace-1 which bun  # Should return /root/.bun/bin/bun
   ```

2. **Pre-warm containers** (future feature)
   ```bash
   # Keep containers running
   docker run -d --name workspace-pool-1 vibe-workspace sleep infinity
   ```

3. **Use faster models**
   ```bash
   AGENT_DEFAULT_MODEL=claude-sonnet-4  # Faster than opus
   ```

### Optimize Memory Usage

1. **Limit concurrent tasks**
   ```bash
   # Future feature: MAX_CONCURRENT_TASKS=3
   ```

2. **Use memory limits**
   ```bash
   docker run -m 2g ...  # 2GB limit
   ```

3. **Monitor memory**
   ```bash
   docker stats workspace-1
   ```

### Optimize Cost

1. **Use cost-effective models**
   ```bash
   AGENT_DEFAULT_MODEL=gpt-4-turbo      # Cheaper than opus
   ```

2. **Set appropriate timeouts**
   ```bash
   AGENT_TIMEOUT_SECONDS=300            # Fail fast for simple tasks
   ```

3. **Monitor usage** (future feature)
   ```bash
   # Track API costs per task
   curl http://localhost:3000/api/tasks/123/cost
   ```

## Best Practices

### 1. Choose the Right Model

- **Simple tasks**: `claude-sonnet-4` or `gpt-4-turbo`
- **Complex tasks**: `claude-opus-4`
- **Cost-sensitive**: `gpt-4-turbo`

### 2. Set Appropriate Timeouts

- **Simple tasks** (< 5 min): `AGENT_TIMEOUT_SECONDS=300`
- **Medium tasks** (5-10 min): `AGENT_TIMEOUT_SECONDS=600`
- **Complex tasks** (10-20 min): `AGENT_TIMEOUT_SECONDS=1200`

### 3. Monitor Progress

```bash
# Poll progress every 5 seconds
watch -n 5 'curl -s http://localhost:3000/api/tasks/123/progress | jq'
```

### 4. Review Events

```bash
# Check for errors
curl http://localhost:3000/api/tasks/123/events | \
  jq '.events[] | select(.type == "message" and .level == "error")'
```

### 5. Handle Failures

```bash
# Check error message
curl http://localhost:3000/api/tasks/123 | jq '.error_message'

# View last events
curl http://localhost:3000/api/tasks/123/events?limit=10
```

## API Reference

See [api-reference.md](./api-reference.md) for complete API documentation.

### Key Endpoints

- `POST /api/tasks` - Create task
- `GET /api/tasks/:id/status` - Get status with progress
- `GET /api/tasks/:id/plans` - Get execution plan
- `GET /api/tasks/:id/events` - Get agent events
- `GET /api/tasks/:id/progress` - Get progress percentage
- `POST /api/tasks/:id/execute` - Execute task
- `POST /api/tasks/:id/cancel` - Cancel task

## Additional Resources

- **[ACP Integration Guide](./acp-integration.md)** - Complete ACP documentation
- **[Troubleshooting Guide](./troubleshooting.md)** - Common issues and solutions
- **[API Reference](./api-reference.md)** - All API endpoints
- **[User Guide](./user-guide.md)** - Complete usage guide

---

**Last Updated:** 2026-02-08  
**Version:** 0.4.0-mvp
