# VibeRepo Documentation

**Version:** 0.4.0

Welcome to the VibeRepo documentation! This directory contains comprehensive documentation for the VibeRepo project, organized by category.

## 📚 Documentation Structure

### 📡 [API Documentation](./api/)
API specifications, usage guides, and feature documentation.

- **[Task API Design](./api/task-api-design.md)** - Complete Task API specifications with 14 endpoints
- **[Issue Polling Feature](./api/issue-polling-feature.md)** - Automatic issue synchronization with intelligent filtering
- **[Issue Polling Fallback Design](./api/issue-polling-fallback-design.md)** - Dual-mode issue tracking (webhook + polling)
- **[Init Scripts Guide](./api/init-scripts-guide.md)** - Automated container setup with custom shell scripts
- **[Migration Guide: Init Scripts](./api/migration-guide-init-scripts.md)** - Migrating from custom_dockerfile_path to init scripts
- **[Container Lifecycle Management](./api/container-lifecycle-management.md)** - Docker container management with health monitoring

### 🗄️ [Database Documentation](./database/)
Database schema, migrations, and data model documentation.

- **Schema Documentation** - Complete database schema reference (see AGENTS.md)
- **Migration Guides** - Database migration instructions and best practices

### 🎨 [Design Documentation](./design/)
Feature design documents, architecture decisions, and technical specifications.

- **[Initial PRD](./design/INIT-PRD.md)** - Product Requirements Document
- **[Workspace Feature Analysis](./design/workspace-feature-analysis.md)** - Workspace feature design and analysis

### 📋 [Plans](./plans/)
Implementation plans, roadmaps, and session summaries.

- **[Implementation Roadmap](./plans/2026-01-17-implementation-roadmap.md)** - Overall project roadmap
- **[Repository Management Design](./plans/2026-01-17-repository-management-design.md)** - Repository management feature design
- **[Workspace Implementation Plans](./plans/)** - Phase-by-phase workspace implementation
- **[Container Lifecycle Management Design](./plans/2026-01-20-container-lifecycle-management-design.md)** - Container management design
- **[Webhook Refactor Plan](./plans/webhook-refactor-plan.md)** - Webhook system refactoring
- **[AgentFS Research](./plans/)** - AgentFS integration research and test results

### 🔬 [Research Documentation](./research/)
Research findings, comparisons, and technical investigations.

- **[Task Implementation Research](./research/task-implementation-research.md)** - Task execution engine research
- **[Agents Comparison](./research/agents-comparison-final-summary.md)** - AI coding agents comparison
- **[Coding Agents Capture Research](./research/coding-agents-capture-research-summary.md)** - Message capture research
- **[Claude Code Capture Demo](./research/claude-code-capture-demo-summary.md)** - Claude Code message capture demo
- **[ACP Message History Analysis](./research/acp-message-history-analysis.md)** - Anthropic Claude Protocol analysis
- **[Gemini CLI Capture Analysis](./research/gemini-cli-capture-analysis.md)** - Gemini CLI capture analysis

### 🧪 [Testing Documentation](./tests/)
Test plans, test reports, and testing strategies.

- **[AgentFS Integration Tests](./tests/)** - Docker and container integration test plans and results
- **[OpenCode Capture Test Report](./tests/opencode-capture-test-report.md)** - OpenCode capture testing
- **[Testing Strategy](./tests/README.md)** - Comprehensive testing approach and best practices

### 🗺️ [Roadmap](./roadmap/)
Project roadmap, milestones, and version history.

- **[Project Roadmap](./roadmap/README.md)** - Complete roadmap with completed features, current work, and planned enhancements
- **Completed Features** - v0.1.0 to v0.4.0 feature history
- **In Progress** - Current development status
- **Planned Features** - Short-term, mid-term, and long-term plans
- **Milestones** - Key project milestones and target dates

### 🛠️ [Development Guide](./development/)
Development guidelines, coding standards, and best practices.

- **[Development Guide](./development/README.md)** - Comprehensive development guide
- **Quick Start** - Setup development environment
- **Code Style Guidelines** - Naming conventions, formatting, and patterns
- **Testing Philosophy** - TDD workflow and test structure
- **API Development** - Creating new endpoints
- **Database Development** - Using SeaORM and migrations
- **Git Workflow** - Commit standards and branch strategy

## 🚀 Quick Links

### For Developers
- [Development Guide](./development/) - Setup, coding standards, and best practices
- [AGENTS.md](../AGENTS.md) - Quick reference for AI agents
- [API Documentation](./api/) - API specifications and usage guides
- [Database Schema](./database/schema.md) - Complete database reference

### For Contributors
- [Roadmap](./roadmap/) - Project roadmap and planned features
- [Design Documentation](./design/) - Feature design and architecture
- [Plans](./plans/) - Implementation roadmap and plans
- [Research](./research/) - Technical research and investigations

### For Users
- [README.md](../README.md) - Getting started and usage examples
- [API Documentation](./api/) - Feature guides and API reference

## 📖 Documentation Guidelines

### File Organization

- **API docs** (`docs/api/`) - API specifications, feature guides, usage examples
- **Database docs** (`docs/database/`) - Schema, migrations, data models
- **Design docs** (`docs/design/`) - Feature designs, architecture decisions
- **Plans** (`docs/plans/`) - Implementation plans, roadmaps, session notes
- **Research** (`docs/research/`) - Technical research, comparisons, investigations
- **Tests** (`docs/tests/`) - Test plans, test reports, testing strategies
- **Roadmap** (`docs/roadmap/`) - Project roadmap, milestones, version history
- **Development** (`docs/development/`) - Development guidelines, coding standards, best practices

### Naming Conventions

- Use descriptive, kebab-case filenames: `feature-name-design.md`
- Include dates for time-sensitive docs: `2026-01-21-feature-plan.md`
- Use clear prefixes for document types:
  - Design docs: `feature-name-design.md`
  - Plans: `YYYY-MM-DD-feature-plan.md`
  - Research: `topic-research.md` or `topic-analysis.md`
  - Tests: `YYYY-MM-DD-feature-test-plan.md` or `YYYY-MM-DD-feature-test-results.md`

### Documentation Standards

- **Language**: English for all documentation
- **Format**: Markdown with GitHub-flavored syntax
- **Structure**: Clear headings, code examples, and cross-references
- **Maintenance**: Keep docs up-to-date with code changes

## 🔄 Version History

- **v0.4.0** (2026-01-21) - Added Task Scheduler, Concurrency Control, Real-time Logs, Execution History, Failure Analysis
- **v0.3.0** (2026-01-20) - Added Container Lifecycle Management, Issue Polling, Task Automation
- **v0.2.0** (2026-01-19) - Added Workspace API, Init Scripts, Agent Management
- **v0.1.0** (2026-01-17) - Initial release with RepoProvider, Repository, Webhook APIs

## 📝 Contributing to Documentation

When adding new documentation:

1. Choose the appropriate directory based on content type
2. Follow naming conventions and file organization rules
3. Update this index (docs/README.md) with links to new documents
4. Use clear, concise language with code examples
5. Cross-reference related documents

## 🆘 Need Help?

- **Issues**: Report documentation issues on GitHub
- **Questions**: Ask in GitHub Discussions
- **API Reference**: Access Swagger UI at `http://localhost:3000/swagger-ui`

---

**Last Updated:** 2026-01-21  
**Maintained By:** VibeRepo Team
