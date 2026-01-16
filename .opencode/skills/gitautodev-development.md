# GitAutoDev Development Skill

Comprehensive development workflow for the GitAutoDev automated programming assistant project.

## Skill Type

**Rigid** - Follow the workflow exactly, especially TDD discipline.

## When to Use This Skill

Use this skill when working on any GitAutoDev development tasks, including:
- Adding new API endpoints
- Implementing new features
- Refactoring code
- Writing tests
- Database changes
- Git provider integrations

## Project Context

**Current Version:** v0.1.20 (Pre-1.0 - Breaking changes allowed)

**Technology Stack:**
- Rust 2021 with Axum 0.7
- SeaORM 0.12 (SQLite/PostgreSQL)
- Tokio async runtime
- Comprehensive testing (unit, integration, property-based)
- TDD methodology

## Development Workflow

### Phase 1: Understanding Requirements

1. **Read AGENTS.md** for project context
2. **Explore codebase** to understand existing patterns
3. **Identify module hierarchy** (first-level vs second-level modules)
4. **Check API documentation** at http://localhost:3000/swagger-ui

### Phase 2: Test-Driven Development (Mandatory)

**CRITICAL: Always follow TDD - Red → Green → Refactor**

#### Step 1: Write Failing Test (RED)

```bash
# Write test first - it should fail
cargo test test_name -- --nocapture
```

**Test Types:**
- **Unit tests**: In `#[cfg(test)] mod tests` at bottom of source files
- **Integration tests**: In `tests/` directory with `_integration_tests.rs` suffix
- **Property tests**: Use `proptest` crate, suffix with `_property_tests.rs`

**Test Naming:**
- Prefix with `test_` for unit/integration tests
- Prefix with `prop_` for property-based tests
- Use descriptive names: `test_health_endpoint_returns_200_when_healthy`

#### Step 2: Implement Minimal Code (GREEN)

Write only the minimal code needed to make the test pass.

#### Step 3: Refactor (Refactor)

Improve code while keeping all tests passing:
```bash
cargo test  # Ensure all tests still pass
```

### Phase 3: Implementation Checklist

**For New API Endpoints:**

1. ✓ Define models in `models.rs` with `#[derive(Serialize, Deserialize, ToSchema)]`
2. ✓ Create handlers in `handlers.rs` with `#[utoipa::path]` documentation
3. ✓ Define routes in `routes.rs`
4. ✓ Register router in parent `mod.rs`
5. ✓ Write integration tests in `tests/`
6. ✓ Run all tests: `cargo test`
7. ✓ Run clippy: `cargo clippy`
8. ✓ Format code: `cargo fmt`

**For Database Changes:**

1. ✓ Create new migration file in `src/migration/`
2. ✓ Generate SeaORM entity: `sea-orm-cli generate entity`
3. ✓ Add entity to `entities/mod.rs`
4. ✓ Update `entities/prelude.rs`
5. ✓ Test migration runs automatically
6. ✓ Write database integration tests

**For Git Provider Integration:**

1. ✓ Implement `GitProvider` trait from `git_provider/traits.rs`
2. ✓ Create provider-specific client
3. ✓ Add factory support in `git_provider/factory.rs`
4. ✓ Write provider integration tests
5. ✓ Test with real Git instance (if available)

## Code Style Guidelines

### Module Documentation
Every module must have a top-level doc comment:
```rust
//! Configuration management module
//!
//! Loads configuration from environment variables with sensible defaults.
```

### Imports Organization
Order: `std::` → external crates (alphabetical) → `crate::` → relative imports

```rust
use std::sync::Arc;

use anyhow::Result;
use axum::Router;

use crate::config::AppConfig;
```

### Struct Definitions
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,
}
```

### Error Handling
Use `GitAutoDevError` enum from `src/error.rs`:
- Return `Result<T, GitAutoDevError>`
- Map to appropriate HTTP status codes
- Use `IntoResponse` trait for HTTP responses

### Async/Await
- Use `#[tokio::main]` for main function
- Use `#[tokio::test]` for async tests
- Use `#[async_trait]` for async trait methods

## Testing Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run single integration test file
cargo test --test health_integration_tests
```

## Code Quality Commands

```bash
# Check for warnings and style issues
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --check
```

## Module Hierarchy Rules

**First-Level Modules** (independent top-level resources):
- Settings (namespace)
- Repository
- Workspace (planned)

**Second-Level Modules** (belong to a parent module):
- RepoProvider (under Settings)
- Agent (under Workspace, planned)
- Task (under Workspace, planned)

When adding new functionality, determine if it's first-level or second-level based on this hierarchy.

## Commit Message Standards

Follow Conventional Commits:
```
<type>(<scope>): <description>
```

**Types:** feat, fix, docs, style, refactor, perf, test, chore
**Scopes:** api, db, deps, test, docs

Examples:
- `feat(api): Add repository initialization feature`
- `test(api): Add credential API integration tests`
- `fix(db): Fix unique constraint validation logic`

## Common Patterns

### Creating a Handler
```rust
#[utoipa::path(
    get,
    path = "/endpoint",
    responses(
        (status = 200, description = "Success", body = ResponseModel),
    )
)]
pub async fn handler_name(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ResponseModel>, GitAutoDevError> {
    // Thin handler - delegate to service layer
}
```

### Database Operations
```rust
use crate::db::DatabasePool;
use sea_orm::{EntityTrait, QueryFilter};

let pool = state.db_pool.connection();
let items = Entity::find().filter(condition).all(pool).await?;
```

### Git Provider Usage
```rust
use crate::git_provider::{GitProvider, GitClientFactory};

let client = GitClientFactory::from_provider(&provider)?;
let repos = client.list_repositories(None).await?;
```

## Troubleshooting

### Tests Failing
1. Check database connection - migrations run automatically
2. Ensure test data is properly cleaned up
3. Verify async operations are properly awaited

### Compilation Errors
1. Check imports are in correct order
2. Ensure all derives are correct (Debug, Clone, Serialize, Deserialize, ToSchema)
3. Verify async traits use `#[async_trait]`

### Database Issues
1. Check `.env` file has correct `DATABASE_URL`
2. Ensure migrations ran successfully on startup
3. Verify entity definitions match database schema

## Quality Gates

Before considering a feature complete:
- ✓ All tests passing (`cargo test`)
- ✓ No clippy warnings (`cargo clippy`)
- ✓ Code formatted (`cargo fmt`)
- ✓ Integration tests pass
- ✓ Property tests pass (if applicable)
- ✓ API docs generated correctly
- ✓ Manual testing at http://localhost:3000/swagger-ui

## Current Status (v0.1.20)

**Completed:**
- ✅ Backend foundation (config, database, error handling, health check)
- ✅ RepoProvider API
- ✅ Repository API
- ✅ Git Provider Abstraction (Gitea)
- ✅ Repository Initialization
- ✅ Static Dispatch Git Client

**In Progress:**
- 🟡 Workspace API (planned next)

## Notes

- All code, comments, and documentation must be in English
- Breaking changes allowed before v1.0.0
- Access tokens masked in API responses (first 8 chars + `***`)
- Background services run automatically
- Graceful shutdown on Ctrl+C
- SQLite for development, PostgreSQL for production
