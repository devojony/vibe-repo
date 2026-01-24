# End-to-End Testing Guide

This guide covers the E2E testing infrastructure for VibeRepo, which tests the complete Issue-to-PR workflow using a real Gitea instance.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Running Tests](#running-tests)
- [Available Test Cases](#available-test-cases)
- [Test Structure](#test-structure)
- [Writing New E2E Tests](#writing-new-e2e-tests)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Overview

E2E (End-to-End) tests validate the complete VibeRepo workflow by:

1. Creating real repositories in Gitea
2. Configuring VibeRepo to manage those repositories
3. Creating issues and tasks
4. Executing automated workflows
5. Verifying PRs are created correctly
6. Cleaning up all test resources

These tests provide confidence that the entire system works together correctly in a production-like environment.

**Key Features:**
- Tests against real Gitea instance (https://gitea.devo.top:66)
- Automatic resource cleanup after each test
- WebSocket log streaming validation
- Complete Issue-to-PR workflow verification
- Docker container lifecycle testing

## Prerequisites

Before running E2E tests, ensure you have:

### 1. VibeRepo Backend Running

The backend must be running locally:

```bash
cd backend
cargo run
```

Verify it's running:
```bash
curl http://localhost:3000/health
```

### 2. Docker Daemon Running

E2E tests create Docker containers for workspaces:

```bash
# Check Docker is running
docker info

# If not running, start Docker Desktop or daemon
```

### 3. Gitea Instance Access

Tests use a real Gitea instance at `https://gitea.devo.top:66`.

**Configuration:**
- Base URL: `https://gitea.devo.top:66`
- Access Token: Configured in `backend/tests/e2e/tests.rs`
- Test User: `devo`

**Note:** The Gitea instance must be accessible from your machine. If you see connection warnings, tests may fail.

### 4. Environment Setup

Ensure your `.env` file in the `backend/` directory is configured:

```bash
DATABASE_URL=sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug
```

## Running Tests

### Using the Test Runner Script (Recommended)

The easiest way to run E2E tests is using the provided script:

```bash
# Run all E2E tests
./scripts/run_e2e_tests.sh

# Run a specific test
./scripts/run_e2e_tests.sh test_e2e_repository_setup

# Run with additional cargo test flags
./scripts/run_e2e_tests.sh "" --test-threads=1
```

The script automatically:
- Checks if the backend is running
- Verifies Docker is available
- Warns if Gitea is not accessible
- Runs tests with proper flags

### Using Cargo Directly

You can also run tests directly with cargo:

```bash
cd backend

# Run all E2E tests
cargo test --test e2e -- --ignored --nocapture

# Run a specific test
cargo test --test e2e test_e2e_repository_setup -- --ignored --nocapture

# Run tests sequentially (recommended for E2E)
cargo test --test e2e -- --ignored --nocapture --test-threads=1
```

**Important Flags:**
- `--test e2e` - Run only the E2E test suite
- `--ignored` - E2E tests are marked with `#[ignore]` to prevent accidental runs
- `--nocapture` - Show test output (recommended for debugging)
- `--test-threads=1` - Run tests sequentially to avoid resource conflicts

## Available Test Cases

### 1. Repository Setup Test

**Test:** `test_e2e_repository_setup`

**Purpose:** Validates the basic repository setup workflow.

**What it tests:**
- Creating a repository in Gitea
- Registering a provider in VibeRepo
- Syncing repositories from Gitea
- Initializing repository with branch and labels

**Duration:** ~10-15 seconds

**Run:**
```bash
./scripts/run_e2e_tests.sh test_e2e_repository_setup
```

### 2. Workspace Setup Test

**Test:** `test_e2e_workspace_setup`

**Purpose:** Validates workspace and agent creation.

**What it tests:**
- All steps from Repository Setup Test
- Creating a Docker-based workspace
- Running initialization scripts
- Creating an AI agent configuration
- Verifying workspace is running

**Duration:** ~20-30 seconds

**Run:**
```bash
./scripts/run_e2e_tests.sh test_e2e_workspace_setup
```

### 3. Complete Issue-to-PR Workflow Test

**Test:** `test_e2e_complete_issue_to_pr_workflow`

**Purpose:** Validates the complete end-to-end workflow from issue creation to PR.

**What it tests:**
- All setup steps (repository, workspace, agent)
- Creating an issue in Gitea
- Creating a task from the issue
- Assigning agent to task
- Executing the task
- Waiting for task completion
- Verifying PR was created
- Verifying PR references the issue
- Cleanup (closing PR, deleting branch)

**Duration:** ~10-15 minutes (includes AI agent execution)

**Run:**
```bash
./scripts/run_e2e_tests.sh test_e2e_complete_issue_to_pr_workflow
```

**Note:** This is the most comprehensive test and takes the longest to run.

### 4. WebSocket Log Monitoring Test

**Test:** `test_e2e_websocket_log_monitoring`

**Purpose:** Validates real-time log streaming via WebSocket.

**What it tests:**
- All setup steps
- Creating a task
- Connecting to WebSocket log stream
- Executing task while monitoring logs
- Receiving log messages in real-time
- Verifying log message format

**Duration:** ~2-3 minutes

**Run:**
```bash
./scripts/run_e2e_tests.sh test_e2e_websocket_log_monitoring
```

## Test Structure

### Test Phases

Each E2E test follows a structured approach with distinct phases:

#### Phase 1: Setup
- Create Gitea repository
- Register provider in VibeRepo
- Sync repositories
- Initialize repository

#### Phase 2: Workspace Setup (if needed)
- Create Docker workspace
- Run initialization scripts
- Create AI agent

#### Phase 3: Task Creation (if needed)
- Create issue in Gitea
- Create task from issue
- Assign agent to task

#### Phase 4: Execution (if needed)
- Execute task
- Monitor progress
- Wait for completion

#### Phase 5: Verification
- Verify expected outcomes
- Check PR creation
- Validate data integrity

#### Phase 6: Cleanup
- Delete workspace and containers
- Delete repository from VibeRepo
- Delete provider
- Delete Gitea repository

### Test Context

All tests use a `TestContext` struct that manages:

```rust
struct TestContext {
    gitea_client: Arc<GiteaClient>,      // Gitea API client
    vibe_client: Arc<Client>,            // VibeRepo API client
    test_repo_name: String,              // Unique test repo name
    test_repo_owner: String,             // Gitea user
    provider_id: Option<i32>,            // Created provider ID
    repository_id: Option<i32>,          // Created repository ID
    workspace_id: Option<i32>,           // Created workspace ID
    agent_id: Option<i32>,               // Created agent ID
    task_id: Option<i32>,                // Created task ID
}
```

### Helper Methods

The `TestContext` provides helper methods for each phase:

- `setup_gitea_repository()` - Create repo in Gitea
- `setup_vibe_provider()` - Register provider
- `sync_repositories()` - Sync repos from provider
- `initialize_repository()` - Initialize with branch/labels
- `create_workspace()` - Create Docker workspace
- `create_agent()` - Create AI agent
- `create_task()` - Create task from issue
- `assign_agent_to_task()` - Assign agent
- `execute_task()` - Start task execution
- `wait_for_task_completion()` - Wait for task to finish
- `monitor_task_logs()` - Monitor via WebSocket
- `cleanup()` - Clean up all resources

## Writing New E2E Tests

### Step 1: Define Your Test

Decide what workflow you want to test. E2E tests should validate complete user workflows, not individual API endpoints.

### Step 2: Create Test Function

Add a new test function in `backend/tests/e2e/tests.rs`:

```rust
#[tokio::test]
#[ignore] // Always mark E2E tests as ignored
async fn test_e2e_your_workflow() {
    let mut ctx = TestContext::new("your-workflow");
    
    // Your test implementation
}
```

### Step 3: Implement Test Phases

Use the helper methods to implement each phase:

```rust
#[tokio::test]
#[ignore]
async fn test_e2e_your_workflow() {
    let mut ctx = TestContext::new("your-workflow");
    
    // Phase 1: Setup
    println!("\n=== Phase 1: Setup ===");
    ctx.setup_gitea_repository().await.expect("Failed to create Gitea repository");
    ctx.setup_vibe_provider().await.expect("Failed to create VibeRepo provider");
    ctx.sync_repositories().await.expect("Failed to sync repositories");
    
    // Phase 2: Your specific workflow
    println!("\n=== Phase 2: Your Workflow ===");
    // ... your test logic ...
    
    // Phase 3: Verification
    println!("\n=== Phase 3: Verification ===");
    // ... assertions ...
    
    // Phase 4: Cleanup
    println!("\n=== Phase 4: Cleanup ===");
    ctx.cleanup().await.expect("Failed to cleanup");
    
    println!("\n✅ Your workflow test passed");
}
```

### Step 4: Add Cleanup

**Always** call `ctx.cleanup()` at the end, even if the test fails. Consider using a defer pattern or ensuring cleanup runs:

```rust
// Option 1: Use expect with cleanup
ctx.cleanup().await.expect("Failed to cleanup");

// Option 2: Use a guard (more advanced)
struct CleanupGuard<'a>(&'a TestContext);
impl<'a> Drop for CleanupGuard<'a> {
    fn drop(&mut self) {
        // Cleanup logic
    }
}
```

### Step 5: Test Your Test

Run your new test:

```bash
./scripts/run_e2e_tests.sh test_e2e_your_workflow
```

### Example: Testing Webhook Integration

```rust
#[tokio::test]
#[ignore]
async fn test_e2e_webhook_trigger() {
    let mut ctx = TestContext::new("webhook-trigger");
    
    // Setup
    ctx.setup_gitea_repository().await.expect("Setup failed");
    ctx.setup_vibe_provider().await.expect("Provider failed");
    ctx.sync_repositories().await.expect("Sync failed");
    ctx.initialize_repository().await.expect("Init failed");
    
    // Configure webhook
    let repository_id = ctx.repository_id.expect("No repo ID");
    let webhook_url = format!("{}/api/webhooks/gitea", VIBE_REPO_BASE_URL);
    
    let response = ctx.vibe_client
        .post(&format!("{}/api/repositories/{}/webhook", VIBE_REPO_BASE_URL, repository_id))
        .json(&json!({
            "webhook_url": webhook_url,
            "secret": "test-secret",
        }))
        .send()
        .await
        .expect("Failed to configure webhook");
    
    assert!(response.status().is_success());
    
    // Create issue (should trigger webhook)
    let issue = ctx.gitea_client
        .create_issue(&ctx.test_repo_owner, &ctx.test_repo_name, "Test", "Body", vec![])
        .await
        .expect("Failed to create issue");
    
    // Wait for webhook to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // Verify task was created automatically
    let tasks_response = ctx.vibe_client
        .get(&format!("{}/api/tasks", VIBE_REPO_BASE_URL))
        .send()
        .await
        .expect("Failed to get tasks");
    
    let tasks: Vec<serde_json::Value> = tasks_response.json().await.expect("Parse failed");
    let task = tasks.iter().find(|t| t["issue_number"].as_i64() == Some(issue.number));
    
    assert!(task.is_some(), "Task should be created automatically via webhook");
    
    // Cleanup
    ctx.cleanup().await.expect("Cleanup failed");
    
    println!("✅ Webhook trigger test passed");
}
```

## Troubleshooting

### Backend Not Running

**Error:**
```
❌ VibeRepo backend is not running at http://localhost:3000
```

**Solution:**
```bash
cd backend
cargo run
```

Wait for the server to start, then run tests again.

### Docker Not Running

**Error:**
```
❌ Docker is not running
```

**Solution:**
- Start Docker Desktop (macOS/Windows)
- Start Docker daemon (Linux): `sudo systemctl start docker`

### Gitea Connection Failed

**Warning:**
```
⚠ Warning: Cannot connect to Gitea instance
```

**Possible Causes:**
1. Gitea instance is down
2. Network connectivity issues
3. SSL certificate issues
4. Firewall blocking connection

**Solutions:**
- Verify Gitea is accessible: `curl -k https://gitea.devo.top:66/api/v1/version`
- Check network connection
- Verify VPN/proxy settings
- Check firewall rules

### Test Timeout

**Error:**
```
Task did not complete in time
```

**Possible Causes:**
1. AI agent taking longer than expected
2. Docker container startup issues
3. Network latency

**Solutions:**
- Increase timeout in test: `ctx.wait_for_task_completion(1200)` (20 minutes)
- Check Docker container logs
- Verify workspace initialization completed
- Check agent configuration

### Resource Cleanup Failed

**Error:**
```
Failed to cleanup: ...
```

**Impact:** May leave test resources (repos, containers) behind.

**Solutions:**
1. **Manual Cleanup:**
   ```bash
   # List and remove test containers
   docker ps -a | grep vibe-repo-test
   docker rm -f <container-id>
   
   # Delete test repositories in Gitea
   # (via Gitea web UI or API)
   ```

2. **Database Cleanup:**
   ```bash
   # If using SQLite
   cd backend
   rm -f data/vibe-repo/db/vibe-repo.db
   cargo run  # Will recreate database
   ```

3. **Check Logs:**
   ```bash
   # Check backend logs for errors
   cd backend
   RUST_LOG=debug cargo run
   ```

### WebSocket Connection Failed

**Error:**
```
Failed to connect to WebSocket: ...
```

**Solutions:**
- Verify backend is running
- Check WebSocket endpoint: `ws://localhost:3000/api/tasks/{task_id}/logs/stream`
- Verify task ID is valid
- Check for firewall blocking WebSocket connections

### Test Flakiness

**Symptom:** Tests pass sometimes but fail other times.

**Common Causes:**
1. **Race Conditions:** Async operations not properly awaited
2. **Resource Conflicts:** Multiple tests running in parallel
3. **External Dependencies:** Gitea or Docker issues

**Solutions:**
1. **Run Tests Sequentially:**
   ```bash
   ./scripts/run_e2e_tests.sh "" --test-threads=1
   ```

2. **Add Delays:**
   ```rust
   // Wait for async operations to complete
   tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
   ```

3. **Use Proper Waiting:**
   ```rust
   // Instead of fixed delays, use wait_for_condition
   wait_for_condition(
       || async { /* check condition */ },
       timeout_secs,
       check_interval_ms,
   ).await?;
   ```

## Best Practices

### 1. Always Use Unique Test Names

Each test should have a unique name to avoid resource conflicts:

```rust
let mut ctx = TestContext::new("unique-test-name");
```

The test name is used to generate unique repository names with timestamps.

### 2. Always Clean Up Resources

**Always** call `ctx.cleanup()` at the end of tests:

```rust
// At the end of every test
ctx.cleanup().await.expect("Failed to cleanup");
```

This prevents resource leaks and ensures tests don't interfere with each other.

### 3. Use Descriptive Phase Labels

Print clear phase labels to make test output readable:

```rust
println!("\n=== Phase 1: Setup ===");
// ... setup code ...

println!("\n=== Phase 2: Execution ===");
// ... execution code ...
```

### 4. Add Verification Steps

Don't just create resources - verify they were created correctly:

```rust
// Create workspace
ctx.create_workspace().await.expect("Failed to create workspace");

// Verify it exists and is running
let workspace_id = ctx.workspace_id.expect("Workspace ID not set");
let response = ctx.vibe_client
    .get(&format!("{}/api/workspaces/{}", VIBE_REPO_BASE_URL, workspace_id))
    .send()
    .await
    .expect("Failed to get workspace");

let workspace: serde_json::Value = response.json().await.expect("Parse failed");
assert_eq!(workspace["status"].as_str(), Some("Running"));
```

### 5. Use Appropriate Timeouts

Set realistic timeouts based on what the test does:

- Repository setup: 10-30 seconds
- Workspace creation: 30-60 seconds
- Task execution: 5-15 minutes
- WebSocket monitoring: 1-5 minutes

```rust
// Short timeout for simple operations
tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

// Long timeout for AI agent execution
ctx.wait_for_task_completion(600).await?; // 10 minutes
```

### 6. Handle Errors Gracefully

Provide clear error messages:

```rust
ctx.setup_gitea_repository()
    .await
    .expect("Failed to create Gitea repository - check Gitea connectivity");
```

### 7. Test One Workflow Per Test

Each test should focus on one complete workflow:

- ✅ Good: `test_e2e_complete_issue_to_pr_workflow`
- ❌ Bad: `test_e2e_everything`

### 8. Use Helper Methods

Leverage the `TestContext` helper methods instead of duplicating code:

```rust
// ✅ Good
ctx.create_workspace().await?;

// ❌ Bad - duplicating logic
let response = ctx.vibe_client
    .post(&format!("{}/api/workspaces", VIBE_REPO_BASE_URL))
    .json(&json!({ /* ... */ }))
    .send()
    .await?;
// ... more code ...
```

### 9. Run Tests Sequentially

E2E tests can conflict if run in parallel. Use `--test-threads=1`:

```bash
./scripts/run_e2e_tests.sh "" --test-threads=1
```

### 10. Document Your Tests

Add clear documentation to new tests:

```rust
/// Tests the complete webhook-triggered workflow
///
/// This test verifies that:
/// 1. Webhooks can be configured for a repository
/// 2. Creating an issue triggers the webhook
/// 3. A task is automatically created from the webhook
/// 4. The task can be executed successfully
#[tokio::test]
#[ignore]
async fn test_e2e_webhook_workflow() {
    // ...
}
```

## Test Environment Configuration

### Gitea Configuration

The tests use the following Gitea configuration:

```rust
const GITEA_BASE_URL: &str = "https://gitea.devo.top:66";
const GITEA_TOKEN: &str = "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2";
```

**To use a different Gitea instance:**

1. Update constants in `backend/tests/e2e/tests.rs`
2. Ensure the access token has permissions to:
   - Create repositories
   - Create issues
   - Create pull requests
   - Delete repositories

### VibeRepo Configuration

Tests connect to the local VibeRepo instance:

```rust
const VIBE_REPO_BASE_URL: &str = "http://localhost:3000";
```

Ensure your backend is running on port 3000 or update this constant.

### Docker Configuration

Tests create Docker containers with:
- Image: `ubuntu:22.04`
- Init script timeout: 300 seconds
- Agent timeout: 600 seconds

Adjust these in the test code if needed.

## Continuous Integration

### Running E2E Tests in CI

E2E tests can be run in CI pipelines with proper setup:

```yaml
# Example GitHub Actions workflow
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    
    services:
      docker:
        image: docker:dind
    
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Start Backend
        run: |
          cd backend
          cargo run &
          sleep 10
      
      - name: Run E2E Tests
        run: ./scripts/run_e2e_tests.sh
        env:
          GITEA_BASE_URL: ${{ secrets.GITEA_BASE_URL }}
          GITEA_TOKEN: ${{ secrets.GITEA_TOKEN }}
```

**Note:** You'll need to:
1. Set up Gitea access in CI environment
2. Ensure Docker is available
3. Configure secrets for Gitea credentials

## Summary

E2E tests provide comprehensive validation of VibeRepo's complete workflow. Key points:

- ✅ Always run with backend and Docker running
- ✅ Use the test runner script for convenience
- ✅ Tests automatically clean up resources
- ✅ Run tests sequentially to avoid conflicts
- ✅ Each test validates a complete user workflow
- ✅ WebSocket log streaming is fully tested

For questions or issues, see the [main documentation](../README.md) or open an issue on GitHub.
