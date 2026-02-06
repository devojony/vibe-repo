# End-to-End Functional Testing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create comprehensive end-to-end functional tests that validate the complete Issue-to-PR workflow using a real Gitea instance.

**Architecture:** Integration tests that interact with actual Gitea API, VibeRepo backend, Docker containers, and WebSocket connections to verify the entire automation pipeline from issue creation to PR generation.

**Tech Stack:** 
- Rust (cargo test for integration tests)
- Gitea API (https://gitea.devo.top:66/)
- Docker (for workspace containers)
- WebSocket (for real-time log monitoring)
- curl/reqwest (for API interactions)

---

## Test Environment Configuration

**Gitea Instance:**
- Base URL: `https://gitea.devo.top:66/`
- API Token: `fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2`
- Network: Same LAN as VibeRepo backend

**Prerequisites:**
- VibeRepo backend running at `http://localhost:3000`
- Docker daemon accessible
- Network connectivity to Gitea instance
- Test repository created in Gitea (will be created in setup)

---

## Test Workflow Overview

The E2E test will validate this complete flow:

1. **Setup Phase**
   - Create test repository in Gitea
   - Configure VibeRepo with Gitea provider
   - Initialize repository with branch and labels
   - Create workspace with Docker container
   - Configure AI agent

2. **Execution Phase**
   - Create issue in Gitea with specific label
   - Trigger task creation (manual or via polling)
   - Monitor task execution via WebSocket
   - Verify container execution
   - Verify PR creation in Gitea

3. **Verification Phase**
   - Verify PR exists with correct title/body
   - Verify PR links to issue
   - Verify branch was created
   - Verify task status updated correctly
   - Verify execution logs captured

4. **Cleanup Phase**
   - Delete test PR
   - Delete test branch
   - Close/delete test issue
   - Remove workspace and container
   - Remove test repository

---

## Implementation Tasks

See the detailed implementation plan in the backup file for complete code examples.

### Task 1: Create E2E Test Infrastructure
- Create test module structure (`backend/tests/e2e/mod.rs`)
- Create Gitea API client (`backend/tests/e2e/gitea_client.rs`)
- Create test helpers (`backend/tests/e2e/helpers.rs`)

### Task 2: Create E2E Test Setup Phase
- Create TestContext struct
- Implement setup methods (repository, provider, sync)
- Implement cleanup method

### Task 3: Implement Repository Initialization Test
- Add repository initialization method
- Write first E2E test for repository setup
- Verify initialization works

### Task 4: Implement Workspace and Agent Setup
- Add workspace creation method
- Add agent creation method
- Write workspace setup test

### Task 5: Implement Complete Issue-to-PR Workflow Test
- Add task creation and execution methods
- Add wait for completion method
- Write complete workflow test
- Verify PR creation in Gitea

### Task 6: Add WebSocket Log Monitoring Test
- Add WebSocket dependencies
- Add log monitoring method
- Write WebSocket monitoring test
- Make TestContext cloneable

### Task 7: Create Test Runner Script and Documentation
- Create `scripts/run_e2e_tests.sh`
- Create `docs/testing/e2e-testing.md`
- Update README.md

---

## Summary

This implementation plan creates a comprehensive E2E testing framework for VibeRepo that:

✅ **Tests the complete workflow** - From issue creation to PR generation  
✅ **Uses real Gitea instance** - Tests against actual Git provider API  
✅ **Validates Docker integration** - Tests workspace and container management  
✅ **Monitors WebSocket logs** - Tests real-time log streaming  
✅ **Cleans up resources** - Prevents resource leaks and test pollution  
✅ **Provides documentation** - Complete guide for running and writing tests  

### Test Coverage

| Test | Duration | Coverage |
|------|----------|----------|
| Repository Setup | ~10s | Provider, repository, initialization |
| Workspace Setup | ~30s | Workspace, agent, container |
| Complete Workflow | ~5-10min | Full Issue-to-PR automation |
| WebSocket Monitoring | ~2min | Real-time log streaming |

### File Structure

```
backend/
├── tests/
│   └── e2e/
│       ├── mod.rs              # Module definition
│       ├── helpers.rs          # Test utilities
│       ├── gitea_client.rs     # Gitea API client
│       └── tests.rs            # Test cases
├── Cargo.toml                  # Dependencies
docs/
├── testing/
│   └── e2e-testing.md          # E2E testing guide
└── plans/
    ├── 2026-01-24-e2e-functional-testing.md         # This plan
    └── 2026-01-24-e2e-functional-testing.md.backup  # Detailed version
scripts/
└── run_e2e_tests.sh            # Test runner script
README.md                        # Updated with E2E info
```

### Running the Tests

After implementation:

```bash
# 1. Start backend
cd backend && cargo run

# 2. Run all E2E tests
./scripts/run_e2e_tests.sh

# 3. Run specific test
./scripts/run_e2e_tests.sh test_e2e_complete_issue_to_pr_workflow
```

### Next Steps

After completing this plan:

1. **Update Gitea username** - Replace `"your-gitea-username"` in `TestContext::new()`
2. **Configure CI/CD** - Add E2E tests to nightly builds
3. **Add more scenarios** - Test error cases, retries, webhooks
4. **Performance testing** - Add metrics collection
5. **Multi-provider testing** - Test with GitHub/GitLab

### Dependencies

**Required:**
- Rust 1.70+
- Docker
- Gitea instance (https://gitea.devo.top:66/)
- Network connectivity

**Optional:**
- WebSocket auth token (for authenticated testing)

---

## Execution Options

Plan complete and saved to `docs/plans/2026-01-24-e2e-functional-testing.md`.

**Two execution options:**

### 1. Subagent-Driven (this session)
- I dispatch fresh subagent per task
- Review between tasks
- Fast iteration
- Good for: Interactive development, debugging

**To proceed:** Say "use subagent-driven approach"

### 2. Parallel Session (separate)
- Open new session with executing-plans
- Batch execution with checkpoints
- Autonomous execution
- Good for: Unattended execution, CI/CD

**To proceed:** 
1. Open new Claude Code session
2. Navigate to worktree
3. Say "use superpowers:executing-plans with docs/plans/2026-01-24-e2e-functional-testing.md"

**Which approach would you like to use?**

---

**Note:** The detailed implementation with complete code examples is available in the backup file:
`docs/plans/2026-01-24-e2e-functional-testing.md.backup`
