# Agent Guidelines for VibeRepo

**Current Version:** v0.3.0 (Pre-1.0 - Breaking changes allowed)

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

### Current Status (v0.3.0)

**Completed Modules:**
- ✅ Backend Foundation, RepoProvider API, Repository API
- ✅ Git Provider Abstraction, Repository Initialization
- ✅ Webhook Integration, Workspace API, Init Script Feature
- ✅ Container Lifecycle Management, Agent Management
- ✅ Task Automation, Issue Polling
- ✅ Task Execution Engine, Task Scheduler
- ✅ Concurrency Control, Real-time Log Streaming
- ✅ Execution History Tracking, Intelligent Failure Analysis
- ✅ Complete Issue-to-PR Workflow (PR Creation & Issue Closure)

### Technology Stack

- **Language**: Rust (Edition 2021)
- **Framework**: Axum 0.7 with WebSocket support
- **Async Runtime**: Tokio with full features
- **Database ORM**: SeaORM 1.1 (supports SQLite and PostgreSQL)
- **HTTP Client**: Reqwest 0.11 for Git provider APIs
- **API Documentation**: utoipa 4.x with Swagger UI
- **Testing**: Comprehensive TDD approach with 327 tests

### Module Hierarchy

```
Settings (namespace)
└── RepoProvider (entity)
    └── Repository (entity) [many-to-one]
        ├── WebhookConfig (entity) [one-to-one]
        └── Workspace (entity) [one-to-one]
            ├── InitScript (entity) [one-to-one]
            ├── Agent (entity) [one-to-many]
            └── Task (entity) [one-to-many]
                └── TaskExecution (entity) [one-to-many]
```

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

**Background Services:**
- Implement `BackgroundService` trait
- Register with `ServiceManager`
- Services start automatically with application

**Working with Git Providers:**
```rust
use crate::git_provider::{GitProvider, GitClientFactory};

// Create a client from a RepoProvider entity
let client = GitClientFactory::from_provider(&provider)?;

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

**Test Coverage (v0.3.0):**
- Total tests: 589+
- Passing: 100%
- Unit tests: 375+
- Integration tests: 214+

## 🗄️ Database Schema

For complete database schema documentation, see **[docs/database/schema.md](./docs/database/schema.md)**.

### Implemented Tables (v0.3.0)

- **repo_providers** - Git provider configurations
- **repositories** - Repository records with validation and polling
- **webhook_configs** - Webhook configurations
- **workspaces** - Docker-based development environments
- **init_scripts** - Container initialization scripts
- **agents** - AI agent configurations
- **tasks** - Automated development tasks
- **task_executions** - Task execution history

### Entity Relationships

```
Settings (namespace)
└── RepoProvider (entity)
    └── Repository (entity) [many-to-one]
        ├── WebhookConfig (entity) [one-to-one]
        └── Workspace (entity) [one-to-one]
            ├── InitScript (entity) [one-to-one]
            ├── Agent (entity) [one-to-many]
            └── Task (entity) [one-to-many]
                └── TaskExecution (entity) [one-to-many]
```

## ⚙️ Configuration

For complete configuration guide, see **[docs/development/README.md](./docs/development/README.md#configuration)**.

### Environment Variables

Create a `.env` file in project root:

```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
```

## 📋 Development Standards

For detailed standards, see **[docs/development/README.md](./docs/development/README.md#development-standards)**.

### Language Standards

- **Primary Language**: English for all code, comments, documentation, and commit messages
- **Breaking Changes**: Allowed before v1.0.0 (currently v0.3.0)

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

**Last Updated:** 2026-01-24  
**Version:** 0.3.0
