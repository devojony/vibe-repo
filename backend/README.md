# GitAutoDev Backend

Rust-based backend service for the GitAutoDev automated programming assistant.

## Quick Start

### Prerequisites

- Rust 1.70+ (Edition 2021)
- SQLite or PostgreSQL

### Configuration

The backend uses environment variables for configuration. You can set them in two ways:

1. **Using `.env` file** (recommended for development):
   ```bash
   # Copy the example file
   cp ../.env.example ../.env
   
   # Edit .env with your settings
   ```

2. **Using environment variables directly**:
   ```bash
   export DATABASE_URL="sqlite:./data/gitautodev/db/gitautodev.db?mode=rwc"
   export SERVER_PORT=3000
   ```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite:./data/gitautodev/db/gitautodev.db?mode=rwc` | Database connection string |
| `DATABASE_MAX_CONNECTIONS` | `10` | Connection pool size |
| `SERVER_HOST` | `0.0.0.0` | Server bind address |
| `SERVER_PORT` | `3000` | Server port |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `LOG_FORMAT` | (empty) | Set to `json` for JSON logs (production) |

### Running the Server

```bash
# Development mode (from project root)
cargo run

# Or from backend directory
cd backend
cargo run

# With custom configuration
DATABASE_URL=sqlite:./custom.db cargo run
```

The server will:
1. Load `.env` file if it exists (from project root)
2. Initialize database connection
3. Run migrations automatically
4. Start HTTP server on configured host:port

### API Documentation

Once the server is running, access:

- **Swagger UI**: http://localhost:3000/swagger-ui
- **OpenAPI Spec**: http://localhost:3000/api-docs/openapi.json
- **Health Check**: http://localhost:3000/health

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test config
cargo test health

# Run with output
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

### Code Quality

```bash
# Check for warnings
cargo clippy

# Format code
cargo fmt

# Check formatting
cargo fmt --check
```

### Database Migrations

Migrations run automatically on startup. To manage migrations manually:

```bash
# Install SeaORM CLI
cargo install sea-orm-cli

# Create new migration
sea-orm-cli migrate generate <migration_name>

# Run migrations
sea-orm-cli migrate up

# Rollback migration
sea-orm-cli migrate down

# Generate entity models from database
sea-orm-cli generate entity -o src/entities
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
│   │   └── health/          # Health check module
│   ├── services/            # Background services
│   ├── db/                  # Database connection
│   ├── entities/            # SeaORM entities
│   ├── migration/           # Database migrations
│   └── test_utils/          # Test utilities
├── tests/                   # Integration tests
├── Cargo.toml               # Dependencies
└── Dockerfile               # Container image
```

## Architecture

The backend follows a layered architecture:

- **HTTP Layer**: Axum routes and handlers
- **Service Layer**: Business logic and state management
- **Data Layer**: SeaORM entities and migrations

### Key Patterns

- **Modular API Design**: Each feature has its own module with routes and handlers
- **Background Services**: Long-running tasks managed by ServiceManager
- **Error Handling**: Unified error types with HTTP response conversion
- **Testing**: Comprehensive unit, integration, and property-based tests

## Testing Philosophy

This project follows **Test-Driven Development (TDD)**:

1. Write failing test first
2. Implement minimal code to pass
3. Refactor while keeping tests green

All modules have:
- Unit tests for individual functions
- Integration tests for API endpoints
- Property-based tests for correctness properties

## Troubleshooting

### Database Connection Issues

If you see "unable to open database file":

```bash
# Create data directory
mkdir -p data/gitautodev/db

# Or use absolute path in DATABASE_URL
export DATABASE_URL="sqlite:/absolute/path/to/gitautodev.db?mode=rwc"
```

### Port Already in Use

If port 3000 is already in use:

```bash
# Use different port
export SERVER_PORT=8080
cargo run
```

### Environment Variables Not Loading

Make sure:
1. `.env` file is in the **project root** (not backend directory)
2. File is named exactly `.env` (not `.env.txt`)
3. No syntax errors in `.env` file

## License

[Your License Here]
