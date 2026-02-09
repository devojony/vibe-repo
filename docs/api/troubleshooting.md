# Troubleshooting Guide

**Version:** 0.4.0-mvp  
**Last Updated:** 2026-02-08

This guide helps you diagnose and resolve common issues with VibeRepo.

## Table of Contents

- [Agent Issues](#agent-issues)
  - [Agent Startup Failures](#agent-startup-failures)
  - [Agent Timeout Errors](#agent-timeout-errors)
  - [Agent Crashes](#agent-crashes)
- [Permission Issues](#permission-issues)
  - [Permission Denied Errors](#permission-denied-errors)
  - [Command Blocked](#command-blocked)
  - [Path Validation Failures](#path-validation-failures)
- [Docker Issues](#docker-issues)
  - [Container Startup Failures](#container-startup-failures)
  - [Image Build Failures](#image-build-failures)
  - [Volume Mount Issues](#volume-mount-issues)
- [API Issues](#api-issues)
  - [Authentication Errors](#authentication-errors)
  - [API Key Issues](#api-key-issues)
  - [Rate Limiting](#rate-limiting)
- [Database Issues](#database-issues)
  - [Connection Failures](#connection-failures)
  - [Migration Errors](#migration-errors)
  - [JSONB Query Issues](#jsonb-query-issues)
- [Task Execution Issues](#task-execution-issues)
  - [Tasks Stuck in Pending](#tasks-stuck-in-pending)
  - [Tasks Failing Immediately](#tasks-failing-immediately)
  - [PR Creation Failures](#pr-creation-failures)
- [Performance Issues](#performance-issues)
  - [Slow Agent Startup](#slow-agent-startup)
  - [High Memory Usage](#high-memory-usage)
  - [Database Performance](#database-performance)

---

## Agent Issues

### Agent Startup Failures

**Symptoms:**
- Task fails immediately with "Agent startup failed"
- Logs show "Failed to spawn agent process"
- Timeout during agent initialization

**Common Causes:**

1. **Bun not installed in Docker image**
   ```bash
   # Check Bun installation
   docker exec workspace-1 bun --version
   ```
   
   **Solution:** Rebuild Docker image with Bun:
   ```bash
   docker build --no-cache -t vibe-workspace -f docker/workspace/Dockerfile .
   ```

2. **OpenCode not installed globally**
   ```bash
   # Check OpenCode installation
   docker exec workspace-1 bun opencode --version
   ```
   
   **Solution:** Install OpenCode in Docker image:
   ```dockerfile
   RUN bun install -g opencode-ai
   ```

3. **Invalid API key**
   ```bash
   # Verify API key is set
   echo $AGENT_API_KEY
   ```
   
   **Solution:** Set valid API key in `.env`:
   ```bash
   AGENT_API_KEY=sk-ant-api03-xxx
   ```

4. **Network connectivity issues**
   ```bash
   # Test network from container
   docker exec workspace-1 curl -I https://api.anthropic.com
   ```
   
   **Solution:** Check firewall rules and proxy settings.

**Debugging Steps:**

1. Check agent logs:
   ```bash
   curl http://localhost:3000/api/tasks/123/logs
   ```

2. Check container logs:
   ```bash
   docker logs workspace-1
   ```

3. Test agent manually:
   ```bash
   docker exec -it workspace-1 bun opencode acp --version
   ```

4. Check environment variables:
   ```bash
   docker exec workspace-1 env | grep AGENT
   ```

### Agent Timeout Errors

**Symptoms:**
- Task fails with "Agent timeout exceeded"
- Task runs for exactly the timeout duration
- No progress updates after certain point

**Common Causes:**

1. **Task too complex for timeout limit**
   
   **Solution:** Increase timeout in `.env`:
   ```bash
   # For complex tasks (20 minutes)
   AGENT_TIMEOUT_SECONDS=1200
   ```

2. **Agent stuck in infinite loop**
   
   **Solution:** Check last events to identify stuck operation:
   ```bash
   curl http://localhost:3000/api/tasks/123/events?limit=10
   ```

3. **Network latency for API calls**
   
   **Solution:** Use faster model or increase timeout:
   ```bash
   AGENT_DEFAULT_MODEL=claude-sonnet-4  # Faster than opus
   AGENT_TIMEOUT_SECONDS=900
   ```

4. **Agent waiting for user input**
   
   **Solution:** Ensure agent is configured for non-interactive mode.

**Debugging Steps:**

1. Check task progress:
   ```bash
   curl http://localhost:3000/api/tasks/123/progress
   ```

2. View recent events:
   ```bash
   curl http://localhost:3000/api/tasks/123/events?limit=20
   ```

3. Check agent status:
   ```bash
   curl http://localhost:3000/api/tasks/123/status
   ```

4. Force cancel if needed:
   ```bash
   curl -X POST http://localhost:3000/api/tasks/123/cancel
   ```

### Agent Crashes

**Symptoms:**
- Task fails with "Agent process crashed"
- Container exits unexpectedly
- Logs show segmentation fault or panic

**Common Causes:**

1. **Out of memory**
   ```bash
   # Check container memory usage
   docker stats workspace-1
   ```
   
   **Solution:** Increase container memory limit:
   ```bash
   docker run -m 2g ...  # 2GB memory limit
   ```

2. **Bun runtime bug**
   
   **Solution:** Update Bun to latest version:
   ```dockerfile
   RUN curl -fsSL https://bun.sh/install | bash
   ```

3. **Agent code bug**
   
   **Solution:** Check agent logs for stack trace:
   ```bash
   docker logs workspace-1 2>&1 | tail -100
   ```

4. **Disk space exhausted**
   ```bash
   # Check disk usage
   docker exec workspace-1 df -h
   ```
   
   **Solution:** Clean up workspace or increase disk space.

**Debugging Steps:**

1. Check container status:
   ```bash
   docker ps -a | grep workspace-1
   ```

2. Inspect container logs:
   ```bash
   docker logs workspace-1 --tail 200
   ```

3. Check system resources:
   ```bash
   docker stats --no-stream
   ```

4. Restart container:
   ```bash
   docker restart workspace-1
   ```

---

## Permission Issues

### Permission Denied Errors

**Symptoms:**
- Task fails with "Permission denied"
- Events show rejected permission requests
- Agent cannot write files or execute commands

**Common Causes:**

1. **Attempting to write outside workspace**
   
   **Solution:** Ensure all file operations are within workspace:
   ```bash
   # Check workspace path
   docker exec workspace-1 pwd
   ```

2. **Executing blocked commands**
   
   **Solution:** Check command allowlist/denylist in [acp-integration.md](./acp-integration.md#command-allowlist).

3. **File permissions in container**
   ```bash
   # Check file permissions
   docker exec workspace-1 ls -la /workspace
   ```
   
   **Solution:** Fix permissions:
   ```bash
   docker exec workspace-1 chmod -R u+w /workspace
   ```

**Debugging Steps:**

1. Check permission logs:
   ```bash
   curl http://localhost:3000/api/tasks/123/events | \
     jq '.events[] | select(.type == "message" and (.content | contains("permission")))'
   ```

2. View denied operations:
   ```bash
   curl http://localhost:3000/api/tasks/123/events?event_type=message | \
     jq '.events[] | select(.content | contains("denied"))'
   ```

3. Verify workspace path:
   ```bash
   docker exec workspace-1 realpath /workspace
   ```

### Command Blocked

**Symptoms:**
- Specific commands fail with "Command not allowed"
- Events show command in denylist

**Common Causes:**

1. **Command in denylist**
   
   Dangerous commands are blocked by default:
   - `rm -rf`
   - `dd`
   - `mkfs`
   - `shutdown`
   - `sudo`
   
   **Solution:** Use safer alternatives or modify permission policy (future feature).

2. **Command not in allowlist**
   
   Only safe commands are allowed by default. See [acp-integration.md](./acp-integration.md#command-allowlist).
   
   **Solution:** Request command to be added to allowlist if safe.

**Debugging Steps:**

1. Check which command was blocked:
   ```bash
   curl http://localhost:3000/api/tasks/123/events | \
     jq '.events[] | select(.type == "tool_call" and .tool_name == "execute_command")'
   ```

2. Review permission policy:
   See [acp-integration.md](./acp-integration.md#permission-system)

### Path Validation Failures

**Symptoms:**
- File operations fail with "Path outside workspace"
- Symlink operations fail

**Common Causes:**

1. **Absolute paths outside workspace**
   
   **Solution:** Use relative paths within workspace:
   ```bash
   # Bad: /etc/config
   # Good: ./config
   ```

2. **Symlinks pointing outside workspace**
   
   **Solution:** Avoid symlinks or ensure they point within workspace.

3. **Path traversal attempts**
   
   **Solution:** Avoid `../` patterns that escape workspace.

**Debugging Steps:**

1. Check failed paths:
   ```bash
   curl http://localhost:3000/api/tasks/123/events | \
     jq '.events[] | select(.content | contains("outside workspace"))'
   ```

2. Verify workspace boundary:
   ```bash
   docker exec workspace-1 pwd
   docker exec workspace-1 ls -la /workspace
   ```

---

## Docker Issues

### Container Startup Failures

**Symptoms:**
- Container fails to start
- `docker ps` shows container exited
- Workspace creation fails

**Common Causes:**

1. **Port conflicts**
   ```bash
   # Check port usage
   lsof -i :3000
   ```
   
   **Solution:** Use different port or stop conflicting service.

2. **Volume mount issues**
   ```bash
   # Check volume mounts
   docker inspect workspace-1 | jq '.[0].Mounts'
   ```
   
   **Solution:** Ensure mount paths exist and have correct permissions.

3. **Image not found**
   ```bash
   # List images
   docker images | grep vibe-workspace
   ```
   
   **Solution:** Build image:
   ```bash
   docker build -t vibe-workspace -f docker/workspace/Dockerfile .
   ```

4. **Resource limits**
   
   **Solution:** Increase Docker resource limits in Docker Desktop settings.

**Debugging Steps:**

1. Check container status:
   ```bash
   docker ps -a | grep workspace
   ```

2. View container logs:
   ```bash
   docker logs workspace-1
   ```

3. Inspect container:
   ```bash
   docker inspect workspace-1
   ```

4. Try manual start:
   ```bash
   docker run -it --rm vibe-workspace /bin/bash
   ```

### Image Build Failures

**Symptoms:**
- `docker build` fails
- Bun or OpenCode installation fails
- Network timeouts during build

**Common Causes:**

1. **Network issues during build**
   
   **Solution:** Use build cache and retry:
   ```bash
   docker build --network=host -t vibe-workspace -f docker/workspace/Dockerfile .
   ```

2. **Bun installation fails**
   
   **Solution:** Use specific Bun version:
   ```dockerfile
   RUN curl -fsSL https://bun.sh/install | bash -s "bun-v1.0.0"
   ```

3. **OpenCode installation fails**
   
   **Solution:** Install with verbose output:
   ```dockerfile
   RUN bun install -g opencode-ai --verbose
   ```

4. **Disk space issues**
   ```bash
   # Check disk space
   df -h
   ```
   
   **Solution:** Clean up Docker:
   ```bash
   docker system prune -a
   ```

**Debugging Steps:**

1. Build with verbose output:
   ```bash
   docker build --progress=plain --no-cache -t vibe-workspace -f docker/workspace/Dockerfile .
   ```

2. Test intermediate layers:
   ```bash
   docker build --target builder -t test-build -f docker/workspace/Dockerfile .
   docker run -it test-build /bin/bash
   ```

3. Check Dockerfile syntax:
   ```bash
   docker build --check -f docker/workspace/Dockerfile .
   ```

### Volume Mount Issues

**Symptoms:**
- Files not visible in container
- Permission denied when accessing mounted files
- Changes not persisting

**Common Causes:**

1. **Incorrect mount path**
   ```bash
   # Check mounts
   docker inspect workspace-1 | jq '.[0].Mounts'
   ```
   
   **Solution:** Verify mount paths in docker run command.

2. **SELinux issues (Linux)**
   
   **Solution:** Add `:z` flag to volume mount:
   ```bash
   docker run -v /path/to/repo:/workspace:z ...
   ```

3. **File ownership mismatch**
   ```bash
   # Check ownership
   docker exec workspace-1 ls -la /workspace
   ```
   
   **Solution:** Fix ownership:
   ```bash
   docker exec workspace-1 chown -R $(id -u):$(id -g) /workspace
   ```

**Debugging Steps:**

1. Verify mount exists:
   ```bash
   docker exec workspace-1 ls -la /workspace
   ```

2. Check mount permissions:
   ```bash
   docker exec workspace-1 stat /workspace
   ```

3. Test write access:
   ```bash
   docker exec workspace-1 touch /workspace/test.txt
   ```

---

## API Issues

### Authentication Errors

**Symptoms:**
- API calls fail with 401 Unauthorized
- Git operations fail with authentication errors

**Common Causes:**

1. **Invalid access token**
   
   **Solution:** Generate new token with required permissions:
   - GitHub: Settings → Developer settings → Personal access tokens
   - Required scopes: `repo`, `workflow`, `write:packages`

2. **Token expired**
   
   **Solution:** Regenerate token and update repository configuration.

3. **Insufficient permissions**
   
   **Solution:** Ensure token has all required scopes.

**Debugging Steps:**

1. Test token manually:
   ```bash
   curl -H "Authorization: token ghp_xxx" https://api.github.com/user
   ```

2. Check repository access:
   ```bash
   curl -H "Authorization: token ghp_xxx" https://api.github.com/repos/owner/repo
   ```

3. Verify token scopes:
   ```bash
   curl -I -H "Authorization: token ghp_xxx" https://api.github.com/user | grep X-OAuth-Scopes
   ```

### API Key Issues

**Symptoms:**
- Agent fails with "Invalid API key"
- LLM API calls fail with 401

**Common Causes:**

1. **API key not set**
   ```bash
   # Check if set
   echo $AGENT_API_KEY
   ```
   
   **Solution:** Set in `.env`:
   ```bash
   AGENT_API_KEY=sk-ant-api03-xxx
   ```

2. **Invalid API key format**
   
   **Solution:** Verify key format:
   - Anthropic: `sk-ant-api03-xxx`
   - OpenAI: `sk-xxx`

3. **API key for wrong provider**
   
   **Solution:** Ensure API key matches model provider:
   ```bash
   # For Claude models
   AGENT_API_KEY=sk-ant-xxx
   AGENT_DEFAULT_MODEL=claude-sonnet-4
   
   # For GPT models
   AGENT_API_KEY=sk-xxx
   AGENT_DEFAULT_MODEL=gpt-4
   ```

**Debugging Steps:**

1. Test API key manually:
   ```bash
   # Anthropic
   curl https://api.anthropic.com/v1/messages \
     -H "x-api-key: $AGENT_API_KEY" \
     -H "anthropic-version: 2023-06-01" \
     -H "content-type: application/json" \
     -d '{"model":"claude-sonnet-4","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'
   ```

2. Check agent logs:
   ```bash
   curl http://localhost:3000/api/tasks/123/logs | grep -i "api key"
   ```

### Rate Limiting

**Symptoms:**
- API calls fail with 429 Too Many Requests
- Intermittent failures during high load

**Common Causes:**

1. **GitHub API rate limit exceeded**
   
   **Solution:** Use authenticated requests (higher limits):
   ```bash
   # Check rate limit
   curl -H "Authorization: token ghp_xxx" https://api.github.com/rate_limit
   ```

2. **LLM API rate limit exceeded**
   
   **Solution:** Reduce concurrent tasks or upgrade API plan.

3. **Too many concurrent requests**
   
   **Solution:** Implement request throttling (future feature).

**Debugging Steps:**

1. Check rate limit status:
   ```bash
   curl -I https://api.github.com/rate_limit
   ```

2. Monitor request frequency:
   ```bash
   docker logs vibe-repo | grep "rate limit"
   ```

---

## Database Issues

### Connection Failures

**Symptoms:**
- Application fails to start with "Database connection failed"
- API calls fail with 500 Internal Server Error

**Common Causes:**

1. **Invalid DATABASE_URL**
   ```bash
   # Check DATABASE_URL
   echo $DATABASE_URL
   ```
   
   **Solution:** Fix DATABASE_URL in `.env`:
   ```bash
   # SQLite
   DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
   
   # PostgreSQL
   DATABASE_URL=postgresql://user:pass@localhost:5432/vibe_repo
   ```

2. **Database file permissions (SQLite)**
   ```bash
   # Check permissions
   ls -la ./data/vibe-repo/db/
   ```
   
   **Solution:** Fix permissions:
   ```bash
   chmod 644 ./data/vibe-repo/db/vibe-repo.db
   ```

3. **PostgreSQL not running**
   ```bash
   # Check PostgreSQL status
   pg_isready -h localhost -p 5432
   ```
   
   **Solution:** Start PostgreSQL:
   ```bash
   sudo systemctl start postgresql
   ```

**Debugging Steps:**

1. Test database connection:
   ```bash
   # SQLite
   sqlite3 ./data/vibe-repo/db/vibe-repo.db "SELECT 1;"
   
   # PostgreSQL
   psql $DATABASE_URL -c "SELECT 1;"
   ```

2. Check application logs:
   ```bash
   docker logs vibe-repo | grep -i database
   ```

3. Verify database exists:
   ```bash
   # SQLite
   ls -la ./data/vibe-repo/db/
   
   # PostgreSQL
   psql -l | grep vibe_repo
   ```

### Migration Errors

**Symptoms:**
- Application fails to start with "Migration failed"
- Database schema out of sync

**Common Causes:**

1. **Incomplete migration**
   
   **Solution:** Run migrations manually:
   ```bash
   cargo run -- migrate
   ```

2. **Schema conflict**
   
   **Solution:** Reset database (development only):
   ```bash
   rm ./data/vibe-repo/db/vibe-repo.db
   cargo run  # Recreates database
   ```

3. **Migration version mismatch**
   
   **Solution:** Check migration status:
   ```bash
   sqlite3 ./data/vibe-repo/db/vibe-repo.db "SELECT * FROM seaorm_migration;"
   ```

**Debugging Steps:**

1. Check migration logs:
   ```bash
   docker logs vibe-repo | grep -i migration
   ```

2. Verify schema:
   ```bash
   sqlite3 ./data/vibe-repo/db/vibe-repo.db ".schema tasks"
   ```

3. List applied migrations:
   ```bash
   sqlite3 ./data/vibe-repo/db/vibe-repo.db "SELECT * FROM seaorm_migration;"
   ```

### JSONB Query Issues

**Symptoms:**
- Events or plans not returned correctly
- JSON parsing errors in logs

**Common Causes:**

1. **Invalid JSON in database**
   
   **Solution:** Check JSON validity:
   ```sql
   SELECT id, plans FROM tasks WHERE json_valid(plans) = 0;
   ```

2. **SQLite JSON limitations**
   
   **Solution:** Use PostgreSQL for better JSONB support:
   ```bash
   DATABASE_URL=postgresql://user:pass@localhost:5432/vibe_repo
   ```

3. **Large JSONB fields**
   
   **Solution:** Events are automatically compacted to last 100 entries.

**Debugging Steps:**

1. Query JSONB directly:
   ```sql
   SELECT id, json_extract(plans, '$[0].steps') FROM tasks WHERE id = 123;
   ```

2. Check JSONB size:
   ```sql
   SELECT id, length(plans) as plans_size, length(events) as events_size FROM tasks;
   ```

3. Validate JSON:
   ```bash
   curl http://localhost:3000/api/tasks/123/events | jq '.'
   ```

---

## Task Execution Issues

### Tasks Stuck in Pending

**Symptoms:**
- Tasks remain in "pending" status
- No progress updates
- Task executor not picking up tasks

**Common Causes:**

1. **Task executor service not running**
   
   **Solution:** Check service status:
   ```bash
   docker logs vibe-repo | grep "TaskExecutorService"
   ```

2. **Workspace not ready**
   
   **Solution:** Check workspace status:
   ```bash
   curl http://localhost:3000/api/repositories/1 | jq '.workspace_status'
   ```

3. **Agent not configured**
   
   **Solution:** Verify agent configuration:
   ```bash
   echo $AGENT_TYPE
   echo $AGENT_API_KEY
   ```

**Debugging Steps:**

1. Check task status:
   ```bash
   curl http://localhost:3000/api/tasks/123/status
   ```

2. Manually trigger execution:
   ```bash
   curl -X POST http://localhost:3000/api/tasks/123/execute
   ```

3. Check service logs:
   ```bash
   docker logs vibe-repo | tail -100
   ```

### Tasks Failing Immediately

**Symptoms:**
- Tasks fail within seconds
- No events or plans generated
- Error message in task

**Common Causes:**

1. **Agent startup failure**
   
   See [Agent Startup Failures](#agent-startup-failures)

2. **Invalid issue description**
   
   **Solution:** Ensure issue has clear description:
   ```json
   {
     "issue_title": "Add authentication",
     "issue_body": "Implement JWT-based authentication with refresh tokens"
   }
   ```

3. **Workspace not accessible**
   
   **Solution:** Check workspace container:
   ```bash
   docker ps | grep workspace
   ```

**Debugging Steps:**

1. Check error message:
   ```bash
   curl http://localhost:3000/api/tasks/123 | jq '.error_message'
   ```

2. View task logs:
   ```bash
   curl http://localhost:3000/api/tasks/123/logs
   ```

3. Check last events:
   ```bash
   curl http://localhost:3000/api/tasks/123/events?limit=5
   ```

### PR Creation Failures

**Symptoms:**
- Task completes but no PR created
- PR creation fails with error
- Branch pushed but PR not opened

**Common Causes:**

1. **Insufficient permissions**
   
   **Solution:** Ensure token has `repo` scope.

2. **Branch already has PR**
   
   **Solution:** Check existing PRs:
   ```bash
   curl -H "Authorization: token ghp_xxx" \
     https://api.github.com/repos/owner/repo/pulls?head=owner:branch-name
   ```

3. **No changes in branch**
   
   **Solution:** Verify branch has commits:
   ```bash
   docker exec workspace-1 git log origin/main..HEAD
   ```

**Debugging Steps:**

1. Check PR status:
   ```bash
   curl http://localhost:3000/api/tasks/123 | jq '.pr_url'
   ```

2. Manually create PR:
   ```bash
   curl -X POST http://localhost:3000/api/tasks/123/create-pr
   ```

3. Check git operations:
   ```bash
   docker exec workspace-1 git status
   docker exec workspace-1 git log --oneline -5
   ```

---

## Performance Issues

### Slow Agent Startup

**Symptoms:**
- Tasks take long time to start
- High latency before first event

**Common Causes:**

1. **Using Node.js instead of Bun**
   
   **Solution:** Ensure Bun is installed and used:
   ```bash
   docker exec workspace-1 which bun
   ```

2. **Cold start overhead**
   
   **Solution:** Implement agent pooling (future feature).

3. **Large Docker image**
   
   **Solution:** Optimize Dockerfile:
   ```dockerfile
   # Use multi-stage build
   FROM ubuntu:22.04 AS builder
   # ... build steps ...
   
   FROM ubuntu:22.04
   COPY --from=builder /usr/local/bin/bun /usr/local/bin/
   ```

**Debugging Steps:**

1. Measure startup time:
   ```bash
   time docker exec workspace-1 bun opencode acp --version
   ```

2. Profile agent startup:
   ```bash
   docker exec workspace-1 bun --inspect opencode acp
   ```

### High Memory Usage

**Symptoms:**
- Container using excessive memory
- System slowdown
- OOM kills

**Common Causes:**

1. **Too many concurrent tasks**
   
   **Solution:** Limit concurrent tasks (future feature).

2. **Memory leak in agent**
   
   **Solution:** Restart container periodically:
   ```bash
   docker restart workspace-1
   ```

3. **Large event storage**
   
   **Solution:** Events are automatically compacted to 100 entries.

**Debugging Steps:**

1. Check memory usage:
   ```bash
   docker stats workspace-1 --no-stream
   ```

2. Monitor over time:
   ```bash
   docker stats workspace-1
   ```

3. Check process memory:
   ```bash
   docker exec workspace-1 ps aux --sort=-%mem | head -10
   ```

### Database Performance

**Symptoms:**
- Slow API responses
- High database CPU usage
- Query timeouts

**Common Causes:**

1. **Large JSONB fields**
   
   **Solution:** Use PostgreSQL for better JSONB performance.

2. **Missing indexes**
   
   **Solution:** Indexes are created automatically by migrations.

3. **Too many events**
   
   **Solution:** Events are automatically compacted.

**Debugging Steps:**

1. Check query performance:
   ```sql
   EXPLAIN QUERY PLAN SELECT * FROM tasks WHERE id = 123;
   ```

2. Monitor database size:
   ```bash
   ls -lh ./data/vibe-repo/db/vibe-repo.db
   ```

3. Vacuum database (SQLite):
   ```bash
   sqlite3 ./data/vibe-repo/db/vibe-repo.db "VACUUM;"
   ```

---

## Getting Help

If you're still experiencing issues after following this guide:

1. **Check Logs:**
   ```bash
   docker logs vibe-repo --tail 200
   docker logs workspace-1 --tail 200
   ```

2. **Enable Debug Logging:**
   ```bash
   RUST_LOG=debug cargo run
   ```

3. **Collect Diagnostic Info:**
   ```bash
   # System info
   docker version
   docker info
   
   # Application info
   curl http://localhost:3000/health
   
   # Task info
   curl http://localhost:3000/api/tasks/123
   curl http://localhost:3000/api/tasks/123/events
   ```

4. **Report Issue:**
   - GitHub Issues: https://github.com/your-org/vibe-repo/issues
   - Include logs, configuration, and steps to reproduce

---

**Last Updated:** 2026-02-08  
**Version:** 0.4.0-mvp
