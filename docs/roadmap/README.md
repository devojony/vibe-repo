# VibeRepo Roadmap

**Version:** 0.4.0-mvp (Simplified MVP)  
**Last Updated:** 2026-02-06

> **🎯 Simplified MVP**: This roadmap reflects the simplified architecture. The focus is on building a solid foundation before adding advanced features.

This document outlines the development roadmap for VibeRepo, including completed features, current work, and planned enhancements.

## 🎯 Project Vision

VibeRepo is an automated programming assistant that converts Git repository Issues directly into Pull Requests. The system combines Rust's high-performance concurrency, Docker's environment isolation, and AI CLI tools to achieve end-to-end development automation.

## 📊 Current Status (v0.4.0-mvp)

### ✅ Completed Features (Simplified MVP)

#### Core Automation (v0.4.0-mvp)
- ✅ Backend Foundation
  - Configuration management via environment variables
  - Database connection (SQLite/PostgreSQL)
  - Error handling system
  - Basic health check endpoint
  - Logging infrastructure

#### Git Provider Integration (v0.4.0-mvp)
- ✅ Environment-based Provider Configuration
  - GitHub token configuration via GITHUB_TOKEN
  - Base URL configuration via GITHUB_BASE_URL
  - Webhook secret via WEBHOOK_SECRET
- ✅ Repository API
  - Repository listing and validation
  - Validation status tracking
  - Repository initialization
- ✅ Git Provider Abstraction
  - Unified interface for GitHub
  - Static dispatch Git client
  - Compile-time polymorphism

#### Event Processing (v0.4.0-mvp)
- ✅ Webhook Integration
  - Real-time event processing
  - Signature verification
  - Repository-based webhook URLs

#### Workspace & Agent Management (v0.4.0-mvp)
- ✅ Simplified Workspace Management
  - Docker-based isolated environments
  - Basic container lifecycle management
  - Single agent per workspace
- ✅ Environment-based Agent Configuration
  - DEFAULT_AGENT_COMMAND configuration
  - DEFAULT_AGENT_TIMEOUT configuration
  - DEFAULT_DOCKER_IMAGE configuration
  - Automatic agent assignment

#### Task Management (v0.4.0-mvp)
- ✅ Simplified Task Automation
  - Complete task lifecycle management
  - Simplified state machine (no Assigned state)
  - Priority management (High/Medium/Low)
  - Automatic agent assignment
  - Soft delete support
  - PR integration tracking
  - Inline log storage (tasks.last_log)

#### Task Execution (v0.4.0-mvp)
- ✅ Basic Task Execution Engine
  - Docker-based task execution
  - Command building with task context
  - Output parsing for PR information
  - Status auto-update
  - Inline log storage

#### Pull Request Automation (v0.4.0-mvp)
- ✅ PR Creation Service
  - Automatic PR creation from completed tasks
  - Manual PR creation endpoint
  - PR body formatting with "Closes #N"
- ✅ Issue Closure Service
  - Manual issue closure endpoint
  - Status synchronization
- ✅ Complete Issue-to-PR Workflow
  - End-to-end automation from issue to PR
  - Automatic branch creation and commits
  - PR information extraction from agent output

---

## 🗑️ Removed Features (from v0.3.0)

The following features were removed in the simplified MVP to create a solid foundation:

### Background Services
- ❌ Issue Polling Service (use webhooks only)
- ❌ Webhook Retry Service (simplified error handling)
- ❌ Webhook Cleanup Service
- ❌ Log Cleanup Service
- ❌ Init Script Service (workspaces use default setup)
- ❌ Task Failure Analyzer (basic error messages only)
- ❌ Health Check Service (basic health endpoint only)
- ❌ Image Management Service (use default Docker images)

### API Endpoints
- ❌ Provider Management API (configured via environment variables)
- ❌ Workspace Management API (workspaces created automatically)
- ❌ Agent Management API (agents configured via environment variables)
- ❌ Init Script API (no custom init scripts)
- ❌ Task Retry Endpoint (no manual retry)
- ❌ Task Assignment Endpoint (automatic assignment)
- ❌ Webhook Config API (configured via environment variables)

### Features
- ❌ WebSocket Real-time Logs (logs stored in tasks.last_log)
- ❌ Task Execution History (no task_executions table)
- ❌ Multiple Agents per Workspace (one agent per workspace)
- ❌ Manual Agent Assignment (automatic assignment)
- ❌ Task Retry Mechanism (no automatic retry)
- ❌ Assigned Task State (simplified state machine)
- ❌ Issue Polling (webhook-only)
- ❌ Custom Init Scripts (default workspace setup)

### Database Tables
- ❌ repo_providers (configured via environment variables)
- ❌ webhook_configs (configured via environment variables)
- ❌ init_scripts (no custom init scripts)
- ❌ task_executions (logs in tasks.last_log)
- ❌ task_logs (logs in tasks.last_log)
- ❌ containers (info in workspaces table)

---

## 📋 Planned Features

### Phase 1: Restore Essential Features (v0.5.0)
**Target Date:** 2026-03-15  
**Priority:** High

#### 1. Task Retry Mechanism
- [ ] Add retry_count and max_retries fields back to tasks table
- [ ] Implement automatic retry on failure
- [ ] Add manual retry endpoint
- [ ] Implement exponential backoff

**Success Criteria:**
- Failed tasks automatically retry up to max_retries
- Manual retry endpoint available
- Retry count tracked in database

#### 2. Task Execution History
- [ ] Restore task_executions table
- [ ] Track execution attempts with full logs
- [ ] Add execution history API endpoint
- [ ] Implement log file storage for large outputs

**Success Criteria:**
- All execution attempts tracked
- Historical logs accessible via API
- Large logs stored in files

#### 3. Real-time Log Streaming
- [ ] Restore WebSocket log streaming
- [ ] Implement task_logs table
- [ ] Add log level filtering
- [ ] Implement log retention policy

**Success Criteria:**
- Real-time logs available via WebSocket
- Log levels: Debug, Info, Warning, Error, Critical
- Logs automatically cleaned up after 30 days

---

### Phase 2: Enhanced Management (v0.6.0)
**Target Date:** 2026-04-15  
**Priority:** Medium

#### 4. Provider Management API
- [ ] Restore repo_providers table
- [ ] Add provider CRUD endpoints
- [ ] Implement provider validation
- [ ] Add provider sync functionality

**Success Criteria:**
- Multiple providers supported
- Provider tokens validated
- Automatic repository sync

#### 5. Workspace Management API
- [ ] Restore workspace CRUD endpoints
- [ ] Add container restart endpoint
- [ ] Implement resource statistics
- [ ] Add workspace health monitoring

**Success Criteria:**
- Workspaces manageable via API
- Container health tracked
- Resource usage monitored

#### 6. Agent Management API
- [ ] Restore agent CRUD endpoints
- [ ] Support multiple agents per workspace
- [ ] Add agent enable/disable functionality
- [ ] Implement agent performance tracking

**Success Criteria:**
- Multiple agents per workspace
- Agents manageable via API
- Agent performance metrics available

---

### Phase 3: Advanced Features (v0.7.0)
**Target Date:** 2026-05-15  
**Priority:** Medium

#### 7. Issue Polling Service
- [ ] Restore issue polling service
- [ ] Add polling configuration per repository
- [ ] Implement intelligent filtering
- [ ] Add dual-mode operation (webhook + polling)

**Success Criteria:**
- Automatic issue synchronization
- Configurable polling intervals
- Label and mention filtering

#### 8. Task Failure Analyzer
- [ ] Restore failure analysis service
- [ ] Implement 9 failure categories
- [ ] Add context-aware recommendations
- [ ] Track recurring failures

**Success Criteria:**
- Automatic failure categorization
- Actionable recommendations
- Recurring failure detection

#### 9. Init Script Feature
- [ ] Restore init_scripts table
- [ ] Add init script CRUD endpoints
- [ ] Implement script execution service
- [ ] Add script log storage

**Success Criteria:**
- Custom init scripts supported
- Scripts executed on container startup
- Script logs accessible

---

### Phase 4: Multi-Provider Support (v0.8.0)
**Target Date:** 2026-06-15  
**Priority:** High

#### 10. Gitea Provider Support
- [ ] Implement Gitea client
- [ ] Add Gitea webhook support
- [ ] Test with Gitea instances
- [ ] Update documentation

**Success Criteria:**
- Full feature parity with GitHub
- All tests passing
- Documentation updated

#### 11. GitLab Provider Support
- [ ] Implement GitLab client
- [ ] Add GitLab webhook support
- [ ] Test with GitLab instances
- [ ] Update documentation

**Success Criteria:**
- Full feature parity with GitHub
- All tests passing
- Documentation updated

---

### Phase 5: Production Ready (v1.0.0)
**Target Date:** 2026-09-01  
**Priority:** High

#### 12. Stability & Performance
- [ ] API stability guaranteed (no breaking changes)
- [ ] Performance optimization
- [ ] Security hardening
- [ ] Comprehensive documentation
- [ ] Production deployment guide

**Success Criteria:**
- 99%+ uptime in production
- API response time <100ms (p95)
- Security audit passed
- Complete documentation

#### 13. Web UI Dashboard
- [ ] Design UI/UX
- [ ] Implement frontend framework
- [ ] Create dashboard views
- [ ] Add real-time updates
- [ ] Implement user authentication

**Success Criteria:**
- Intuitive web interface
- Real-time task monitoring
- User authentication working

---

## 🎯 Milestones

### Milestone 1: Simplified MVP (v0.4.0-mvp)
**Target Date:** 2026-02-06  
**Status:** ✅ COMPLETED

- ✅ Core Issue-to-PR automation
- ✅ Simplified architecture
- ✅ Environment-based configuration
- ✅ Single agent per workspace
- ✅ Basic task execution
- ✅ PR creation and issue closure

**Success Criteria:**
- ✅ End-to-end automation working
- ✅ Simplified codebase (23% reduction)
- ✅ 99.4% test pass rate (336/338 tests)
- ✅ Clean compilation (0 errors)

### Milestone 2: Essential Features Restored (v0.5.0)
**Target Date:** 2026-03-15

- 🟡 Task retry mechanism
- 🟡 Task execution history
- 🟡 Real-time log streaming

**Success Criteria:**
- Retry mechanism working
- Execution history tracked
- WebSocket logs streaming

### Milestone 3: Full Feature Parity (v0.7.0)
**Target Date:** 2026-05-15

- 🟡 All v0.3.0 features restored
- 🟡 Improved architecture
- 🟡 Better performance

**Success Criteria:**
- Feature parity with v0.3.0
- Improved code quality
- Better test coverage

### Milestone 4: Production Ready (v1.0.0)
**Target Date:** 2026-09-01

- 🟡 Stable API (no breaking changes)
- 🟡 Multi-provider support
- 🟡 Web UI dashboard
- 🟡 Production deployment guide

**Success Criteria:**
- API stability guaranteed
- 99%+ uptime in production
- Security audit passed
- Performance benchmarks met

---

## 📈 Performance Goals

### Current Performance (v0.4.0-mvp)
- Task execution: ~45 seconds average
- Concurrent tasks: 1 per workspace (simplified)
- Test coverage: 280+ unit tests (100% passing)
- API endpoints: 10 core endpoints

### Target Performance (v0.5.0)
- Task execution: ~40 seconds average
- Concurrent tasks: 3 per workspace
- Test coverage: 350+ tests (100% passing)
- API endpoints: 20+ endpoints

### Target Performance (v1.0.0)
- Task execution: <30 seconds average
- Concurrent tasks: 10 per workspace
- Test coverage: 500+ tests (100% passing)
- API response time: <100ms (p95)
- Database query time: <10ms (p95)

---

## 🔄 Version History

| Version | Date | Key Features |
|---------|------|--------------|
| v0.4.0-mvp | 2026-02-06 | Simplified MVP: Core automation, environment-based config, single agent per workspace, inline logs |
| v0.3.0 | 2026-01-20 | Container Lifecycle, Issue Polling, Task Automation, Task Scheduler, Concurrency Control, Real-time Logs, Execution History, Failure Analysis, PR Creation, Issue Closure |
| v0.2.0 | 2026-01-19 | Workspace API, Init Scripts, Agent Management |
| v0.1.0 | 2026-01-17 | Initial release with RepoProvider, Repository, Webhook APIs |

---

## 🤝 Contributing to Roadmap

We welcome community input on the roadmap! To suggest features or changes:

1. Open a GitHub Issue with the `roadmap` label
2. Describe the feature and its use case
3. Explain the expected impact
4. Discuss implementation approach (optional)

---

## 📚 Related Documentation

- **[Implementation Plans](../plans/)** - Detailed implementation plans for features
- **[Design Documents](../design/)** - Feature design and architecture decisions
- **[API Documentation](../api/)** - API specifications and usage guides
- **[Migration Guide](../../MIGRATION.md)** - Migrating from v0.3.0 to v0.4.0-mvp

---

**Maintained By:** VibeRepo Team  
**Next Review:** 2026-03-01
