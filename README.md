# VibeRepo

**Version:** 0.1.1 (Pre-1.0 - Breaking changes allowed)

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

## Features

- **Multi-Provider Support**: Unified interface for Gitea, GitHub, and GitLab
- **Automated Repository Management**: Automatic sync and validation of repositories
- **Repository Initialization**: Automated branch and label setup for new repositories
- **Webhook Integration**: Real-time event processing from Git providers
- **Background Services**: Scheduled repository synchronization
- **RESTful API**: Comprehensive API with OpenAPI documentation
- **Database Flexibility**: Support for both SQLite (development) and PostgreSQL (production)

## Technology Stack

- **Language**: Rust (Edition 2021)
- **Framework**: Axum 0.7 with WebSocket support
- **Async Runtime**: Tokio with full features
- **Database ORM**: SeaORM 1.1 (supports SQLite and PostgreSQL)
- **HTTP Client**: Reqwest 0.11 for Git provider APIs
- **API Documentation**: utoipa 4.x with Swagger UI
- **Testing**: Comprehensive TDD approach with 181+ tests

## Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- SQLite 3 or PostgreSQL
- Git provider account (Gitea/GitHub/GitLab)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/vibe-repo.git
cd vibe-repo/backend
```

2. Create `.env` file in project root:
```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
```

3. Build and run:
```bash
cargo build
cargo run
```

The server will start at `http://localhost:3000`

### Verify Installation

Check the health endpoint:
```bash
curl http://localhost:3000/health
```

Access the API documentation:
```
http://localhost:3000/swagger-ui
```

## API Endpoints

### Health Check
- `GET /health` - Service health status with database connectivity check

### Settings Module

#### RepoProvider Management
- `GET /api/settings/providers` - List all Git provider configurations
- `POST /api/settings/providers` - Create a new provider configuration
- `GET /api/settings/providers/:id` - Get provider details
- `PUT /api/settings/providers/:id` - Update provider configuration
- `DELETE /api/settings/providers/:id` - Delete provider (if not locked)
- `POST /api/settings/providers/:id/validate` - Validate provider token
- `POST /api/settings/providers/:id/sync` - Manually trigger repository sync

### Repository Module
- `GET /api/repositories` - List repositories with optional filters
- `GET /api/repositories/:id` - Get repository details
- `POST /api/repositories/:id/refresh` - Refresh repository validation status
- `POST /api/repositories/:id/initialize` - Initialize single repository
- `POST /api/repositories/batch-initialize` - Batch initialize repositories

### Webhook Module
- `POST /api/webhooks/:repository_id` - Receive webhook events from Git providers

## Development

### Build Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run
```

### Testing

```bash
# Run all tests (181+ tests)
cargo test

# Run specific test
cargo test test_name

# Run with output visible
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

### Code Quality

```bash
# Check for warnings and style issues
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --check
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
│   ├── api/                 # HTTP API layer
│   │   ├── health/          # Health check module
│   │   ├── settings/        # Settings namespace
│   │   │   └── providers/   # RepoProvider API
│   │   ├── repositories/    # Repository API
│   │   └── webhooks/        # Webhook receiver
│   ├── services/            # Background services
│   ├── git_provider/        # Git provider abstraction
│   │   ├── traits.rs        # GitProvider trait
│   │   ├── factory.rs       # GitClientFactory
│   │   └── gitea/           # Gitea implementation
│   ├── db/                  # Database connection
│   ├── entities/            # SeaORM entities
│   ├── migration/           # Database migrations
│   └── test_utils/          # Test utilities
└── tests/                   # Integration tests
```

## Configuration

Configuration is loaded from `.env` file:

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | Database connection URL | `sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc` |
| `DATABASE_MAX_CONNECTIONS` | Max database connections | `10` |
| `SERVER_HOST` | Server bind address | `0.0.0.0` |
| `SERVER_PORT` | Server port | `3000` |
| `RUST_LOG` | Log level | `info` |
| `LOG_FORMAT` | Log format (text/json) | `text` |

## Database Schema

### repo_providers
Git provider configurations with authentication credentials.

**Key Fields:**
- `name`, `type`, `base_url`, `access_token`
- `locked` - Prevents deletion when true
- Unique constraint on (name, base_url, access_token)

### repositories
Repository records with validation status.

**Key Fields:**
- `provider_id` (FK to repo_providers)
- `name`, `full_name`, `clone_url`, `default_branch`
- `validation_status` - 'valid', 'invalid', 'pending'
- Validation flags: `has_required_branches`, `has_required_labels`, etc.

### webhook_configs
Webhook configurations for repository event monitoring.

**Key Fields:**
- `repository_id` (FK to repositories, one-to-one)
- `provider_id` (FK to repo_providers, redundant for performance)
- `webhook_id`, `webhook_secret`, `webhook_url`
- `events` - JSON array of subscribed events
- Retry mechanism: `retry_count`, `last_retry_at`, `next_retry_at`

## Architecture

### Module Hierarchy

```
Settings (namespace)
└── RepoProvider (entity)
    └── Repository (entity) [many-to-one]
        └── Workspace (entity) [one-to-one] (planned)
            ├── Agent (entity) [one-to-many] (planned)
            └── Task (entity) [one-to-many] (planned)
```

### Git Provider Abstraction

Unified interface for different Git platforms:

```rust
use vibe_repo::git_provider::{GitProvider, GitClientFactory};

// Create a client from a RepoProvider entity
let client = GitClientFactory::from_provider(&provider)?;

// Use the unified interface
let repos = client.list_repositories(None).await?;
let branches = client.list_branches("owner", "repo").await?;
```

**Supported Operations:**
- Repository operations (list, get)
- Branch operations (list, get, create, delete)
- Issue operations (list, get, create, update, add/remove labels)
- Pull request operations (list, get, create, update, merge)
- Label operations (list, create, delete)
- Webhook operations (create, delete, list)

## Testing Philosophy

This project follows **Test-Driven Development (TDD)**:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Refactor code while keeping tests passing

**Test Coverage (v0.1.1):**
- Total tests: 181+
- Passing: 100%
- Property tests: 14
- Integration tests: 12+
- Unit tests: 155+

## Contributing

### Development Standards

- **Language**: English for all code, comments, and documentation
- **Commit Messages**: Follow Conventional Commits specification
- **Code Style**: Run `cargo fmt` before committing
- **Testing**: Write tests first (TDD approach)
- **Breaking Changes**: Allowed before v1.0.0

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

**Scopes:** `api`, `db`, `deps`, `test`, `docs`

**Examples:**
```bash
feat(api): Add repository initialization feature
fix(db): Fix unique constraint validation logic
test(api): Add webhook integration tests
```

## Roadmap

### Current Status (v0.1.1)

**Completed:**
- ✅ Backend Foundation
- ✅ RepoProvider API
- ✅ Repository API
- ✅ Git Provider Abstraction (Gitea)
- ✅ Repository Initialization
- ✅ Webhook Integration

**In Progress:**
- 🟡 Workspace API (planned next)
- 🟡 GitHub/GitLab provider implementations

**Planned:**
- 📋 Agent Management
- 📋 Task Automation
- 📋 Issue-to-PR Workflow

## License

[Add your license here]

## Support

- **Documentation**: See [AGENTS.md](./AGENTS.md) for detailed development guidelines
- **Issues**: Report bugs and feature requests on GitHub Issues
- **API Docs**: Access Swagger UI at `http://localhost:3000/swagger-ui`

## Acknowledgments

Built with Rust and powered by:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SeaORM](https://www.sea-ql.org/SeaORM/) - Database ORM
- [Tokio](https://tokio.rs/) - Async runtime
- [utoipa](https://github.com/juhaku/utoipa) - OpenAPI documentation
