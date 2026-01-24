# VibeRepo

**Version:** 0.3.0 (Pre-1.0 - Breaking changes allowed)

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

## ✨ Key Features

- **Automated Issue-to-PR Workflow** - Convert issues to pull requests automatically
- **Multi-Provider Support** - Unified interface for Gitea, GitHub, and GitLab
- **Docker-based Workspaces** - Isolated development environments with health monitoring
- **Task Scheduler** - Automatic background execution with priority-based scheduling
- **Real-time Monitoring** - WebSocket log streaming and intelligent failure analysis
- **Dual-Mode Issue Tracking** - Webhook-first with automatic polling fallback

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ ([rustup.rs](https://rustup.rs)) or Docker
- SQLite 3 or PostgreSQL
- Docker (for workspace features)
- Git provider account (Gitea/GitHub/GitLab)

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
Issue Detection (Webhook/Polling)
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
- **Framework**: Axum 0.7 with WebSocket
- **Database**: SeaORM 1.1 (SQLite/PostgreSQL)
- **Async Runtime**: Tokio
- **Testing**: 327 tests (100% passing)

## 🗺️ Roadmap

**Current Status (v0.3.0):**
- ✅ Complete Issue-to-PR automation (90% done)
- ✅ Task Scheduler with priority-based execution
- ✅ Real-time log streaming via WebSocket
- ✅ Intelligent failure analysis with recommendations

**Next Steps:**
- 🟡 Complete Issue-to-PR Workflow (PR creation)
- 📋 GitHub/GitLab provider implementations
- 📋 Task execution metrics dashboard
- 📋 Multi-Agent coordination

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
| Version | 0.3.0 |
| Tests | 589+ (100% passing) |
| API Endpoints | 50+ |
| Database Tables | 10 |
| Documentation | 55 files |

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
