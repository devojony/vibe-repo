# Simplified MVP Deployment Guide

**Version:** v0.4.0-mvp  
**Status:** Simplified MVP - Production Ready

## Overview

This is the simplified MVP version of VibeRepo, designed for minimal complexity while maintaining core Issue-to-PR automation functionality.

## Key Differences from Full Version

### Removed Features
- ❌ Issue polling service (webhook-only)
- ❌ Webhook retry and cleanup services
- ❌ WebSocket real-time log streaming
- ❌ Task execution history tracking
- ❌ Init script service
- ❌ Task failure analyzer
- ❌ Health check service
- ❌ Image management service
- ❌ Multi-agent support (single agent per repository)
- ❌ Task retry mechanism
- ❌ Provider management API (environment variable configuration)

### Simplified Features
- ✅ Single agent per repository (configured via environment variables)
- ✅ Logs stored in tasks table (last_log field, 10MB limit)
- ✅ Webhook-based task creation only
- ✅ Simplified task status machine (Pending → Running → Completed/Failed)
- ✅ Environment variable configuration (no database configuration)

### Core Features Retained
- ✅ Repository management
- ✅ Webhook integration (GitHub)
- ✅ Task creation and execution
- ✅ Docker container management
- ✅ PR creation and Issue closure
- ✅ Log query API

## Environment Variables

Create a `.env` file in the project root:

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

# Logging
RUST_LOG=info
```

### Required Variables

1. **GITHUB_TOKEN**: GitHub personal access token with repo permissions
2. **WEBHOOK_SECRET**: Secret for webhook signature verification (generate with `openssl rand -hex 32`)

### Optional Variables

- **GITHUB_BASE_URL**: GitHub API base URL (default: https://api.github.com)
- **DEFAULT_AGENT_COMMAND**: Default agent command (default: opencode)
- **DEFAULT_AGENT_TIMEOUT**: Default agent timeout in seconds (default: 600)
- **DEFAULT_DOCKER_IMAGE**: Default Docker image (default: ubuntu:22.04)
- **RUST_LOG**: Log level (default: info)

## Database Setup

The simplified MVP uses SQLite by default. The database will be automatically created on first run.

### SQLite (Default)

```bash
# Create data directory
mkdir -p data/vibe-repo/db

# Database will be created automatically at:
# ./data/vibe-repo/db/vibe-repo.db
```

### PostgreSQL (Optional)

To use PostgreSQL instead:

```bash
# Update DATABASE_URL in .env
DATABASE_URL=postgresql://user:password@localhost:5432/vibe_repo
```

## Docker Deployment

### Using Docker Compose (Recommended)

```bash
# 1. Clone the repository
git clone https://github.com/your-org/vibe-repo.git
cd vibe-repo

# 2. Checkout the simplified MVP branch
git checkout mvp-simplified

# 3. Create .env file
cp .env.example .env
# Edit .env and set required variables

# 4. Start the service
docker-compose up -d

# 5. Check logs
docker-compose logs -f vibe-repo-api

# 6. Verify health
curl http://localhost:3000/health
```

### Using Docker Directly

```bash
# 1. Build the image
cd backend
docker build -t vibe-repo:mvp .

# 2. Run the container
docker run -d \
  --name vibe-repo-api \
  -p 3000:3000 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v $(pwd)/data:/data/vibe-repo \
  -e DATABASE_URL=sqlite:/data/vibe-repo/db/vibe-repo.db?mode=rwc \
  -e GITHUB_TOKEN=your_token \
  -e WEBHOOK_SECRET=your_secret \
  vibe-repo:mvp

# 3. Check logs
docker logs -f vibe-repo-api
```

## Native Deployment

### Prerequisites

- Rust 1.75+ (Edition 2021)
- Docker (for container management)
- SQLite or PostgreSQL

### Build and Run

```bash
# 1. Clone and checkout
git clone https://github.com/your-org/vibe-repo.git
cd vibe-repo
git checkout mvp-simplified

# 2. Create .env file
cp .env.example .env
# Edit .env and set required variables

# 3. Build
cd backend
cargo build --release

# 4. Run
./target/release/vibe-repo

# Or use cargo run
cargo run --release
```

## API Endpoints

The simplified MVP exposes 10 core endpoints:

### Repository Management
- `POST /repositories` - Create repository
- `GET /repositories` - List repositories
- `GET /repositories/:id` - Get repository details
- `DELETE /repositories/:id` - Delete repository

### Webhook Integration
- `POST /webhooks/github` - GitHub webhook handler

### Task Management
- `GET /tasks` - List tasks
- `POST /tasks/:id/execute` - Execute task
- `GET /tasks/:id/logs` - Get task logs
- `GET /tasks/:id/status` - Get task status
- `DELETE /tasks/:id` - Delete task

For detailed API documentation, see [API Reference](../api/api-reference.md).

## Webhook Configuration

### GitHub Webhook Setup

1. Go to your repository settings on GitHub
2. Navigate to Webhooks → Add webhook
3. Configure:
   - **Payload URL**: `https://your-domain.com/webhooks/github`
   - **Content type**: `application/json`
   - **Secret**: Use the same value as `WEBHOOK_SECRET` in .env
   - **Events**: Select "Issues" and "Pull requests"
4. Save the webhook

### Webhook Events

The system automatically creates tasks for:
- Issue opened
- Issue reopened
- Issue labeled (with specific labels)

## Monitoring and Logs

### Application Logs

```bash
# Docker Compose
docker-compose logs -f vibe-repo-api

# Docker
docker logs -f vibe-repo-api

# Native
# Logs are written to stdout/stderr
```

### Task Logs

Query task logs via API:

```bash
# Get task logs
curl http://localhost:3000/tasks/{task_id}/logs

# Get task status
curl http://localhost:3000/tasks/{task_id}/status
```

### Log Retention

- Task logs are stored in the `tasks.last_log` field
- Maximum log size: 10MB per task
- Logs are truncated if they exceed the limit
- No automatic cleanup (manual deletion required)

## Troubleshooting

### Common Issues

**1. Database connection error**
```bash
# Check database file permissions
ls -la data/vibe-repo/db/

# Ensure directory exists
mkdir -p data/vibe-repo/db
```

**2. Docker socket permission denied**
```bash
# Add user to docker group
sudo usermod -aG docker $USER

# Or run with sudo (not recommended for production)
sudo docker-compose up -d
```

**3. Webhook signature verification failed**
```bash
# Ensure WEBHOOK_SECRET matches GitHub webhook secret
# Check .env file and GitHub webhook configuration
```

**4. Task execution timeout**
```bash
# Increase DEFAULT_AGENT_TIMEOUT in .env
DEFAULT_AGENT_TIMEOUT=1200  # 20 minutes
```

### Debug Mode

Enable debug logging:

```bash
# In .env
RUST_LOG=debug

# Or set environment variable
export RUST_LOG=debug
cargo run
```

## Migration from Full Version

If you're migrating from the full version (v0.3.0), note the following:

### Database Changes

The simplified MVP has a different database schema:
- Removed tables: `webhook_configs`, `init_scripts`, `task_executions`, `workspaces`
- Modified tables: `tasks` (added `last_log`), `repositories` (added agent fields), `agents` (removed `enabled`)

**Migration is not automatic.** You'll need to:
1. Export data from the full version
2. Transform data to match the simplified schema
3. Import into the simplified version

### Configuration Changes

- Provider configuration moved from database to environment variables
- Agent configuration moved from database to environment variables
- Webhook configuration moved from database to environment variables

### API Changes

- Removed endpoints: `/providers`, `/workspaces`, `/agents`, `/stats`, `/health`
- Removed WebSocket endpoint: `/tasks/:id/logs/stream`
- Simplified task creation (no agent_id parameter)

## Security Considerations

### Production Deployment

1. **Use strong secrets**
   ```bash
   # Generate webhook secret
   openssl rand -hex 32
   ```

2. **Use HTTPS**
   - Deploy behind a reverse proxy (nginx, Caddy)
   - Use Let's Encrypt for SSL certificates

3. **Restrict Docker socket access**
   - Run with minimal privileges
   - Consider using Docker socket proxy

4. **Database security**
   - Use PostgreSQL for production
   - Enable SSL connections
   - Use strong passwords

5. **Network security**
   - Use firewall rules
   - Restrict API access
   - Use VPN for internal access

## Performance Tuning

### Database

```bash
# Increase connection pool size
DATABASE_MAX_CONNECTIONS=20
```

### Agent Timeout

```bash
# Adjust based on task complexity
DEFAULT_AGENT_TIMEOUT=1200  # 20 minutes
```

### Docker Resources

```bash
# In docker-compose.yml, add resource limits
services:
  vibe-repo-api:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
```

## Support

For issues and questions:
- GitHub Issues: https://github.com/your-org/vibe-repo/issues
- Documentation: https://github.com/your-org/vibe-repo/tree/mvp-simplified/docs

---

**Last Updated:** 2026-02-06  
**Version:** v0.4.0-mvp
