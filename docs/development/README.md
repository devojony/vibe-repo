# Development Guide

**Version:** 0.3.0  
**Last Updated:** 2026-01-21

This guide provides comprehensive development guidelines for contributing to VibeRepo.

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- SQLite 3 or PostgreSQL
- Docker (for workspace features)
- Git provider account (Gitea/GitHub/GitLab)

### Setup Development Environment

1. **Clone the repository:**
```bash
git clone https://github.com/yourusername/vibe-repo.git
cd vibe-repo/backend
```

2. **Create `.env` file:**
```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
```

3. **Build and run:**
```bash
cargo build
cargo run
```

4. **Verify installation:**
```bash
curl http://localhost:3000/health
```

## 🏗️ Project Structure

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
│   │   ├── settings/        # Settings namespace
│   │   │   └── providers/   # RepoProvider API
│   │   ├── repositories/    # Repository API
│   │   ├── webhooks/        # Webhook receiver
│   │   ├── workspaces/      # Workspace API
│   │   └── tasks/           # Task API
│   ├── services/            # Background services
│   │   ├── mod.rs
│   │   ├── service_manager.rs
│   │   ├── repository_service.rs
│   │   ├── issue_polling_service.rs
│   │   └── task_scheduler_service.rs
│   ├── git_provider/        # Git provider abstraction
│   │   ├── mod.rs
│   │   ├── traits.rs        # GitProvider trait
│   │   ├── models.rs        # Unified data models
│   │   ├── error.rs         # Provider-specific errors
│   │   ├── factory.rs       # GitClientFactory
│   │   └── gitea/           # Gitea implementation
│   ├── db/                  # Database connection
│   │   ├── mod.rs
│   │   └── database.rs
│   ├── entities/            # SeaORM entities
│   │   ├── mod.rs
│   │   ├── prelude.rs
│   │   ├── repo_provider.rs
│   │   ├── repository.rs
│   │   ├── webhook_config.rs
│   │   ├── workspace.rs
│   │   ├── init_script.rs
│   │   ├── agent.rs
│   │   ├── task.rs
│   │   └── task_execution.rs
│   ├── migration/           # Database migrations
│   │   ├── mod.rs
│   │   └── m*.rs            # Migration files
│   └── test_utils/          # Test utilities
│       ├── mod.rs
│       ├── db.rs
│       └── state.rs
└── tests/                   # Integration tests
    ├── health_integration_tests.rs
    ├── repository_integration_tests.rs
    └── task_integration_tests.rs
```

## 🛠️ Build Commands

### Building

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run

# Run in release mode
cargo run --release
```

### Testing

```bash
# Run all tests (327 tests)
cargo test

# Run specific test
cargo test test_name

# Run tests in a specific module
cargo test config
cargo test health

# Run with output visible
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run a single integration test file
cargo test --test health_integration_tests
```

### Code Quality

```bash
# Check for warnings and style issues
cargo clippy

# Check with all features
cargo clippy --all-features

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --check
```

## 📝 Code Style Guidelines

### Module Documentation

Every module must have a top-level doc comment (`//!`) describing its purpose:

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

```rust
#[derive(Debug, thiserror::Error)]
pub enum VibeRepoError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
}
```

### Naming Conventions

- **Modules**: `snake_case` (e.g., `git_provider`, `repository_service`)
- **Structs/Enums**: `PascalCase` (e.g., `AppConfig`, `VibeRepoError`)
- **Functions/Variables**: `snake_case` (e.g., `create_router`, `db_pool`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_CONNECTIONS`)
- **Type Aliases**: `PascalCase` (e.g., `Result`)

### Error Handling

Use the unified `VibeRepoError` enum for application errors:

```rust
pub type Result<T> = std::result::Result<T, VibeRepoError>;

// Map errors to appropriate HTTP status codes
impl IntoResponse for VibeRepoError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            VibeRepoError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            VibeRepoError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            VibeRepoError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            // ... other mappings
        };
        // Return JSON error response
    }
}
```

### Async/Await

```rust
#[async_trait]
pub trait GitProvider: Send + Sync {
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError>;
}
```

## 🧪 Testing Philosophy

### Test-Driven Development (TDD)

VibeRepo strictly follows TDD:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Refactor code while keeping tests passing

### Test Structure

- **Unit tests**: In `#[cfg(test)] mod tests` at bottom of source files
- **Integration tests**: In `tests/` directory with `_integration_tests.rs` suffix
- **Property tests**: Use `proptest` crate, suffix with `_property_tests.rs`

### Test Naming

- Prefix with `test_` for unit/integration tests
- Prefix with `prop_` for property-based tests
- Use descriptive names: `test_health_endpoint_returns_200_when_healthy`

### Test Documentation

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

### Test Coverage (v0.3.0)

- Total tests: 327
- Passing: 100%
- Unit tests: 310
- Integration tests: 17
- Test categories:
  - Task management: 50+ tests
  - Execution engine: 10+ tests
  - Failure analysis: 4 tests
  - Scheduler: 7 tests
  - Concurrency control: 6 tests
  - WebSocket logs: 4 tests

## 🔧 API Development

### Creating New API Endpoints

1. **Define models** in `models.rs`:
```rust
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTaskRequest {
    pub workspace_id: i32,
    pub issue_number: i32,
    pub issue_title: String,
}
```

2. **Create handlers** in `handlers.rs`:
```rust
#[utoipa::path(
    post,
    path = "/api/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created", body = TaskResponse),
    )
)]
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, VibeRepoError> {
    // Implementation
}
```

3. **Define routes** in `routes.rs`:
```rust
pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(handlers::create_task))
        .route("/:id", get(handlers::get_task))
}
```

4. **Register router** in `api/mod.rs`:
```rust
let app = Router::new()
    .nest("/api/tasks", tasks::routes::create_router());
```

5. **Write tests** in `tests/`:
```rust
#[tokio::test]
async fn test_create_task_returns_201() {
    // Test implementation
}
```

## 🗄️ Database Development

### Using SeaORM

```rust
use sea_orm::*;
use crate::entities::{task, prelude::*};

// Query
let tasks = Task::find()
    .filter(task::Column::Status.eq("Pending"))
    .all(&db)
    .await?;

// Insert
let new_task = task::ActiveModel {
    workspace_id: Set(1),
    issue_number: Set(42),
    task_status: Set(TaskStatus::Pending),
    ..Default::default()
};
let result = Task::insert(new_task).exec(&db).await?;

// Update
let mut task: task::ActiveModel = task.into();
task.task_status = Set(TaskStatus::Running);
task.update(&db).await?;
```

### Creating Migrations

```bash
cd backend
sea-orm-cli migrate generate <migration_name>
```

Edit the generated migration file:
```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Task::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Task::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Task::WorkspaceId).integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Task::Table).to_owned())
            .await
    }
}
```

## 🔌 Git Provider Integration

### Implementing a New Provider

1. **Create provider module** in `src/git_provider/<provider>/`:
```rust
// src/git_provider/github/mod.rs
pub mod client;
pub mod models;
```

2. **Implement GitProvider trait**:
```rust
#[async_trait]
impl GitProvider for GitHubClient {
    async fn list_repositories(&self, page: Option<u32>) -> Result<Vec<Repository>, GitProviderError> {
        // Implementation
    }
    
    async fn list_issues(&self, owner: &str, repo: &str, params: Option<IssueListParams>) -> Result<Vec<Issue>, GitProviderError> {
        // Implementation
    }
    
    // ... other methods
}
```

3. **Update factory**:
```rust
// src/git_provider/factory.rs
pub fn from_provider(provider: &repo_provider::Model) -> Result<Box<dyn GitProvider>, GitProviderError> {
    match provider.r#type.as_str() {
        "gitea" => Ok(Box::new(GiteaClient::new(/* ... */))),
        "github" => Ok(Box::new(GitHubClient::new(/* ... */))),
        _ => Err(GitProviderError::UnsupportedProvider(provider.r#type.clone())),
    }
}
```

4. **Write tests**:
```rust
#[tokio::test]
async fn test_github_list_repositories() {
    // Test implementation
}
```

## 🐳 Docker Development

### Building Workspace Image

```bash
cd backend
docker build -t vibe-workspace -f Dockerfile.workspace .
```

### Testing Container Integration

```bash
# Start a test container
docker run -d --name test-workspace vibe-workspace

# Execute commands
docker exec test-workspace ls -la

# Check logs
docker logs test-workspace

# Clean up
docker stop test-workspace
docker rm test-workspace
```

## 📊 Logging

Use `tracing` macros for logging:

```rust
use tracing::{info, warn, error, debug};

#[tracing::instrument(skip(db))]
async fn create_task(db: &DatabaseConnection, req: CreateTaskRequest) -> Result<Task> {
    info!("Creating task for issue #{}", req.issue_number);
    
    let task = Task::insert(/* ... */).exec(db).await?;
    
    debug!("Task created with id: {}", task.id);
    Ok(task)
}
```

### Log Levels

- `error!` - Critical errors that need immediate attention
- `warn!` - Warning conditions
- `info!` - Informational messages
- `debug!` - Debug-level messages
- `trace!` - Very detailed tracing

## 🔐 Security Best Practices

1. **Never log sensitive data**:
```rust
// Bad
info!("Token: {}", token);

// Good
info!("Token: {}***", &token[..8]);
```

2. **Validate all inputs**:
```rust
pub fn validate_issue_number(num: i32) -> Result<(), ValidationError> {
    if num <= 0 {
        return Err(ValidationError::InvalidIssueNumber);
    }
    Ok(())
}
```

3. **Use prepared statements** (SeaORM does this automatically)

4. **Implement rate limiting** for API endpoints

5. **Verify webhook signatures**:
```rust
fn verify_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    let expected = hmac_sha256(secret.as_bytes(), payload);
    constant_time_compare(&expected, signature)
}
```

## 🚀 Performance Optimization

### Database Queries

```rust
// Bad: N+1 query
for task in tasks {
    let workspace = Workspace::find_by_id(task.workspace_id).one(&db).await?;
}

// Good: Use join
let tasks_with_workspace = Task::find()
    .find_also_related(Workspace)
    .all(&db)
    .await?;
```

### Async Operations

```rust
// Bad: Sequential
let repo1 = fetch_repository(1).await?;
let repo2 = fetch_repository(2).await?;

// Good: Concurrent
let (repo1, repo2) = tokio::join!(
    fetch_repository(1),
    fetch_repository(2)
);
```

### Caching

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Cache {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl Cache {
    pub async fn get(&self, key: &str) -> Option<String> {
        self.data.read().await.get(key).cloned()
    }
    
    pub async fn set(&self, key: String, value: String) {
        self.data.write().await.insert(key, value);
    }
}
```

## 🧪 Experimentation Standards

When conducting feature verification or experimental development:

1. **Use the `tmp/` directory**: All experimental code must be placed in the `tmp/` directory at project root
2. **Git-ignored**: The `tmp/` directory is automatically ignored by git
3. **Isolation**: Keep experiments isolated from production code
4. **Documentation**: Document your experiments in `tmp/README.md` if needed

**Example:**
```bash
# Create experiment directory
mkdir -p tmp/feature-experiment

# Run your experiments
cd tmp/feature-experiment
cargo new test-feature

# Experiments are safe and won't be committed
```

**Why this matters:**
- Prevents accidental commits of experimental code
- Keeps the repository clean
- Allows safe experimentation without affecting the main codebase
- Makes it easy to clean up experiments

## 🔄 Git Workflow

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
feat(api): add task scheduler service
fix(db): fix unique constraint validation logic
test(api): add task execution integration tests
docs: update development guide
```

### Branch Strategy

- `main` - Stable release branch
- `develop` - Development branch
- `feature/<name>` - Feature branches
- `fix/<name>` - Bug fix branches
- `release/<version>` - Release preparation branches

### Pull Request Process

1. Create feature branch from `develop`
2. Implement feature with tests
3. Run all tests and linters
4. Create pull request to `develop`
5. Address review comments
6. Merge after approval

## 📚 Related Documentation

- **[Database Schema](../database/schema.md)** - Complete database schema reference
- **[API Documentation](../api/)** - API specifications and usage guides
- **[Testing Documentation](../tests/)** - Test plans and strategies
- **[Roadmap](../roadmap/)** - Project roadmap and planned features

## 🆘 Getting Help

- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Ask questions in GitHub Discussions
- **Documentation**: Check [docs/README.md](../README.md)
- **API Reference**: Access Swagger UI at `http://localhost:3000/swagger-ui`

---

**Maintained By:** VibeRepo Team  
**Last Updated:** 2026-01-21
