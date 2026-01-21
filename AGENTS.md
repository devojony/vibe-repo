# Agent Guidelines for VibeRepo

**Current Version:** v0.4.0 (Pre-1.0 - Breaking changes allowed)

This document provides coding guidelines for AI agents working on the VibeRepo codebase.

## 📚 Documentation

For comprehensive documentation, see:
- **[docs/README.md](./docs/README.md)** - Documentation index
- **[docs/api/](./docs/api/)** - API specifications and feature guides
- **[docs/database/schema.md](./docs/database/schema.md)** - Complete database schema
- **[docs/design/](./docs/design/)** - Feature designs and architecture
- **[docs/plans/](./docs/plans/)** - Implementation plans and roadmaps
- **[docs/research/](./docs/research/)** - Technical research and investigations

## Project Overview

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

### Current Status (v0.4.0)

**Completed Modules:**
- ✅ Backend Foundation (configuration, database, error handling, health check)
- ✅ RepoProvider API (Git provider configuration management)
- ✅ Repository API (repository listing and validation)
- ✅ Git Provider Abstraction (unified interface for Gitea/GitHub/GitLab)
- ✅ Repository Initialization (branch and label setup)
- ✅ Static Dispatch Git Client (compile-time polymorphism)
- ✅ Webhook Integration (real-time event processing)
- ✅ Workspace API (Docker-based isolated development environments)
- ✅ Init Script Feature (automated container setup)
- ✅ Container Lifecycle Management (health monitoring, auto-restart)
- ✅ Agent Management (AI agent configurations)
- ✅ Task Automation (automated development tasks)
- ✅ Issue Polling (automatic issue synchronization with intelligent filtering)
- ✅ Task Execution Engine (Docker-based task execution)
- ✅ Task Scheduler (automatic background execution)
- ✅ Concurrency Control (per-workspace limits)
- ✅ Real-time Log Streaming (WebSocket)
- ✅ Execution History Tracking
- ✅ Intelligent Failure Analysis

**In Progress:**
- 🟡 Complete Issue-to-PR Workflow (90% done)

### Technology Stack

- **Language**: Rust (Edition 2021)
- **Framework**: Axum 0.7 with WebSocket support
- **Async Runtime**: Tokio with full features
- **Database ORM**: SeaORM 0.12 (supports SQLite and PostgreSQL)
- **HTTP Client**: Reqwest 0.11 for Git provider APIs
- **API Documentation**: utoipa 4.x with Swagger UI
- **Testing**: Comprehensive TDD approach with unit, integration, and property-based tests
- **Architecture**: Layered design (HTTP → Service → Data)

### Module Hierarchy

The system follows a clear module hierarchy:

**First-Level Modules** (independent top-level resources):
- **Settings**: Namespace for global configuration resources
- **Repository**: Git repository management
- **Workspace**: Development workspace (planned)

**Second-Level Modules** (belong to a parent module):
- **RepoProvider** (under Settings): Git provider configurations
- **Agent** (under Workspace): AI agent configurations (planned)
- **Task** (under Workspace): Automated development tasks (planned)

**Entity Relationships:**
```
Settings (namespace)
└── RepoProvider (entity)
    └── Repository (entity) [many-to-one]
        └── Workspace (entity) [one-to-one] (planned)
            ├── Agent (entity) [one-to-many] (planned)
            └── Task (entity) [one-to-many] (planned)
```

## Build, Lint, and Test Commands

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

## Code Style Guidelines

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

## Project Structure

```
backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library root
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types
│   ├── state.rs             # Application state
│   ├── logging.rs           # Logging setup
│   ├── api/                 # HTTP API layer
│   │   ├── mod.rs           # Router setup
│   │   ├── health/          # Health check module
│   │   │   ├── mod.rs
│   │   │   ├── routes.rs
│   │   │   ├── handlers.rs
│   │   │   └── tests.rs
│   │   ├── settings/        # Settings namespace module
│   │   │   ├── mod.rs
│   │   │   └── providers/   # RepoProvider API (second-level)
│   │   │       ├── mod.rs
│   │   │       ├── routes.rs
│   │   │       ├── handlers.rs
│   │   │       ├── models.rs
│   │   │       └── validation.rs
│   │   └── repositories/    # Repository API (first-level)
│   │       ├── mod.rs
│   │       ├── routes.rs
│   │       ├── handlers.rs
│   │       └── models.rs
│   ├── services/            # Background services
│   │   ├── mod.rs
│   │   ├── service_manager.rs
│   │   ├── repository_service.rs
│   │   └── tests.rs
│   ├── git_provider/        # Git provider abstraction
│   │   ├── mod.rs
│   │   ├── traits.rs        # GitProvider trait
│   │   ├── models.rs        # Unified data models
│   │   ├── error.rs         # Provider-specific errors
│   │   ├── factory.rs       # GitClientFactory
│   │   └── gitea/           # Gitea implementation
│   │       ├── mod.rs
│   │       ├── client.rs
│   │       └── models.rs
│   ├── db/                  # Database connection
│   │   ├── mod.rs
│   │   └── database.rs
│   ├── entities/            # SeaORM entities
│   │   ├── mod.rs
│   │   ├── prelude.rs
│   │   ├── repo_provider.rs
│   │   └── repository.rs
│   ├── migration/           # Database migrations
│   │   ├── mod.rs
│   │   ├── m20240101_000001_init.rs
│   │   ├── m20250114_000001_create_repo_providers.rs
│   │   ├── m20250114_000002_create_repositories.rs
│   │   └── m20250114_000003_add_provider_unique_constraint.rs
│   └── test_utils/          # Test utilities
│       ├── mod.rs
│       ├── db.rs
│       └── state.rs
└── tests/                   # Integration tests
    ├── health_integration_tests.rs
    ├── health_property_tests.rs
    ├── logging_integration_tests.rs
    ├── openapi_integration_tests.rs
    └── server_startup_tests.rs
```

## Implemented API Endpoints

### Health Check
- `GET /health` - Service health status with database connectivity check

### Settings Module

#### RepoProvider (Second-Level Module)
- `GET /api/settings/providers` - List all Git provider configurations
- `POST /api/settings/providers` - Create a new provider configuration
- `GET /api/settings/providers/:id` - Get provider details
- `PUT /api/settings/providers/:id` - Update provider configuration
- `DELETE /api/settings/providers/:id` - Delete provider (if not locked)
- `POST /api/settings/providers/:id/validate` - Validate provider token
- `POST /api/settings/providers/:id/sync` - Manually trigger repository sync

**Features:**
- Unique constraint on (name, base_url, access_token)
- Access token masking in responses (first 8 chars + `***`)
- Locked provider protection (prevents deletion)
- Cascade delete to associated repositories

### Repository Module (First-Level Module)
- `GET /api/repositories` - List repositories with optional filters
  - Query params: `provider_id`, `validation_status`
- `GET /api/repositories/:id` - Get repository details
- `POST /api/repositories/:id/refresh` - Refresh repository validation status
- `POST /api/repositories/:id/initialize` - Initialize single repository
- `POST /api/repositories/batch-initialize` - Batch initialize repositories
- `PATCH /api/repositories/:id/polling` - Update issue polling configuration
- `POST /api/repositories/:id/poll-issues` - Manually trigger issue polling

**Features:**
- Automatic repository sync when provider is created/updated
- Validation checks: required branches (vibe-dev), labels (vibe/* prefix), permissions
- Background service for scheduled sync (hourly)
- Repository initialization with configurable branch names and label management
- Issue polling with intelligent filtering (labels, mentions, state, age)
- Dual-mode issue tracking (webhook-first with automatic polling fallback)

### Webhook Module

#### Webhook Event Receiver
- `POST /api/webhooks/:repository_id` - Receive webhook events from Git providers

**Features:**
- Repository-based webhook URLs for direct lookup
- Signature verification using webhook_secret
- Support for multiple Git providers (Gitea/GitHub/GitLab)
- Automatic webhook creation during repository initialization
- Retry mechanism for failed webhook deliveries

**Webhook URL Format:**
```
https://vibe-repo.example.com/api/webhooks/{repository_id}
```

Where `{repository_id}` is the database ID of the repository. This design:
- Enables direct webhook lookup by repository_id (indexed, fast)
- Makes the webhook-repository association explicit
- Avoids ambiguity about which repository the webhook belongs to
- Allows signature verification without additional queries

**Webhook-Repository-Provider Relationship:**
```
Provider (1) ──→ (N) Repository (1) ──→ (1) WebhookConfig
                      ↑                        ↓
                      └────── provider_id ─────┘
                           (redundant for optimization)
```

- **Primary Association**: WebhookConfig → Repository (one-to-one)
  - Each repository has at most one webhook
  - Webhook URL uses repository_id
- **Secondary Association**: WebhookConfig → Provider (many-to-one)
  - provider_id is redundant but kept for performance
  - Enables cascade delete and fast provider-level queries

### API Documentation
- `GET /swagger-ui` - Interactive API documentation (Swagger UI)
- `GET /api-docs/openapi.json` - OpenAPI 3.0 specification

## Environment Configuration

Configuration is loaded from `.env` file in project root:

```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
LOG_FORMAT=json  # Optional: use JSON logs in production

# Issue Polling Configuration
ISSUE_POLLING_ENABLED=true
ISSUE_POLLING_INTERVAL_SECONDS=300
ISSUE_POLLING_BATCH_SIZE=10
ISSUE_POLLING_MAX_ISSUE_AGE_DAYS=30
```

## Common Patterns

For detailed development patterns, see **[docs/development/README.md](./docs/development/README.md)**.

### Quick Reference

**Creating New API Endpoints:**
1. Define models in `models.rs` with `#[derive(Serialize, Deserialize, ToSchema)]`
2. Create handlers in `handlers.rs` with OpenAPI docs
3. Define routes in `routes.rs`
4. Register router in `api/mod.rs`
5. Write integration tests in `tests/`

**Database Operations:**
- Use SeaORM for new code
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

## Development Standards

### Language Standards

- **Primary Language**: English for all code, comments, documentation, and commit messages
- **Code Comments**: Use English for complex logic, module docs, and function descriptions
- **Test Cases**: Test descriptions and assertion messages in English
- **Breaking Changes**: Allowed before v1.0.0 (currently v0.1.20)

### Commit Message Standards

Follow Conventional Commits specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code formatting (no functional impact)
- `refactor`: Refactoring (neither new feature nor bug fix)
- `perf`: Performance optimization
- `test`: Add or modify tests
- `chore`: Build process or auxiliary tool changes

**Scopes:**
- `api`: Backend API related
- `db`: Database related
- `deps`: Dependency updates
- `test`: Test related
- `docs`: Documentation related

**Examples:**
```bash
feat(api): Add repository initialization feature

test(api): Add credential API integration tests

fix(db): Fix unique constraint validation logic
```

### TDD Workflow

This project strictly follows Test-Driven Development:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Refactor code while keeping tests passing

**TDD Commands:**
```bash
# 1. Write test first (should fail)
cargo test test_name -- --nocapture

# 2. Implement feature

# 3. Run test (should pass)
cargo test test_name -- --nocapture

# 4. Refactor and ensure all tests pass
cargo test
```

**Test Types:**
- **Unit Tests**: In `#[cfg(test)] mod tests` at bottom of source files
- **Integration Tests**: In `tests/` directory with `_integration_tests.rs` suffix
- **Property Tests**: Use `proptest` crate, suffix with `_property_tests.rs`

**Test Coverage (v0.4.0):**
- Total tests: 327
- Passing: 100%
- Unit tests: 310 (including scheduler, executor, analyzer tests)
- Integration tests: 17
- Test categories:
  - Task management: 50+ tests
  - Execution engine: 10+ tests
  - Failure analysis: 4 tests
  - Scheduler: 7 tests
  - Concurrency control: 6 tests
  - WebSocket logs: 4 tests

## Database Schema

For complete database schema documentation, see **[docs/database/schema.md](./docs/database/schema.md)**.

### Implemented Tables (v0.4.0)

- **repo_providers** - Git provider configurations with authentication credentials
- **repositories** - Repository records with validation status and polling configuration
- **webhook_configs** - Webhook configurations for repository event monitoring
- **workspaces** - Docker-based isolated development environments
- **init_scripts** - Custom initialization scripts for workspace containers
- **agents** - AI agent configurations for workspaces
- **tasks** - Automated development tasks created from issues
- **task_executions** - Complete history of task execution attempts

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

For detailed field descriptions, constraints, and relationships, see [docs/database/schema.md](./docs/database/schema.md).

## Additional Notes

- **Logging**: Use `tracing` macros (`tracing::info!`, `tracing::error!`, etc.)
- **CORS**: Configured as permissive for development
- **OpenAPI**: Access Swagger UI at `http://localhost:3000/swagger-ui`
- **Health Check**: Available at `http://localhost:3000/health`
- **Graceful Shutdown**: Ctrl+C triggers graceful shutdown of services
- **Database**: SQLite for development, PostgreSQL for production
- **Migrations**: Run automatically on application startup
- **Background Services**: Repository sync service runs hourly, issue polling service runs every 5 minutes (configurable)
- **Token Security**: Access tokens and API keys are masked in all API responses
- **Version Policy**: Pre-1.0 allows breaking changes without migration
