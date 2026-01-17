# Agent Guidelines for VibeRepo

**Current Version:** v0.1.20 (Pre-1.0 - Breaking changes allowed)

This document provides coding guidelines for AI agents working on the VibeRepo codebase.

## Project Overview

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

### Current Status (v0.1.20)

**Completed Modules:**
- ✅ Backend Foundation (configuration, database, error handling, health check)
- ✅ RepoProvider API (Git provider configuration management)
- ✅ Repository API (repository listing and validation)
- ✅ Git Provider Abstraction (unified interface for Gitea/GitHub/GitLab)
- ✅ Repository Initialization (branch and label setup)
- ✅ Static Dispatch Git Client (compile-time polymorphism)

**In Progress:**
- 🟡 Workspace API (planned next)

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

### Building
```bash
# Build the project (from project root or backend/)
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test by name
cargo test test_name

# Run tests in a specific file/module
cargo test config
cargo test health

# Run with output visible
cargo test -- --nocapture

# Run only unit tests (in src/)
cargo test --lib

# Run only integration tests (in tests/)
cargo test --test '*'

# Run a single integration test file
cargo test --test health_integration_tests
```

### Code Quality
```bash
# Check for warnings and style issues
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying files
cargo fmt --check
```

## Code Style Guidelines

### Module Documentation
- Every module must have a top-level doc comment (`//!`) describing its purpose
- Example:
  ```rust
  //! Configuration management module
  //!
  //! Loads configuration from environment variables with sensible defaults.
  ```

### Imports Organization
Organize imports in this order:
1. Standard library (`std::`)
2. External crates (alphabetically)
3. Internal crate modules (`crate::`)
4. Relative imports (`super::`, `self::`)

Example:
```rust
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::state::AppState;
```

### Type Definitions

#### Structs
- Use `#[derive(Debug, Clone)]` for most structs
- Add `Serialize, Deserialize` for API models
- Document public fields with doc comments

```rust
/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
}
```

#### Enums
- Use `#[derive(Debug, Clone)]` at minimum
- Add `thiserror::Error` for error types
- Document variants with doc comments

```rust
#[derive(Debug, thiserror::Error)]
pub enum VibeRepoError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
}
```

### Error Handling

#### Error Types
- Use the unified `VibeRepoError` enum for application errors
- Use `Result<T>` type alias: `pub type Result<T> = std::result::Result<T, VibeRepoError>;`
- Implement `IntoResponse` for custom error types to convert to HTTP responses
- Map errors to appropriate HTTP status codes:
  - `NotFound` → 404
  - `Validation` → 400
  - `Conflict` → 409
  - `Forbidden` → 403
  - `ServiceUnavailable` → 503
  - `Database`, `Config`, `Internal` → 500

#### Error Responses
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
```

### Naming Conventions

- **Modules**: `snake_case` (e.g., `git_provider`, `repository_service`)
- **Structs/Enums**: `PascalCase` (e.g., `AppConfig`, `VibeRepoError`)
- **Functions/Variables**: `snake_case` (e.g., `create_router`, `db_pool`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_CONNECTIONS`)
- **Type Aliases**: `PascalCase` (e.g., `Result`)

### Async/Await
- Use `#[tokio::main]` for main function
- Use `#[tokio::test]` for async tests
- Use `async_trait` for async trait methods
- Always use `.await` for async operations

```rust
#[async_trait]
pub trait GitProvider: Send + Sync {
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError>;
}
```

### Testing Philosophy

This project follows **Test-Driven Development (TDD)**:
1. Write failing test first
2. Implement minimal code to pass
3. Refactor while keeping tests green

#### Test Structure
- **Unit tests**: In `#[cfg(test)] mod tests` at bottom of source files
- **Integration tests**: In `tests/` directory with `_integration_tests.rs` suffix
- **Property tests**: Use `proptest` crate, suffix with `_property_tests.rs`

#### Test Naming
- Prefix with `test_` for unit/integration tests
- Prefix with `prop_` for property-based tests
- Use descriptive names: `test_health_endpoint_returns_200_when_healthy`

#### Test Documentation
```rust
/// Test GET /health returns 200 when healthy
/// Requirements: 7.1, 7.2
#[tokio::test]
async fn test_health_endpoint_returns_200_when_healthy() {
    // Arrange: Create test application
    let app = create_test_app().await.expect("Failed to create test app");
    
    // Act: Send GET request
    let response = app.oneshot(request).await.unwrap();
    
    // Assert: Verify response
    assert_eq!(response.status(), StatusCode::OK);
}
```

### API Handlers

- Use Axum extractors for request data
- Return `Result<Json<T>, VibeRepoError>` or `impl IntoResponse`
- Add OpenAPI documentation with `#[utoipa::path]`
- Keep handlers thin - delegate to service layer

```rust
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    )
)]
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, VibeRepoError> {
    // Implementation
}
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

**Features:**
- Automatic repository sync when provider is created/updated
- Validation checks: required branches (vibe-dev), labels (vibe/* prefix), permissions
- Background service for scheduled sync (hourly)
- Repository initialization with configurable branch names and label management

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
```

## Common Patterns

### Creating New API Endpoints
1. Define models in `models.rs` with `#[derive(Serialize, Deserialize, ToSchema)]`
2. Create handlers in `handlers.rs` with OpenAPI docs
3. Define routes in `routes.rs`
4. Register router in `api/mod.rs`
5. Write integration tests in `tests/`

### Database Operations
- Use SeaORM for new code (legacy SQLx code exists)
- Migrations run automatically on startup
- Use `DatabasePool::new()` to create connection pool
- Access connection via `db_pool.connection()`

### Background Services
- Implement `BackgroundService` trait
- Register with `ServiceManager`
- Services start automatically with application

### Working with Git Providers

The Git Provider abstraction provides a unified interface for different Git platforms:

```rust
use crate::git_provider::{GitProvider, GitClientFactory};

// Create a client from a RepoProvider entity
let client = GitClientFactory::from_provider(&provider)?;

// Use the unified interface
let repos = client.list_repositories(None).await?;
let branches = client.list_branches("owner", "repo").await?;
let issues = client.list_issues("owner", "repo", None).await?;
```

**Supported Operations:**
- Repository operations (list, get)
- Branch operations (list, get, create, delete)
- Issue operations (list, get, create, update, add/remove labels)
- Pull request operations (list, get, create, update, merge)
- Label operations (list, create, delete)

**Current Implementations:**
- ✅ Gitea (full support)
- 🔄 GitHub (placeholder)
- 🔄 GitLab (placeholder)

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

**Test Coverage (v0.1.20):**
- Total tests: 181+
- Passing: 100%
- Property tests: 14
- Integration tests: 12+
- Unit tests: 155+

## Database Schema

### Implemented Tables (v0.1.20)

#### repo_providers
Git provider configurations with authentication credentials.

**Fields:**
- `id` (INTEGER, PRIMARY KEY)
- `name` (TEXT, NOT NULL)
- `type` (TEXT, NOT NULL) - Currently only 'gitea' supported
- `base_url` (TEXT, NOT NULL)
- `access_token` (TEXT, NOT NULL) - Masked in API responses
- `locked` (BOOLEAN, DEFAULT false) - Prevents deletion when true
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

**Constraints:**
- UNIQUE (name, base_url, access_token)

#### repositories
Repository records with validation status.

**Fields:**
- `id` (INTEGER, PRIMARY KEY)
- `provider_id` (INTEGER, FOREIGN KEY → repo_providers.id)
- `name` (TEXT, NOT NULL)
- `full_name` (TEXT, NOT NULL)
- `clone_url` (TEXT, NOT NULL)
- `default_branch` (TEXT, NOT NULL)
- `branches` (JSON) - Array of branch names
- `validation_status` (TEXT) - 'valid', 'invalid', 'pending'
- `validation_message` (TEXT, NULLABLE)
- `has_required_branches` (BOOLEAN) - vibe-dev branch exists
- `has_required_labels` (BOOLEAN) - vibe/* labels exist
- `can_manage_prs` (BOOLEAN) - Token has PR permissions
- `can_manage_issues` (BOOLEAN) - Token has Issue permissions
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)

**Relationships:**
- CASCADE DELETE: Deleting a provider deletes all its repositories

#### webhook_configs
Webhook configurations for repository event monitoring.

**Fields:**
- `id` (INTEGER, PRIMARY KEY)
- `provider_id` (INTEGER, FOREIGN KEY → repo_providers.id)
- `repository_id` (INTEGER, FOREIGN KEY → repositories.id)
- `webhook_id` (TEXT, NOT NULL) - Provider's webhook ID
- `webhook_secret` (TEXT, NOT NULL) - Secret for signature verification
- `webhook_url` (TEXT, NOT NULL) - Full webhook URL
- `events` (TEXT, NOT NULL) - JSON array of subscribed events
- `enabled` (BOOLEAN, DEFAULT true)
- `created_at` (TIMESTAMP)
- `updated_at` (TIMESTAMP)
- `retry_count` (INTEGER, DEFAULT 0)
- `last_retry_at` (TIMESTAMP, NULLABLE)
- `next_retry_at` (TIMESTAMP, NULLABLE)
- `last_error` (TEXT, NULLABLE)

**Relationships:**
- **Primary**: webhook_config → repository (one-to-one)
  - Each repository has at most one webhook configuration
  - Webhook URL format: `/api/webhooks/{repository_id}`
- **Secondary**: webhook_config → provider (many-to-one, redundant)
  - provider_id is redundant but kept for performance optimization
  - Enables cascade delete when provider is removed
  - Allows fast queries without JOIN operations

**Constraints:**
- UNIQUE (repository_id) - One webhook per repository
- CASCADE DELETE: Deleting a repository deletes its webhook config
- CASCADE DELETE: Deleting a provider deletes all its webhook configs

**Design Rationale:**
Webhooks are per-repository in Git providers (Gitea/GitHub/GitLab), not per-provider.
The webhook URL uses `repository_id` to make this association explicit and enable
direct lookup without database queries. While `provider_id` is technically redundant
(can be obtained via `repository.provider_id`), it provides significant performance
benefits for common operations like cascade deletion and provider-level queries.

### Planned Tables

- `workspaces` - Development workspace records
- `agents` - AI agent configurations
- `tasks` - Automated task records
- `task_logs` - Task execution logs

## Additional Notes

- **Logging**: Use `tracing` macros (`tracing::info!`, `tracing::error!`, etc.)
- **CORS**: Configured as permissive for development
- **OpenAPI**: Access Swagger UI at `http://localhost:3000/swagger-ui`
- **Health Check**: Available at `http://localhost:3000/health`
- **Graceful Shutdown**: Ctrl+C triggers graceful shutdown of services
- **Database**: SQLite for development, PostgreSQL for production
- **Migrations**: Run automatically on application startup
- **Background Services**: Repository sync service runs hourly
- **Token Security**: Access tokens and API keys are masked in all API responses
- **Version Policy**: Pre-1.0 allows breaking changes without migration
