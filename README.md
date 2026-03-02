# VibeRepo

**Version:** 0.4.0-mvp (Simplified MVP)

> **🎯 Simplified MVP**: This version focuses on core Issue-to-PR automation with a streamlined architecture. Many advanced features have been removed to create a solid foundation for future development. See [CHANGELOG.md](./CHANGELOG.md) for details.

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

## ✨ Key Features (Simplified MVP)

- **Automated Issue-to-PR Workflow** - Convert issues to pull requests automatically via webhooks
- **Repository-Centric Architecture** - Each repository is self-contained with its own provider configuration
- **Single Agent per Repository** - Simplified agent management with automatic assignment
- **Docker-based Workspaces** - Isolated development environments for each repository
- **Task Management** - Simple task lifecycle (Pending → Running → Completed/Failed/Cancelled)
- **Webhook Integration** - GitHub/Gitea/GitLab webhook support for automatic task creation
- **Manual Repository Addition** - Explicitly add only the repositories you want to automate
- **ACP Integration** ⭐ - Agent Client Protocol for structured agent communication
- **Real-time Progress Tracking** ⭐ - Monitor agent execution with plans and events
- **Permission-based Security** ⭐ - Fine-grained control over agent actions
- **10x Faster Startup** ⭐ - Bun runtime for lightning-fast agent initialization

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ ([rustup.rs](https://rustup.rs)) or Docker
- SQLite 3 or PostgreSQL
- Docker (for workspace features)
- **@devcontainers/cli** - DevContainer CLI for workspace management
- Git provider account (Gitea/GitHub/GitLab)

#### Install DevContainer CLI

```bash
# Install globally via npm
npm install -g @devcontainers/cli

# Verify installation
devcontainer --version
```

> **Note**: The DevContainer CLI is required for creating and managing isolated development environments. It replaces the legacy Docker API integration.

### Option 1: Docker (推荐)

```bash
# Clone repository
git clone https://github.com/yourusername/vibe-repo.git
cd vibe-repo

# Copy and configure environment
cp .env.docker .env
# Edit .env and set WEBHOOK_SECRET_KEY

# Start with Docker Compose
docker-compose up -d

# View logs
docker-compose logs -f
```

📖 **详细说明**: [Docker 部署指南](./docs/deployment/docker.md)

### Option 2: 本地开发

```bash
# Clone repository
git clone https://github.com/yourusername/vibe-repo.git
cd vibe-repo/backend

# Create .env file
cat > .env << EOF
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# DevContainer CLI Configuration
DEVCONTAINER_CLI_PATH=devcontainer

# Agent Configuration (ACP Integration)
AGENT_TYPE=opencode
AGENT_API_KEY=sk-ant-api03-xxx
AGENT_DEFAULT_MODEL=claude-sonnet-4
AGENT_TIMEOUT_SECONDS=600

# Workspace Configuration
WORKSPACE_BASE_DIR=./data/vibe-repo/workspaces

# Logging
RUST_LOG=debug
EOF

# Build and run
cargo build
cargo run
```

Server starts at `http://localhost:3000`

### Verify Installation

```bash
# Check health
curl http://localhost:3000/health

# Access API docs
open http://localhost:3000/swagger-ui
```

## 📖 Documentation

### For Users
- **[User Guide](./docs/api/user-guide.md)** - Complete usage guide with examples
- **[API Reference](./docs/api/api-reference.md)** - All API endpoints
- **[Configuration Guide](./docs/development/README.md#configuration)** - Environment setup
- **[DevContainer Guide](./docs/api/devcontainer-guide.md)** 🆕 - Using devcontainer.json for custom environments
- **[ACP Integration Guide](./docs/api/acp-integration.md)** ⭐ - Agent Client Protocol documentation
- **[Agent Quick Reference](./docs/api/agent-quick-reference.md)** ⭐ - Quick agent configuration
- **[MCP Integration](./docs/api/mcp-integration.md)** ⭐ - Model Context Protocol servers
- **[Troubleshooting](./docs/api/troubleshooting.md)** ⭐ - Common issues and solutions

### For Developers
- **[Development Guide](./docs/development/README.md)** - Setup, coding standards, best practices
- **[Database Schema](./docs/database/schema.md)** - Complete database reference
- **[Testing Guide](./docs/tests/README.md)** - Testing strategy and TDD workflow

### For Contributors
- **[Roadmap](./docs/roadmap/README.md)** - Project roadmap and planned features
- **[Design Documents](./docs/design/)** - Feature designs and architecture
- **[Implementation Plans](./docs/plans/)** - Detailed implementation plans

### Quick References
- **[AGENTS.md](./AGENTS.md)** - AI agent coding guidelines
- **[docs/README.md](./docs/README.md)** - Documentation index

## 🏗️ Architecture

```
Manual Repository Addition
  ↓
Repository + Workspace + Agent Creation (Atomic)
  ↓
Webhook Setup
  ↓
Issue Detection (Webhook)
  ↓
Task Creation
  ↓
Task Scheduler (Priority-based)
  ↓
Docker Container Execution
  ↓
PR Creation
  ↓
Issue Closure
```

**Technology Stack:**
- **Language**: Rust (Edition 2021)
- **Framework**: Axum 0.7
- **Database**: SeaORM 1.1 (SQLite/PostgreSQL)
- **Async Runtime**: Tokio
- **Agent Protocol**: ACP (Agent Client Protocol) ⭐
- **Agent Runtime**: Bun (10x faster than Node.js) ⭐
- **Default Agent**: OpenCode with native ACP support ⭐
- **Testing**: 280+ tests (100% passing)

**Architecture Highlights:**
- Repository-Centric: Each repository is self-contained with provider configuration
- 2 database tables (down from 4): `repositories`, `workspaces`
- 1 database query per operation (down from 2)
- Per-repository token management (principle of least privilege)
- Support for mixed providers (GitHub + Gitea + GitLab)

## 🗺️ Roadmap

**Current Status (v0.4.0-mvp):**
- ✅ Repository-Centric Architecture (2 tables instead of 4)
- ✅ Manual Repository Addition with Provider Configuration
- ✅ Per-Repository Token Management
- ✅ Complete Issue-to-PR Automation
- ✅ Task Scheduler with Priority-based Execution
- ✅ Webhook Integration (GitHub/Gitea/GitLab)

**Next Steps:**
- 📋 Token Encryption (envelope encryption for access tokens)
- 📋 Import from URL (auto-detect provider config from clone URL)
- 📋 Bulk Token Update API
- 📋 Task Execution Metrics Dashboard
- 📋 Multi-Agent Coordination

See [full roadmap](./docs/roadmap/README.md) for details.

## 🧪 Testing

```bash
# Run all tests (327 tests)
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

**Test Coverage:**
- Unit tests: 310
- Integration tests: 17
- Coverage: 100% passing

See [Testing Guide](./docs/tests/README.md) for details.

### End-to-End Tests

Run E2E tests with real Gitea instance:

```bash
./scripts/run_e2e_tests.sh
```

See [E2E Testing Guide](docs/testing/e2e-testing.md) for details.

## 🤝 Contributing

We welcome contributions! Please see:

- **[Development Guide](./docs/development/README.md)** - Setup and coding standards
- **[Roadmap](./docs/roadmap/README.md)** - Planned features
- **[AGENTS.md](./AGENTS.md)** - AI agent guidelines

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

Examples:
feat(api): add task scheduler service
fix(db): fix unique constraint validation
docs: update user guide
```

## 📊 Project Status

| Metric | Value |
|--------|-------|
| Version | 0.4.0-mvp |
| Tests | 280+ (100% passing) |
| API Endpoints | 10 core endpoints |
| Database Tables | 2 (repositories, workspaces) |
| Documentation | 60+ files |
| Architecture | Repository-Centric |

## 📝 License

[Add your license here]

## 🆘 Support

- **Documentation**: [docs/README.md](./docs/README.md)
- **API Docs**: `http://localhost:3000/swagger-ui`
- **Issues**: [GitHub Issues](https://github.com/yourusername/vibe-repo/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/vibe-repo/discussions)

## 🙏 Acknowledgments

Built with Rust and powered by:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SeaORM](https://www.sea-ql.org/SeaORM/) - Database ORM
- [Tokio](https://tokio.rs/) - Async runtime
- [utoipa](https://github.com/juhaku/utoipa) - OpenAPI documentation

---

**Made with ❤️ by the VibeRepo Team**
