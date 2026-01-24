# Environment Variables Reference

**Version:** 0.3.0  
**Last Updated:** 2026-01-24

This document provides a comprehensive reference for all environment variables used in VibeRepo.

## Table of Contents

- [Overview](#overview)
- [Database Configuration](#database-configuration)
- [Server Configuration](#server-configuration)
- [Webhook Configuration](#webhook-configuration)
- [Issue Polling Configuration](#issue-polling-configuration)
- [Workspace Configuration](#workspace-configuration)
- [WebSocket Configuration](#websocket-configuration)
- [Quick Start Examples](#quick-start-examples)

---

## Overview

VibeRepo uses environment variables for configuration with sensible defaults. All variables are optional unless marked as **Required**.

**Configuration Priority:**
1. Environment variables (highest priority)
2. `.env` file
3. Default values (lowest priority)

**Loading `.env` file:**
```bash
# Create .env file in project root
cp .env.example .env

# Edit with your values
vim .env
```

---

## Database Configuration

### DATABASE_URL

Database connection URL supporting SQLite and PostgreSQL.

- **Type:** String
- **Default:** `sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc`
- **Required:** No

**SQLite Example:**
```bash
DATABASE_URL="sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc"
```

**PostgreSQL Example:**
```bash
DATABASE_URL="postgresql://user:password@localhost:5432/vibe_repo"
```

**Notes:**
- SQLite: Recommended for development
- PostgreSQL: Recommended for production
- `mode=rwc` creates SQLite database if it doesn't exist

### DATABASE_MAX_CONNECTIONS

Maximum number of database connections in the pool.

- **Type:** Integer
- **Default:** `10`
- **Required:** No
- **Range:** 1-100

**Example:**
```bash
DATABASE_MAX_CONNECTIONS=20
```

**Recommendations:**
- Development: 5-10 connections
- Production: 20-50 connections
- Adjust based on concurrent request load

---

## Server Configuration

### SERVER_HOST

Server bind host address.

- **Type:** String
- **Default:** `0.0.0.0`
- **Required:** No

**Examples:**
```bash
# Listen on all interfaces (default)
SERVER_HOST="0.0.0.0"

# Listen on localhost only
SERVER_HOST="127.0.0.1"

# Listen on specific IP
SERVER_HOST="192.168.1.100"
```

**Security Notes:**
- `0.0.0.0`: Accessible from any network interface
- `127.0.0.1`: Localhost only (more secure for development)

### SERVER_PORT

Server bind port number.

- **Type:** Integer
- **Default:** `3000`
- **Required:** No
- **Range:** 1-65535

**Example:**
```bash
SERVER_PORT=8080
```

**Common Ports:**
- `3000`: Default development port
- `8080`: Alternative HTTP port
- `80`: Standard HTTP (requires root/admin)
- `443`: Standard HTTPS (requires root/admin)

---

## Webhook Configuration

### WEBHOOK_DOMAIN

Base domain for webhook URLs.

- **Type:** String
- **Default:** `http://localhost:3000`
- **Required:** No

**Examples:**
```bash
# Development
WEBHOOK_DOMAIN="http://localhost:3000"

# Production
WEBHOOK_DOMAIN="https://vibe-repo.example.com"
```

**Notes:**
- Must be accessible from your Git provider
- Use HTTPS in production
- No trailing slash

### WEBHOOK_SECRET_KEY

Secret key for signing webhook payloads.

- **Type:** String
- **Default:** `default-webhook-secret-change-in-production`
- **Required:** Yes (production)

**Example:**
```bash
WEBHOOK_SECRET_KEY="your-secure-random-secret-key-here"
```

**Security:**
- **CRITICAL**: Change default in production
- Use strong random string (32+ characters)
- Keep secret, never commit to version control

**Generate secure key:**
```bash
openssl rand -hex 32
```

### WEBHOOK_BOT_USERNAME

Bot username for mention detection in webhook events.

- **Type:** String
- **Default:** `vibe-repo-bot`
- **Required:** No

**Example:**
```bash
WEBHOOK_BOT_USERNAME="my-bot"
```

### WEBHOOK_MAX_RETRIES

Maximum number of webhook retry attempts.

- **Type:** Integer
- **Default:** `5`
- **Required:** No
- **Range:** 0-10

**Example:**
```bash
WEBHOOK_MAX_RETRIES=3
```

### WEBHOOK_INITIAL_DELAY_SECS

Initial retry delay in seconds.

- **Type:** Integer
- **Default:** `60` (1 minute)
- **Required:** No

**Example:**
```bash
WEBHOOK_INITIAL_DELAY_SECS=30
```

### WEBHOOK_MAX_DELAY_SECS

Maximum retry delay in seconds.

- **Type:** Integer
- **Default:** `3600` (1 hour)
- **Required:** No

**Example:**
```bash
WEBHOOK_MAX_DELAY_SECS=1800
```

### WEBHOOK_BACKOFF_MULTIPLIER

Exponential backoff multiplier for retries.

- **Type:** Float
- **Default:** `2.0`
- **Required:** No

**Example:**
```bash
WEBHOOK_BACKOFF_MULTIPLIER=1.5
```

**Retry Delay Calculation:**
```
delay = min(initial_delay * (multiplier ^ attempt), max_delay)
```

### WEBHOOK_RETRY_POLLING_FALLBACK_THRESHOLD

Retry count threshold to enable polling fallback.

- **Type:** Integer
- **Default:** `5`
- **Required:** No

**Example:**
```bash
WEBHOOK_RETRY_POLLING_FALLBACK_THRESHOLD=3
```

**Behavior:**
- After N failed webhook deliveries, system falls back to polling
- Helps ensure reliability when webhooks are unreliable

---

## Issue Polling Configuration

### ISSUE_POLLING_ENABLED

Enable or disable issue polling.

- **Type:** Boolean
- **Default:** `false`
- **Required:** No

**Example:**
```bash
ISSUE_POLLING_ENABLED=true
```

**Notes:**
- Set to `true` to enable automatic issue polling
- Polling runs at configured interval
- Can be used alongside webhooks as fallback

### ISSUE_POLLING_INTERVAL_SECONDS

Polling interval in seconds.

- **Type:** Integer
- **Default:** `300` (5 minutes)
- **Required:** No
- **Range:** 60-86400 (1 minute to 24 hours)

**Example:**
```bash
ISSUE_POLLING_INTERVAL_SECONDS=600
```

**Recommendations:**
- Development: 300 seconds (5 minutes)
- Production: 300-600 seconds
- Lower values increase API usage

### ISSUE_POLLING_REQUIRED_LABELS

Comma-separated list of required labels for issues to be processed.

- **Type:** String (comma-separated)
- **Default:** `vibe-auto`
- **Required:** No

**Examples:**
```bash
# Single label
ISSUE_POLLING_REQUIRED_LABELS="vibe-auto"

# Multiple labels (OR logic)
ISSUE_POLLING_REQUIRED_LABELS="vibe-auto,bug,enhancement"
```

**Behavior:**
- Issues must have at least one of these labels
- Empty value processes all issues

### ISSUE_POLLING_BOT_USERNAME

Bot username to filter out bot-created issues.

- **Type:** String
- **Default:** `vibe-repo-bot`
- **Required:** No

**Example:**
```bash
ISSUE_POLLING_BOT_USERNAME="my-bot"
```

**Purpose:**
- Prevents bot from processing its own issues
- Avoids infinite loops

### ISSUE_POLLING_MAX_ISSUE_AGE_DAYS

Maximum age of issues to process (in days).

- **Type:** Integer
- **Default:** `30`
- **Required:** No

**Example:**
```bash
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=7
```

**Purpose:**
- Ignores old issues
- Focuses on recent work
- Set to `0` to process all issues

### ISSUE_POLLING_MAX_CONCURRENT

Maximum number of concurrent repository polls.

- **Type:** Integer
- **Default:** `10`
- **Required:** No
- **Range:** 1-50

**Example:**
```bash
ISSUE_POLLING_MAX_CONCURRENT=5
```

**Performance:**
- Higher values = faster polling
- Lower values = less API load
- Adjust based on Git provider rate limits

### ISSUE_POLLING_MAX_RETRIES

Maximum number of retry attempts for rate-limited requests.

- **Type:** Integer
- **Default:** `3`
- **Required:** No

**Example:**
```bash
ISSUE_POLLING_MAX_RETRIES=5
```

---

## Workspace Configuration

### WORKSPACE_BASE_DIR

Base directory for all workspace containers.

- **Type:** String
- **Default:** `./data/vibe-repo/workspaces`
- **Required:** No

**Example:**
```bash
WORKSPACE_BASE_DIR="/var/lib/vibe-repo/workspaces"
```

**Notes:**
- Directory is created automatically if it doesn't exist
- Must have write permissions
- Each workspace gets a subdirectory

**Directory Structure:**
```
workspaces/
├── workspace-1/
│   ├── repo/
│   └── logs/
├── workspace-2/
│   ├── repo/
│   └── logs/
└── ...
```

---

## WebSocket Configuration

### WEBSOCKET_AUTH_TOKEN

Authentication token for WebSocket connections.

- **Type:** String
- **Default:** None (authentication disabled)
- **Required:** No (Yes for production)

**Example:**
```bash
WEBSOCKET_AUTH_TOKEN="your-secure-websocket-token"
```

**Security:**
- **CRITICAL**: Set in production to secure WebSocket connections
- If not set, WebSocket authentication is disabled
- Use strong random string (32+ characters)

**Generate secure token:**
```bash
openssl rand -hex 32
```

**Usage:**
```javascript
// Connect with authentication
const ws = new WebSocket('ws://localhost:3000/api/tasks/123/logs?token=your-token');
```

**Security Best Practices:**
1. Always set in production
2. Use HTTPS/WSS in production
3. Rotate tokens periodically
4. Never commit tokens to version control
5. Use different tokens per environment

---

## Quick Start Examples

### Development Environment

Minimal configuration for local development:

```bash
# .env
DATABASE_URL="sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc"
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST="127.0.0.1"
SERVER_PORT=3000
WEBHOOK_DOMAIN="http://localhost:3000"
WEBHOOK_SECRET_KEY="dev-secret-key"
WEBHOOK_BOT_USERNAME="vibe-repo-bot"
WORKSPACE_BASE_DIR="./data/vibe-repo/workspaces"
```

### Production Environment (SQLite)

Production configuration with SQLite:

```bash
# .env
DATABASE_URL="sqlite:/var/lib/vibe-repo/db/vibe-repo.db?mode=rwc"
DATABASE_MAX_CONNECTIONS=20
SERVER_HOST="0.0.0.0"
SERVER_PORT=3000
WEBHOOK_DOMAIN="https://vibe-repo.example.com"
WEBHOOK_SECRET_KEY="<generate-with-openssl-rand-hex-32>"
WEBHOOK_BOT_USERNAME="vibe-repo-bot"
WEBHOOK_MAX_RETRIES=5
WEBHOOK_INITIAL_DELAY_SECS=60
WEBHOOK_MAX_DELAY_SECS=3600
WEBHOOK_BACKOFF_MULTIPLIER=2.0
WEBHOOK_RETRY_POLLING_FALLBACK_THRESHOLD=5
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=300
ISSUE_POLLING_REQUIRED_LABELS="vibe-auto"
ISSUE_POLLING_BOT_USERNAME="vibe-repo-bot"
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=30
ISSUE_POLLING_MAX_CONCURRENT=10
ISSUE_POLLING_MAX_RETRIES=3
WORKSPACE_BASE_DIR="/var/lib/vibe-repo/workspaces"
WEBSOCKET_AUTH_TOKEN="<generate-with-openssl-rand-hex-32>"
```

### Production Environment (PostgreSQL)

Production configuration with PostgreSQL:

```bash
# .env
DATABASE_URL="postgresql://vibe_user:secure_password@localhost:5432/vibe_repo"
DATABASE_MAX_CONNECTIONS=50
SERVER_HOST="0.0.0.0"
SERVER_PORT=3000
WEBHOOK_DOMAIN="https://vibe-repo.example.com"
WEBHOOK_SECRET_KEY="<generate-with-openssl-rand-hex-32>"
WEBHOOK_BOT_USERNAME="vibe-repo-bot"
WEBHOOK_MAX_RETRIES=5
WEBHOOK_INITIAL_DELAY_SECS=60
WEBHOOK_MAX_DELAY_SECS=3600
WEBHOOK_BACKOFF_MULTIPLIER=2.0
WEBHOOK_RETRY_POLLING_FALLBACK_THRESHOLD=5
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=300
ISSUE_POLLING_REQUIRED_LABELS="vibe-auto"
ISSUE_POLLING_BOT_USERNAME="vibe-repo-bot"
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=30
ISSUE_POLLING_MAX_CONCURRENT=20
ISSUE_POLLING_MAX_RETRIES=3
WORKSPACE_BASE_DIR="/var/lib/vibe-repo/workspaces"
WEBSOCKET_AUTH_TOKEN="<generate-with-openssl-rand-hex-32>"
```

### Docker Compose Environment

Configuration for Docker Compose deployment:

```bash
# .env
DATABASE_URL="postgresql://postgres:postgres@postgres:5432/vibe_repo"
DATABASE_MAX_CONNECTIONS=30
SERVER_HOST="0.0.0.0"
SERVER_PORT=3000
WEBHOOK_DOMAIN="https://vibe-repo.example.com"
WEBHOOK_SECRET_KEY="${WEBHOOK_SECRET_KEY}"
WEBHOOK_BOT_USERNAME="vibe-repo-bot"
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=300
WORKSPACE_BASE_DIR="/app/workspaces"
WEBSOCKET_AUTH_TOKEN="${WEBSOCKET_AUTH_TOKEN}"
```

### Testing Environment

Configuration for running tests:

```bash
# .env.test
DATABASE_URL="sqlite::memory:"
DATABASE_MAX_CONNECTIONS=5
SERVER_HOST="127.0.0.1"
SERVER_PORT=0
WEBHOOK_DOMAIN="http://localhost:3000"
WEBHOOK_SECRET_KEY="test-secret"
WEBHOOK_BOT_USERNAME="test-bot"
WORKSPACE_BASE_DIR="./tmp/test-workspaces"
```

---

## Environment Variable Summary

| Variable | Type | Default | Required |
|----------|------|---------|----------|
| `DATABASE_URL` | String | `sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc` | No |
| `DATABASE_MAX_CONNECTIONS` | Integer | `10` | No |
| `SERVER_HOST` | String | `0.0.0.0` | No |
| `SERVER_PORT` | Integer | `3000` | No |
| `WEBHOOK_DOMAIN` | String | `http://localhost:3000` | No |
| `WEBHOOK_SECRET_KEY` | String | `default-webhook-secret-change-in-production` | Yes (prod) |
| `WEBHOOK_BOT_USERNAME` | String | `vibe-repo-bot` | No |
| `WEBHOOK_MAX_RETRIES` | Integer | `5` | No |
| `WEBHOOK_INITIAL_DELAY_SECS` | Integer | `60` | No |
| `WEBHOOK_MAX_DELAY_SECS` | Integer | `3600` | No |
| `WEBHOOK_BACKOFF_MULTIPLIER` | Float | `2.0` | No |
| `WEBHOOK_RETRY_POLLING_FALLBACK_THRESHOLD` | Integer | `5` | No |
| `ISSUE_POLLING_ENABLED` | Boolean | `false` | No |
| `ISSUE_POLLING_INTERVAL_SECONDS` | Integer | `300` | No |
| `ISSUE_POLLING_REQUIRED_LABELS` | String | `vibe-auto` | No |
| `ISSUE_POLLING_BOT_USERNAME` | String | `vibe-repo-bot` | No |
| `ISSUE_POLLING_MAX_ISSUE_AGE_DAYS` | Integer | `30` | No |
| `ISSUE_POLLING_MAX_CONCURRENT` | Integer | `10` | No |
| `ISSUE_POLLING_MAX_RETRIES` | Integer | `3` | No |
| `WORKSPACE_BASE_DIR` | String | `./data/vibe-repo/workspaces` | No |
| `WEBSOCKET_AUTH_TOKEN` | String | None | Yes (prod) |

---

**Last Updated:** 2026-01-24  
**Version:** 0.3.0
