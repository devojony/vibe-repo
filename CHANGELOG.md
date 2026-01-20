# Changelog

All notable changes to VibeRepo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Version Policy

**Pre-1.0 (Current)**: Breaking changes allowed, version increments by 0.0.1 for each update.

**Post-1.0**: Follows strict semantic versioning (MAJOR.MINOR.PATCH).

---

## [Unreleased]

### Added
- **Init Scripts Feature**: Automated container setup with custom shell scripts
  - Create init scripts when creating workspaces
  - Update and execute scripts via API
  - Hybrid storage strategy (≤4KB in DB, >4KB in files)
  - Automatic log cleanup (30-day retention)
  - Concurrency control with database locking
  - 6 execution states: Pending, Running, Success, Failed, Timeout, Cancelled
  - 4 new API endpoints for script management
  - Comprehensive integration tests (12 new tests)
  - Complete documentation and migration guide

### Changed
- **BREAKING**: Removed `custom_dockerfile_path` field from workspaces
  - Replaced with more flexible init_script functionality
  - See `docs/migration-guide-init-scripts.md` for migration instructions
- Updated workspace API to include init_script in responses
- Enhanced OpenAPI documentation with init script endpoints
- Increased test coverage to 500+ tests

### Planned
- Workspace API implementation
- Agent configuration management
- Task automation system
- GitHub provider implementation
- GitLab provider implementation

---

## [0.1.1] - 2026-01-16

### Added
- Project skill system in `.opencode/skills/`
- `vibe-repo-development.md` skill for development workflow guidance
- Development progress tracking document (CHANGELOG.md)

### Changed
- Updated AGENTS.md with comprehensive development guidelines
- Enhanced documentation structure

---

## [0.1.0] - 2026-01-14

### Added
- **Backend Foundation**
  - Configuration management with environment variables
  - Database connection pooling (SQLite/PostgreSQL)
  - Unified error handling with `VibeRepoError`
  - Health check endpoint with database connectivity check
  - Structured logging with tracing
  - Graceful shutdown support

- **RepoProvider API** (Settings Module)
  - CRUD operations for Git provider configurations
  - Token validation endpoint
  - Manual repository sync trigger
  - Access token masking in responses
  - Unique constraint on (name, base_url, access_token)
  - Locked provider protection

- **Repository API**
  - List repositories with filtering (provider_id, validation_status)
  - Get repository details
  - Refresh validation status
  - Single repository initialization
  - Batch repository initialization
  - Automatic repository sync on provider create/update

- **Git Provider Abstraction**
  - Unified `GitProvider` trait interface
  - Gitea client implementation (full support)
  - GitHub client placeholder
  - GitLab client placeholder
  - Static dispatch with compile-time polymorphism
  - Repository, branch, issue, PR, and label operations

- **Repository Initialization**
  - Configurable branch creation
  - Label management (vibe/* prefix)
  - Validation checks (branches, labels, permissions)
  - Background sync service (hourly)

- **Database Schema**
  - `repo_providers` table with authentication
  - `repositories` table with validation status
  - Cascade delete relationships
  - SeaORM entity definitions
  - Automatic migrations on startup

- **API Documentation**
  - OpenAPI 3.0 specification
  - Swagger UI at `/swagger-ui`
  - Comprehensive endpoint documentation

- **Testing Infrastructure**
  - 181+ total tests (100% passing)
  - Unit tests in source files
  - Integration tests in `tests/` directory
  - Property-based tests with proptest
  - Test utilities for database and state management

### Technical Details
- **Language**: Rust 2021
- **Framework**: Axum 0.7
- **Database**: SeaORM 0.12
- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest 0.11
- **API Docs**: utoipa 4.x

### Development Standards
- Test-Driven Development (TDD) methodology
- Conventional Commits for commit messages
- English for all code and documentation
- Comprehensive code style guidelines
- Module documentation requirements

---

## Version History Summary

| Version | Date | Key Features | Status |
|---------|------|--------------|--------|
| 0.1.1 | 2026-01-16 | Skill system, documentation | Current |
| 0.1.0 | 2026-01-14 | Initial release, core APIs | Stable |

---

## How to Update Version

When making changes:

1. **Increment version** in `backend/Cargo.toml`:
   ```toml
   version = "0.1.2"  # Increment by 0.0.1
   ```

2. **Update CHANGELOG.md**:
   - Move items from `[Unreleased]` to new version section
   - Add date in format `YYYY-MM-DD`
   - Categorize changes: Added, Changed, Deprecated, Removed, Fixed, Security

3. **Update AGENTS.md**:
   - Update version number in header
   - Update "Current Status" section if needed

4. **Commit with conventional commit**:
   ```bash
   git add backend/Cargo.toml CHANGELOG.md AGENTS.md
   git commit -m "chore: bump version to 0.1.2"
   ```

---

## Categories

- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security vulnerability fixes

---

## Links

- [Project Repository](https://github.com/yourusername/vibe-repo)
- [Issue Tracker](https://github.com/yourusername/vibe-repo/issues)
- [Documentation](./docs/)
- [AGENTS.md](./AGENTS.md) - Development Guidelines

---

**Note**: This project is in pre-1.0 development. Breaking changes may occur between versions.
