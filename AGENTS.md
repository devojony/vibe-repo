# Agent Guidelines for VibeRepo

**Current Version:** v0.4.0-mvp (Simplified MVP - Pre-1.0)

> **🎯 Simplified MVP**: This version focuses on core Issue-to-PR automation with a streamlined architecture. Many advanced features have been removed to create a solid foundation for future development.

This document provides quick reference guidelines for AI agents working on the VibeRepo codebase.

## 📚 Documentation

For comprehensive documentation, see:
- **[docs/README.md](./docs/README.md)** - Documentation index
- **[docs/development/](./docs/development/)** - Complete development guide
- **[docs/api/](./docs/api/)** - API specifications and feature guides
- **[docs/database/schema.md](./docs/database/schema.md)** - Complete database schema
- **[docs/roadmap/](./docs/roadmap/)** - Project roadmap and milestones

## 🎯 Project Overview

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

### Current Status (v0.4.0-mvp - Simplified MVP)

**Core Features:**
- ✅ Backend Foundation & Repository API
- ✅ Git Provider Abstraction (GitHub/Gitea/GitLab)
- ✅ Webhook Integration (GitHub)
- ✅ Docker-based Workspaces
- ✅ Single Agent per Repository
- ✅ Task Management (Simplified State Machine)
- ✅ Task Execution Engine
- ✅ PR Creation & Issue Closure
- ✅ Environment-based Configuration

**Removed Features (from v0.3.0):**
- ❌ Issue Polling Service
- ❌ Webhook Retry Service
- ❌ Init Script Service
- ❌ WebSocket Real-time Logs
- ❌ Task Failure Analyzer
- ❌ Task Execution History
- ❌ Health Check Service
- ❌ Image Management Service
- ❌ Provider Management API
- ❌ Workspace Management API
- ❌ Agent Management API
- ❌ Webhook Config API
- ❌ Task Retry Mechanism
- ❌ Assigned State (tasks go directly from Pending to Running)

### Technology Stack

- **Language**: Rust (Edition 2021)
- **Framework**: Axum 0.7 (WebSocket support removed)
- **Async Runtime**: Tokio with full features
- **Database ORM**: SeaORM 1.1 (supports SQLite and PostgreSQL)
- **HTTP Client**: Reqwest 0.11 for Git provider APIs
- **API Documentation**: utoipa 4.x with Swagger UI
- **Testing**: Comprehensive TDD approach with 280+ unit tests

### Simplified Architecture

```
Repository (entity) [self-contained with provider config]
└── Workspace (entity) [one-to-one]
    ├── Agent (entity) [one-to-one, unique constraint]
    └── Task (entity) [one-to-many]
```

**Key Simplifications:**
- No separate Provider entity (configuration stored in repository)
- No WebhookConfig entity (webhook_secret stored in repository)
- No InitScript entity (workspaces use default setup)
- No TaskExecution entity (logs stored in tasks.last_log field)
- Single agent per workspace (enforced by unique constraint)

## 🛠️ Build, Lint, and Test Commands

For comprehensive development guidelines, see **[docs/development/README.md](./docs/development/README.md)**.

### Quick Reference

**Building:**
```bash
cargo build              # Build the project
cargo build --release    # Build in release mode
cargo run                # Run the application
```

**Testing:**
```bash
cargo test                    # Run all tests (327 tests)
cargo test test_name          # Run specific test
cargo test -- --nocapture     # Run with output visible
cargo test --lib              # Run only unit tests
cargo test --test '*'         # Run only integration tests
```

**Code Quality:**
```bash
cargo clippy             # Check for warnings and style issues
cargo fmt                # Format code
cargo fmt --check        # Check formatting without modifying
```

## 📝 Code Style Guidelines

For detailed code style guidelines, see **[docs/development/README.md](./docs/development/README.md#code-style-guidelines)**.

### Quick Reference

**Naming Conventions:**
- Modules: `snake_case`
- Structs/Enums: `PascalCase`
- Functions/Variables: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`

**Imports Organization:**
1. Standard library (`std::`)
2. External crates (alphabetically)
3. Internal crate modules (`crate::`)
4. Relative imports (`super::`, `self::`)

**Module Documentation:**
```rust
//! Module description
//!
//! Detailed explanation of the module's purpose.
```

## 🏗️ Project Structure

For complete project structure, see **[docs/development/README.md](./docs/development/README.md#project-structure)**.

```
backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library root
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types
│   ├── state.rs             # Application state
│   ├── api/                 # HTTP API layer
│   ├── services/            # Background services
│   ├── git_provider/        # Git provider abstraction
│   ├── db/                  # Database connection
│   ├── entities/            # SeaORM entities
│   ├── migration/           # Database migrations
│   └── test_utils/          # Test utilities
└── tests/                   # Integration tests
```

## 🔧 Common Patterns

For detailed development patterns, see **[docs/development/README.md](./docs/development/README.md#common-patterns)**.

### Quick Reference

**Creating New API Endpoints:**
1. Define models in `models.rs` with `#[derive(Serialize, Deserialize, ToSchema)]`
2. Create handlers in `handlers.rs` with OpenAPI docs
3. Define routes in `routes.rs`
4. Register router in `api/mod.rs`
5. Write integration tests in `tests/`

**Database Operations:**
- Use SeaORM for all database operations
- Migrations run automatically on startup
- Use `DatabasePool::new()` to create connection pool
- **Pre-1.0 Migration Policy**: Breaking schema changes are allowed before v1.0.0 release

**Background Services:**
- Implement `BackgroundService` trait
- Register with `ServiceManager`
- Services start automatically with application

**Working with Git Providers:**
```rust
use crate::git_provider::{GitProvider, GitClientFactory};

// Create a client from a Repository entity
let client = GitClientFactory::from_repository(&repository)?;

// Use the unified interface
let repos = client.list_repositories(None).await?;
let branches = client.list_branches("owner", "repo").await?;
```

## 🧪 Testing Philosophy

For detailed testing guidelines, see **[docs/tests/README.md](./docs/tests/README.md)**.

### TDD Workflow

This project strictly follows Test-Driven Development:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Refactor code while keeping tests passing

**Test Coverage (v0.4.0-mvp):**
- Total tests: 280+ (unit tests)
- Passing: 100%
- Focus: Core functionality and API endpoints

## 🗄️ Database Schema

For complete database schema documentation, see **[docs/database/schema.md](./docs/database/schema.md)**.

### Simplified Tables (v0.4.0-mvp)

- **repositories** - Repository records with self-contained provider configuration
- **workspaces** - Docker-based development environments
- **agents** - AI agent configurations (one per workspace)
- **tasks** - Automated development tasks with inline logs

**Removed Tables:**
- ~~repo_providers~~ (configuration stored in repository)
- ~~webhook_configs~~ (webhook_secret stored in repository)
- ~~init_scripts~~ (workspaces use default setup)
- ~~task_executions~~ (logs stored in tasks.last_log)

### Entity Relationships

```
Repository (entity) [self-contained with provider config]
└── Workspace (entity) [one-to-one]
    ├── Agent (entity) [one-to-one, unique constraint]
    └── Task (entity) [one-to-many]
```

## ⚙️ Configuration

For complete configuration guide, see **[docs/development/README.md](./docs/development/README.md#configuration)**.

### Repository Configuration

Each repository is self-contained with its own provider configuration. Add repositories via the API:

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

### Environment Variables

Create a `.env` file in project root:

```bash
# Database Configuration
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10

# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

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

**Note:** Git provider configuration (tokens, base URLs) is now stored per-repository in the database, not in environment variables.

## 📋 Development Standards

For detailed standards, see **[docs/development/README.md](./docs/development/README.md#development-standards)**.

### Language Standards

- **Primary Language**: English for all code, comments, documentation, and commit messages
- **Breaking Changes**: Allowed before v1.0.0 (currently v0.3.0)

### Experimentation Standards

- **Experimentation Directory**: All feature verification and experimental code must be placed in the `tmp/` directory at project root
- The `tmp/` directory is git-ignored and safe for temporary experiments
- Never commit experimental code outside of `tmp/` directory

### Git Worktree Standards

- **Worktree Directory**: All git worktrees must be created in the `.worktrees/` directory at project root
- The `.worktrees/` directory is git-ignored and safe for parallel development
- Use worktrees for working on multiple branches simultaneously without switching contexts

### Commit Message Standards

Follow Conventional Commits specification:

```
<type>(<scope>): <description>

Examples:
feat(api): add task scheduler service
fix(db): fix unique constraint validation
test(api): add task execution integration tests
docs: update development guide
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`  
**Scopes:** `api`, `db`, `deps`, `test`, `docs`

### TDD Commands

```bash
# 1. Write test first (should fail)
cargo test test_name -- --nocapture

# 2. Implement feature

# 3. Run test (should pass)
cargo test test_name -- --nocapture

# 4. Refactor and ensure all tests pass
cargo test
```

## 📚 Additional Resources

- **[User Guide](./docs/api/user-guide.md)** - Complete usage guide
- **[API Reference](./docs/api/api-reference.md)** - All API endpoints
- **[Roadmap](./docs/roadmap/README.md)** - Project roadmap and milestones
- **[Design Documents](./docs/design/)** - Feature designs and architecture
- **[Implementation Plans](./docs/plans/)** - Detailed implementation plans
- **[Research](./docs/research/)** - Technical research and investigations

## 🆘 Getting Help

- **Documentation**: See [docs/README.md](./docs/README.md)
- **API Docs**: Access Swagger UI at `http://localhost:3000/swagger-ui`
- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Ask questions in GitHub Discussions

---

**Note:** This document serves as a quick reference for AI agents. For comprehensive guidelines, always refer to the detailed documentation in the `docs/` directory.

**Last Updated:** 2026-02-06  
**Version:** 0.4.0-mvp (Simplified MVP)
