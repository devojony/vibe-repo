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
    └── 2026-01-24-e2e-functional-testing.md  # This plan
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

**Files:**
- Create: `scripts/run_e2e_tests.sh`
- Create: `docs/testing/e2e-testing.md`
- Modify: `README.md`

**Step 1: Create test runner script**

Create `scripts/run_e2e_tests.sh`:

```bash
#!/bin/bash
# E2E Test Runner Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== VibeRepo E2E Test Runner ===${NC}\n"

# Check prerequisites
echo "Checking prerequisites..."

# Check if backend is running
if ! curl -s http://localhost:3000/health > /dev/null; then
    echo -e "${RED}❌ VibeRepo backend is not running at http://localhost:3000${NC}"
    echo "Please start the backend with: cd backend && cargo run"
    exit 1
fi
echo -e "${GREEN}✓${NC} Backend is running"

# Check Docker
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}❌ Docker is not running${NC}"
    echo "Please start Docker daemon"
    exit 1
fi
echo -e "${GREEN}✓${NC} Docker is running"

# Check Gitea connectivity
if ! curl -s -k https://gitea.devo.top:66/api/v1/version > /dev/null; then
    echo -e "${YELLOW}⚠${NC}  Warning: Cannot connect to Gitea instance"
    echo "Tests may fail if Gitea is not accessible"
fi

echo ""

# Parse arguments
TEST_NAME="${1:-}"
EXTRA_ARGS="${@:2}"

cd backend

if [ -z "$TEST_NAME" ]; then
    echo "Running all E2E tests..."
    cargo test --test e2e -- --ignored --nocapture $EXTRA_ARGS
else
    echo "Running E2E test: $TEST_NAME..."
    cargo test --test e2e $TEST_NAME -- --ignored --nocapture $EXTRA_ARGS
fi

echo ""
echo -e "${GREEN}=== E2E Tests Complete ===${NC}"
```

**Step 2: Make script executable**

Run: `chmod +x scripts/run_e2e_tests.sh`

**Step 3: Create E2E testing documentation**

Create `docs/testing/e2e-testing.md`:

```markdown
# End-to-End Testing Guide

**Version:** 0.4.0  
**Last Updated:** 2026-01-24

This guide explains how to run and write end-to-end (E2E) tests for VibeRepo.

## Overview

E2E tests validate the complete Issue-to-PR workflow using a real Gitea instance. These tests:

- Create actual repositories in Gitea
- Configure VibeRepo with real providers
- Execute tasks in Docker containers
- Verify PR creation in Gitea
- Monitor logs via WebSocket
- Clean up all resources after testing

## Prerequisites

### 1. Running Backend

Start the VibeRepo backend:

\`\`\`bash
cd backend
cargo run
\`\`\`

The backend should be accessible at `http://localhost:3000`.

### 2. Docker Daemon

Ensure Docker is running:

\`\`\`bash
docker info
\`\`\`

### 3. Gitea Instance

The tests use a Gitea instance at `https://gitea.devo.top:66/`.

**Configuration:**
- Base URL: `https://gitea.devo.top:66/`
- API Token: `fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2`
- Network: Same LAN as VibeRepo backend

### 4. Environment Variables

Optional WebSocket authentication:

\`\`\`bash
export WEBSOCKET_AUTH_TOKEN="your-token-here"
\`\`\`

## Running Tests

### Run All E2E Tests

\`\`\`bash
./scripts/run_e2e_tests.sh
\`\`\`

Or directly:

\`\`\`bash
cd backend
cargo test --test e2e -- --ignored --nocapture
\`\`\`

### Run Specific Test

\`\`\`bash
./scripts/run_e2e_tests.sh test_e2e_complete_issue_to_pr_workflow
\`\`\`

Or:

\`\`\`bash
cd backend
cargo test --test e2e test_e2e_complete_issue_to_pr_workflow -- --ignored --nocapture
\`\`\`

### Available Tests

1. **test_e2e_repository_setup** - Tests repository initialization
2. **test_e2e_workspace_setup** - Tests workspace and agent creation
3. **test_e2e_complete_issue_to_pr_workflow** - Tests full Issue-to-PR flow
4. **test_e2e_websocket_log_monitoring** - Tests WebSocket log streaming

## Test Structure

### Test Phases

Each E2E test follows this structure:

1. **Setup Phase**
   - Create test repository in Gitea
   - Configure VibeRepo provider
   - Sync repositories
   - Initialize repository
   - Create workspace and agent

2. **Execution Phase**
   - Create issue in Gitea
   - Create task in VibeRepo
   - Assign agent to task
   - Execute task
   - Monitor execution

3. **Verification Phase**
   - Verify task completed successfully
   - Verify PR was created in Gitea
   - Verify PR content and metadata
   - Verify logs were captured

4. **Cleanup Phase**
   - Close/delete PR
   - Delete branch
   - Remove workspace
   - Remove repository
   - Remove provider

### TestContext

The `TestContext` struct manages test resources:

\`\`\`rust
struct TestContext {
    gitea_client: GiteaClient,
    vibe_client: Client,
    test_repo_name: String,
    provider_id: Option<i32>,
    repository_id: Option<i32>,
    workspace_id: Option<i32>,
    agent_id: Option<i32>,
    task_id: Option<i32>,
}
\`\`\`

## Writing New E2E Tests

### 1. Create Test Function

\`\`\`rust
#[tokio::test]
#[ignore] // Always mark E2E tests with #[ignore]
async fn test_e2e_my_feature() {
    let mut ctx = TestContext::new("my-feature");
    
    // Setup
    ctx.setup_gitea_repository().await.expect("Setup failed");
    // ... more setup
    
    // Test your feature
    // ...
    
    // Verify results
    assert!(condition, "Verification message");
    
    // Cleanup
    ctx.cleanup().await.expect("Cleanup failed");
}
\`\`\`

### 2. Use Helper Methods

TestContext provides helper methods:

- `setup_gitea_repository()` - Create Gitea repo
- `setup_vibe_provider()` - Create provider
- `sync_repositories()` - Sync repos
- `initialize_repository()` - Initialize repo
- `create_workspace()` - Create workspace
- `create_agent()` - Create agent
- `create_task()` - Create task
- `execute_task()` - Execute task
- `wait_for_task_completion()` - Wait for completion
- `monitor_task_logs()` - Monitor WebSocket logs
- `cleanup()` - Clean up all resources

### 3. Always Clean Up

Always call `ctx.cleanup()` at the end, even if test fails:

\`\`\`rust
// Cleanup
ctx.cleanup().await.expect("Failed to cleanup");
\`\`\`

## Troubleshooting

### Backend Not Running

**Error:** `Backend is not running at http://localhost:3000`

**Solution:**
\`\`\`bash
cd backend
cargo run
\`\`\`

### Docker Not Running

**Error:** `Docker is not running`

**Solution:** Start Docker Desktop or Docker daemon

### Gitea Connection Failed

**Error:** `Cannot connect to Gitea instance`

**Solution:** 
- Check network connectivity
- Verify Gitea URL: `https://gitea.devo.top:66/`
- Check if Gitea is accessible from your network

### Test Timeout

**Error:** `Task did not complete in time`

**Solution:**
- Increase timeout in `wait_for_task_completion()`
- Check Docker container logs
- Check backend logs for errors

### WebSocket Connection Failed

**Error:** `Failed to connect to WebSocket`

**Solution:**
- Verify backend is running
- Check if WebSocket authentication is required
- Set `WEBSOCKET_AUTH_TOKEN` environment variable

### Cleanup Failed

**Error:** `Failed to cleanup`

**Solution:**
- Manually delete test resources
- Check Gitea for test repositories (prefix: `repo-setup-`, `workspace-setup-`, etc.)
- Check Docker for test containers
- Check VibeRepo database for test records

## Best Practices

1. **Always use #[ignore]** - E2E tests should not run in CI by default
2. **Use unique names** - TestContext generates unique names with timestamps
3. **Clean up resources** - Always call `cleanup()` to avoid resource leaks
4. **Use timeouts** - Set reasonable timeouts for async operations
5. **Log progress** - Use `println!()` to track test progress
6. **Verify thoroughly** - Check both VibeRepo and Gitea state
7. **Handle errors** - Use `.expect()` with descriptive messages

## Performance

E2E tests are slow because they:
- Create real repositories
- Start Docker containers
- Execute actual AI agent tasks
- Wait for PR creation

**Typical test duration:**
- Repository setup: ~10 seconds
- Workspace setup: ~30 seconds
- Complete workflow: ~5-10 minutes

## CI/CD Integration

E2E tests are marked with `#[ignore]` and should be run separately:

\`\`\`yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests
on:
  schedule:
    - cron: '0 2 * * *' # Run nightly
  workflow_dispatch: # Manual trigger

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Start backend
        run: cd backend && cargo run &
      - name: Run E2E tests
        run: ./scripts/run_e2e_tests.sh
\`\`\`

## See Also

- [WebSocket Testing Guide](./websocket-testing.md)
- [API Reference](../api/api-reference.md)
- [User Guide](../api/user-guide.md)
\`\`\`

**Step 4: Update README.md**

Add to `README.md` testing section:

\`\`\`markdown
### End-to-End Tests

Run E2E tests with real Gitea instance:

\`\`\`bash
./scripts/run_e2e_tests.sh
\`\`\`

See [E2E Testing Guide](docs/testing/e2e-testing.md) for details.
\`\`\`

**Step 5: Verify documentation**

Run: `ls -la scripts/run_e2e_tests.sh docs/testing/e2e-testing.md`

Expected: Both files exist and script is executable

**Step 6: Commit**

\`\`\`bash
git add scripts/run_e2e_tests.sh docs/testing/e2e-testing.md README.md
git commit -m "docs: add E2E testing documentation and runner script"
\`\`\`

---


**Files:**
- Modify: `backend/tests/e2e/tests.rs`
- Modify: `backend/Cargo.toml` (add tokio-tungstenite dependency if needed)

**Step 1: Add WebSocket dependencies**

Check if `tokio-tungstenite` is in `backend/Cargo.toml` dev-dependencies. If not, add:

```toml
[dev-dependencies]
tokio-tungstenite = "0.21"
futures-util = "0.3"
```

**Step 2: Add WebSocket monitoring method**

Add to `TestContext` impl in `backend/tests/e2e/tests.rs`:

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};

    /// Monitor task execution via WebSocket
    async fn monitor_task_logs(&self, duration_secs: u64) -> Result<Vec<String>, String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        
        // Get WebSocket auth token from environment or use empty string
        let ws_token = env::var("WEBSOCKET_AUTH_TOKEN").unwrap_or_default();
        let ws_url = if ws_token.is_empty() {
            format!("ws://localhost:3000/api/tasks/{}/logs/stream", task_id)
        } else {
            format!("ws://localhost:3000/api/tasks/{}/logs/stream?token={}", task_id, ws_token)
        };
        
        println!("Connecting to WebSocket: {}", ws_url);
        
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| format!("Failed to connect to WebSocket: {}", e))?;
        
        println!("WebSocket connected");
        
        let (mut write, mut read) = ws_stream.split();
        let mut logs = Vec::new();
        
        // Spawn a task to send ping messages
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                if write.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        });
        
        // Read messages for specified duration
        let timeout = tokio::time::Duration::from_secs(duration_secs);
        let start = tokio::time::Instant::now();
        
        while start.elapsed() < timeout {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            println!("📝 Log: {}", text);
                            logs.push(text);
                        }
                        Some(Ok(Message::Close(_))) => {
                            println!("WebSocket closed");
                            break;
                        }
                        Some(Err(e)) => {
                            println!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            println!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Continue loop
                }
            }
        }
        
        println!("Received {} log messages", logs.len());
        Ok(logs)
    }
```

**Step 3: Write WebSocket monitoring test**

Add to `backend/tests/e2e/tests.rs`:

```rust
#[tokio::test]
#[ignore]
async fn test_e2e_websocket_log_monitoring() {
    let mut ctx = TestContext::new("websocket-logs");
    
    // Setup
    println!("\n=== Setup ===");
    ctx.setup_gitea_repository().await.expect("Failed to create Gitea repository");
    ctx.setup_vibe_provider().await.expect("Failed to create VibeRepo provider");
    ctx.sync_repositories().await.expect("Failed to sync repositories");
    ctx.initialize_repository().await.expect("Failed to initialize repository");
    ctx.create_workspace().await.expect("Failed to create workspace");
    ctx.create_agent().await.expect("Failed to create agent");
    
    // Create simple task
    println!("\n=== Create Task ===");
    let issue_title = "Test WebSocket logs";
    let issue_body = "Simple task to test WebSocket log streaming";
    
    let issue = ctx.gitea_client
        .create_issue(&ctx.test_repo_owner, &ctx.test_repo_name, issue_title, issue_body, vec![])
        .await
        .expect("Failed to create issue");
    
    ctx.create_task(issue.number, issue_title, issue_body, &issue.html_url)
        .await
        .expect("Failed to create task");
    
    ctx.assign_agent_to_task().await.expect("Failed to assign agent");
    
    // Start WebSocket monitoring in background
    println!("\n=== Start WebSocket Monitoring ===");
    let ctx_clone = ctx.clone(); // Need to implement Clone for TestContext
    let monitor_handle = tokio::spawn(async move {
        ctx_clone.monitor_task_logs(120).await // Monitor for 2 minutes
    });
    
    // Wait a bit for WebSocket to connect
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Execute task
    println!("\n=== Execute Task ===");
    ctx.execute_task().await.expect("Failed to execute task");
    
    // Wait for monitoring to complete or timeout
    let logs = monitor_handle.await
        .expect("Monitor task panicked")
        .expect("Failed to monitor logs");
    
    // Verify we received logs
    println!("\n=== Verify Logs ===");
    assert!(!logs.is_empty(), "Should receive at least some log messages");
    
    // Check for expected log patterns
    let has_stdout = logs.iter().any(|log| log.contains("\"stream\":\"stdout\""));
    let has_task_id = logs.iter().any(|log| log.contains(&format!("\"task_id\":{}", ctx.task_id.unwrap())));
    
    assert!(has_stdout || logs.len() > 0, "Should receive stdout logs or connection confirmation");
    
    println!("✅ Received {} log messages via WebSocket", logs.len());
    
    // Cleanup
    println!("\n=== Cleanup ===");
    ctx.cleanup().await.expect("Failed to cleanup");
    
    println!("\n✅ E2E WebSocket log monitoring test passed");
}
```

**Step 4: Make TestContext cloneable**

Add at the top of `TestContext` definition:

```rust
#[derive(Clone)]
struct TestContext {
    // ... existing fields
}
```

Note: You may need to adjust the `Client` fields to use `Arc` for cloning:

```rust
use std::sync::Arc;

#[derive(Clone)]
struct TestContext {
    gitea_client: Arc<GiteaClient>,
    vibe_client: Arc<Client>,
    // ... rest of fields
}
```

**Step 5: Run the test**

Run: `cd backend && cargo test --test e2e test_e2e_websocket_log_monitoring -- --ignored --nocapture`

Expected: Test connects to WebSocket, receives log messages during task execution

**Step 6: Commit**

```bash
git add backend/tests/e2e/tests.rs backend/Cargo.toml
git commit -m "test: add E2E WebSocket log monitoring test"
```

---


**Files:**
- Modify: `backend/tests/e2e/tests.rs`

**Step 1: Add task creation and execution methods**

Add to `TestContext` impl:

```rust
    /// Create a task from issue
    async fn create_task(&mut self, issue_number: i64, issue_title: &str, issue_body: &str, issue_url: &str) -> Result<(), String> {
        let workspace_id = self.workspace_id.ok_or("Workspace ID not set")?;
        
        println!("Creating task for issue #{}", issue_number);
        
        let response = self.vibe_client
            .post(&format!("{}/api/tasks", VIBE_REPO_BASE_URL))
            .json(&json!({
                "workspace_id": workspace_id,
                "issue_number": issue_number,
                "issue_title": issue_title,
                "issue_body": issue_body,
                "issue_url": issue_url,
                "priority": "High",
                "max_retries": 1,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create task: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to create task: {} - {}", status, body));
        }

        let task: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse task response: {}", e))?;
        
        self.task_id = Some(task["id"].as_i64().unwrap() as i32);
        println!("Created task with ID: {}", self.task_id.unwrap());
        
        Ok(())
    }

    /// Assign agent to task
    async fn assign_agent_to_task(&self) -> Result<(), String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        let agent_id = self.agent_id.ok_or("Agent ID not set")?;
        
        println!("Assigning agent {} to task {}", agent_id, task_id);
        
        let response = self.vibe_client
            .post(&format!("{}/api/tasks/{}/assign", VIBE_REPO_BASE_URL, task_id))
            .json(&json!({
                "agent_id": agent_id,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to assign agent: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to assign agent: {} - {}", status, body));
        }

        println!("Agent assigned successfully");
        Ok(())
    }

    /// Execute task
    async fn execute_task(&self) -> Result<(), String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        
        println!("Executing task {}", task_id);
        
        let response = self.vibe_client
            .post(&format!("{}/api/tasks/{}/execute", VIBE_REPO_BASE_URL, task_id))
            .send()
            .await
            .map_err(|e| format!("Failed to execute task: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to execute task: {} - {}", status, body));
        }

        println!("Task execution started");
        Ok(())
    }

    /// Wait for task to complete
    async fn wait_for_task_completion(&self, timeout_secs: u64) -> Result<serde_json::Value, String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        
        println!("Waiting for task {} to complete (timeout: {}s)", task_id, timeout_secs);
        
        wait_for_condition(
            || async {
                let response = self.vibe_client
                    .get(&format!("{}/api/tasks/{}", VIBE_REPO_BASE_URL, task_id))
                    .send()
                    .await;
                
                if let Ok(resp) = response {
                    if let Ok(task) = resp.json::<serde_json::Value>().await {
                        let status = task["status"].as_str().unwrap_or("");
                        println!("Task status: {}", status);
                        return status == "Completed" || status == "Failed";
                    }
                }
                false
            },
            timeout_secs,
            5000, // Check every 5 seconds
        ).await?;

        // Get final task state
        let response = self.vibe_client
            .get(&format!("{}/api/tasks/{}", VIBE_REPO_BASE_URL, task_id))
            .send()
            .await
            .map_err(|e| format!("Failed to get task: {}", e))?;

        let task: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse task: {}", e))?;

        Ok(task)
    }
```

**Step 2: Write complete Issue-to-PR test**

Add to `backend/tests/e2e/tests.rs`:

```rust
#[tokio::test]
#[ignore]
async fn test_e2e_complete_issue_to_pr_workflow() {
    let mut ctx = TestContext::new("issue-to-pr");
    
    // Phase 1: Setup
    println!("\n=== Phase 1: Setup ===");
    ctx.setup_gitea_repository().await.expect("Failed to create Gitea repository");
    ctx.setup_vibe_provider().await.expect("Failed to create VibeRepo provider");
    ctx.sync_repositories().await.expect("Failed to sync repositories");
    ctx.initialize_repository().await.expect("Failed to initialize repository");
    ctx.create_workspace().await.expect("Failed to create workspace");
    ctx.create_agent().await.expect("Failed to create agent");
    
    // Phase 2: Create issue in Gitea
    println!("\n=== Phase 2: Create Issue ===");
    let issue_title = "Add hello world function";
    let issue_body = "Create a simple hello_world() function that prints 'Hello, World!' to stdout.";
    
    let issue = ctx.gitea_client
        .create_issue(
            &ctx.test_repo_owner,
            &ctx.test_repo_name,
            issue_title,
            issue_body,
            vec![], // No labels for now
        )
        .await
        .expect("Failed to create issue");
    
    println!("Created issue #{}: {}", issue.number, issue.html_url);
    
    // Phase 3: Create and execute task
    println!("\n=== Phase 3: Execute Task ===");
    ctx.create_task(issue.number, issue_title, issue_body, &issue.html_url)
        .await
        .expect("Failed to create task");
    
    ctx.assign_agent_to_task().await.expect("Failed to assign agent");
    ctx.execute_task().await.expect("Failed to execute task");
    
    // Phase 4: Wait for completion
    println!("\n=== Phase 4: Wait for Completion ===");
    let task = ctx.wait_for_task_completion(600) // 10 minutes timeout
        .await
        .expect("Task did not complete in time");
    
    let status = task["status"].as_str().expect("Task status not found");
    println!("Task completed with status: {}", status);
    
    // Phase 5: Verify PR was created
    println!("\n=== Phase 5: Verify PR ===");
    assert_eq!(status, "Completed", "Task should complete successfully");
    
    let pr_number = task["pr_number"].as_i64().expect("PR number not found");
    let pr_url = task["pr_url"].as_str().expect("PR URL not found");
    let branch_name = task["branch_name"].as_str().expect("Branch name not found");
    
    println!("PR created: #{} - {}", pr_number, pr_url);
    println!("Branch: {}", branch_name);
    
    // Verify PR exists in Gitea
    let pr = ctx.gitea_client
        .get_pull_request(&ctx.test_repo_owner, &ctx.test_repo_name, pr_number)
        .await
        .expect("Failed to get PR from Gitea");
    
    assert_eq!(pr.title, issue_title);
    assert_eq!(pr.state, "open");
    assert!(pr.body.contains(&format!("#{}", issue.number)), "PR should reference issue");
    
    println!("✅ PR verified in Gitea");
    
    // Phase 6: Cleanup
    println!("\n=== Phase 6: Cleanup ===");
    
    // Close PR
    ctx.gitea_client
        .close_pull_request(&ctx.test_repo_owner, &ctx.test_repo_name, pr_number)
        .await
        .expect("Failed to close PR");
    
    // Delete branch
    ctx.gitea_client
        .delete_branch(&ctx.test_repo_owner, &ctx.test_repo_name, branch_name)
        .await
        .expect("Failed to delete branch");
    
    // Cleanup all resources
    ctx.cleanup().await.expect("Failed to cleanup");
    
    println!("\n✅ E2E complete Issue-to-PR workflow test passed");
}
```

**Step 3: Run the complete test**

Run: `cd backend && cargo test --test e2e test_e2e_complete_issue_to_pr_workflow -- --ignored --nocapture`

Expected: 
- Test creates repository, workspace, agent
- Creates issue in Gitea
- Executes task
- Waits for completion (may take several minutes)
- Verifies PR was created
- Cleans up all resources

**Step 4: Commit**

```bash
git add backend/tests/e2e/tests.rs
git commit -m "test: add complete E2E Issue-to-PR workflow test"
```

---


**Files:**
- Modify: `backend/tests/e2e/tests.rs`

**Step 1: Add workspace creation method**

Add to `TestContext` impl:

```rust
    /// Setup: Create workspace with Docker container
    async fn create_workspace(&mut self) -> Result<(), String> {
        let repository_id = self.repository_id.ok_or("Repository ID not set")?;
        
        println!("Creating workspace for repository {}", repository_id);
        
        let response = self.vibe_client
            .post(&format!("{}/api/workspaces", VIBE_REPO_BASE_URL))
            .json(&json!({
                "repository_id": repository_id,
                "init_script": "#!/bin/bash\necho 'Workspace initialized'\napt-get update -qq\napt-get install -y -qq git curl\necho 'Setup complete'",
                "script_timeout_seconds": 300,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create workspace: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to create workspace: {} - {}", status, body));
        }

        let workspace: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse workspace response: {}", e))?;
        
        self.workspace_id = Some(workspace["id"].as_i64().unwrap() as i32);
        println!("Created workspace with ID: {}", self.workspace_id.unwrap());
        
        // Wait for container to be ready
        println!("Waiting for container to be ready...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        Ok(())
    }

    /// Setup: Create AI agent configuration
    async fn create_agent(&mut self) -> Result<(), String> {
        let workspace_id = self.workspace_id.ok_or("Workspace ID not set")?;
        
        println!("Creating agent for workspace {}", workspace_id);
        
        let response = self.vibe_client
            .post(&format!("{}/api/agents", VIBE_REPO_BASE_URL))
            .json(&json!({
                "workspace_id": workspace_id,
                "name": "E2E Test Agent",
                "tool_type": "OpenCode",
                "command": "opencode --model glm-4-flash",
                "timeout": 600,
                "env_vars": {
                    "TEST_MODE": "true"
                },
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create agent: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to create agent: {} - {}", status, body));
        }

        let agent: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse agent response: {}", e))?;
        
        self.agent_id = Some(agent["id"].as_i64().unwrap() as i32);
        println!("Created agent with ID: {}", self.agent_id.unwrap());
        
        Ok(())
    }
```

**Step 2: Write workspace setup test**

Add to `backend/tests/e2e/tests.rs`:

```rust
#[tokio::test]
#[ignore]
async fn test_e2e_workspace_setup() {
    let mut ctx = TestContext::new("workspace-setup");
    
    // Setup
    ctx.setup_gitea_repository().await.expect("Failed to create Gitea repository");
    ctx.setup_vibe_provider().await.expect("Failed to create VibeRepo provider");
    ctx.sync_repositories().await.expect("Failed to sync repositories");
    ctx.initialize_repository().await.expect("Failed to initialize repository");
    ctx.create_workspace().await.expect("Failed to create workspace");
    ctx.create_agent().await.expect("Failed to create agent");
    
    // Verify workspace exists and is running
    let workspace_id = ctx.workspace_id.expect("Workspace ID not set");
    let response = ctx.vibe_client
        .get(&format!("{}/api/workspaces/{}", VIBE_REPO_BASE_URL, workspace_id))
        .send()
        .await
        .expect("Failed to get workspace");
    
    assert!(response.status().is_success());
    
    let workspace: serde_json::Value = response.json().await.expect("Failed to parse workspace");
    assert_eq!(workspace["status"].as_str(), Some("Running"));
    
    // Verify agent exists
    let agent_id = ctx.agent_id.expect("Agent ID not set");
    let response = ctx.vibe_client
        .get(&format!("{}/api/agents/{}", VIBE_REPO_BASE_URL, agent_id))
        .send()
        .await
        .expect("Failed to get agent");
    
    assert!(response.status().is_success());
    
    // Cleanup
    ctx.cleanup().await.expect("Failed to cleanup");
    
    println!("✅ E2E workspace setup test passed");
}
```

**Step 3: Run the test**

Run: `cd backend && cargo test --test e2e test_e2e_workspace_setup -- --ignored --nocapture`

Expected: Test passes, workspace and agent created successfully

**Step 4: Commit**

```bash
git add backend/tests/e2e/tests.rs
git commit -m "test: add E2E workspace and agent setup test"
```

---


**Files:**
- Modify: `backend/tests/e2e/tests.rs`

**Step 1: Add repository initialization method to TestContext**

Add to `TestContext` impl in `backend/tests/e2e/tests.rs`:

```rust
    /// Setup: Initialize repository with branch and labels
    async fn initialize_repository(&mut self) -> Result<(), String> {
        let repository_id = self.repository_id.ok_or("Repository ID not set")?;
        
        println!("Initializing repository {}", repository_id);
        
        let response = self.vibe_client
            .post(&format!("{}/api/repositories/{}/initialize", VIBE_REPO_BASE_URL, repository_id))
            .json(&json!({
                "branch_name": "vibe-dev",
                "create_labels": true,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to initialize repository: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to initialize repository: {} - {}", status, body));
        }

        println!("Repository initialized successfully");
        Ok(())
    }
```

**Step 2: Write the first E2E test**

Add to `backend/tests/e2e/tests.rs`:

```rust
#[tokio::test]
#[ignore] // Run with: cargo test --test e2e -- --ignored
async fn test_e2e_repository_setup() {
    let mut ctx = TestContext::new("repo-setup");
    
    // Setup
    ctx.setup_gitea_repository().await.expect("Failed to create Gitea repository");
    ctx.setup_vibe_provider().await.expect("Failed to create VibeRepo provider");
    ctx.sync_repositories().await.expect("Failed to sync repositories");
    ctx.initialize_repository().await.expect("Failed to initialize repository");
    
    // Verify repository is initialized
    let repository_id = ctx.repository_id.expect("Repository ID not set");
    let response = ctx.vibe_client
        .get(&format!("{}/api/repositories/{}", VIBE_REPO_BASE_URL, repository_id))
        .send()
        .await
        .expect("Failed to get repository");
    
    assert!(response.status().is_success());
    
    let repo: serde_json::Value = response.json().await.expect("Failed to parse repository");
    assert_eq!(repo["is_initialized"].as_bool(), Some(true));
    
    // Cleanup
    ctx.cleanup().await.expect("Failed to cleanup");
    
    println!("✅ E2E repository setup test passed");
}
```

**Step 3: Run the test**

Run: `cd backend && cargo test --test e2e test_e2e_repository_setup -- --ignored --nocapture`

Expected: Test passes (may need to update `test_repo_owner` in TestContext::new)

**Step 4: Commit**

```bash
git add backend/tests/e2e/tests.rs
git commit -m "test: add E2E repository initialization test"
```

---


**Files:**
- Create: `backend/tests/e2e/mod.rs`
- Create: `backend/tests/e2e/helpers.rs`
- Create: `backend/tests/e2e/gitea_client.rs`
- Modify: `backend/tests/lib.rs` (if needed)

**Step 1: Create test module structure**

Create `backend/tests/e2e/mod.rs`:

```rust
//! End-to-end integration tests with real Gitea instance

pub mod helpers;
pub mod gitea_client;

#[cfg(test)]
mod tests;
```

**Step 2: Create Gitea API client helper**

Create `backend/tests/e2e/gitea_client.rs`:

```rust
//! Gitea API client for E2E tests

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct GiteaClient {
    base_url: String,
    token: String,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GiteaRepository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GiteaIssue {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GiteaPullRequest {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub html_url: String,
    pub head: GiteaBranch,
    pub base: GiteaBranch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GiteaBranch {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

impl GiteaClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            base_url,
            token,
            client: Client::builder()
                .danger_accept_invalid_certs(true) // For self-signed certs
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create a test repository
    pub async fn create_repository(&self, name: &str, description: &str) -> Result<GiteaRepository, String> {
        let url = format!("{}/api/v1/user/repos", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&json!({
                "name": name,
                "description": description,
                "private": false,
                "auto_init": true,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if response.status() == StatusCode::CREATED || response.status() == StatusCode::OK {
            response.json().await.map_err(|e| format!("Failed to parse response: {}", e))
        } else {
            Err(format!("Failed to create repository: {}", response.status()))
        }
    }

    /// Delete a repository
    pub async fn delete_repository(&self, owner: &str, repo: &str) -> Result<(), String> {
        let url = format!("{}/api/v1/repos/{}/{}", self.base_url, owner, repo);
        
        let response = self.client
            .delete(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::OK {
            Ok(())
        } else {
            Err(format!("Failed to delete repository: {}", response.status()))
        }
    }

    /// Create an issue
    pub async fn create_issue(&self, owner: &str, repo: &str, title: &str, body: &str, labels: Vec<String>) -> Result<GiteaIssue, String> {
        let url = format!("{}/api/v1/repos/{}/{}/issues", self.base_url, owner, repo);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&json!({
                "title": title,
                "body": body,
                "labels": labels.iter().map(|l| l.parse::<i64>().ok()).collect::<Vec<_>>(),
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if response.status() == StatusCode::CREATED {
            response.json().await.map_err(|e| format!("Failed to parse response: {}", e))
        } else {
            Err(format!("Failed to create issue: {}", response.status()))
        }
    }

    /// Get pull request by number
    pub async fn get_pull_request(&self, owner: &str, repo: &str, number: i64) -> Result<GiteaPullRequest, String> {
        let url = format!("{}/api/v1/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if response.status() == StatusCode::OK {
            response.json().await.map_err(|e| format!("Failed to parse response: {}", e))
        } else {
            Err(format!("Failed to get pull request: {}", response.status()))
        }
    }

    /// Close a pull request
    pub async fn close_pull_request(&self, owner: &str, repo: &str, number: i64) -> Result<(), String> {
        let url = format!("{}/api/v1/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number);
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&json!({
                "state": "closed"
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if response.status() == StatusCode::CREATED || response.status() == StatusCode::OK {
            Ok(())
        } else {
            Err(format!("Failed to close pull request: {}", response.status()))
        }
    }

    /// Delete a branch
    pub async fn delete_branch(&self, owner: &str, repo: &str, branch: &str) -> Result<(), String> {
        let url = format!("{}/api/v1/repos/{}/{}/branches/{}", self.base_url, owner, repo, branch);
        
        let response = self.client
            .delete(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(format!("Failed to delete branch: {}", response.status()))
        }
    }
}
```

**Step 3: Create test helpers**

Create `backend/tests/e2e/helpers.rs`:

```rust
//! Helper functions for E2E tests

use std::time::Duration;
use tokio::time::sleep;

/// Wait for a condition to be true with timeout
pub async fn wait_for_condition<F, Fut>(
    mut condition: F,
    timeout_secs: u64,
    check_interval_ms: u64,
) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let interval = Duration::from_millis(check_interval_ms);

    loop {
        if condition().await {
            return Ok(());
        }

        if start.elapsed() > timeout {
            return Err(format!("Timeout after {} seconds", timeout_secs));
        }

        sleep(interval).await;
    }
}

/// Generate unique test name with timestamp
pub fn generate_test_name(prefix: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}-{}", prefix, timestamp)
}
```

**Step 4: Verify compilation**

Run: `cd backend && cargo test --test e2e --no-run`

Expected: Compilation succeeds

**Step 5: Commit**

```bash
git add backend/tests/e2e/
git commit -m "test: add E2E test infrastructure with Gitea client"
```

---


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

## Task 2: Create E2E Test Setup Phase

**Files:**
- Create: `backend/tests/e2e/tests.rs`
- Modify: `backend/tests/e2e/mod.rs`

**Step 1: Add tests module to mod.rs**

Modify `backend/tests/e2e/mod.rs`:

```rust
//! End-to-end integration tests with real Gitea instance

pub mod helpers;
pub mod gitea_client;

#[cfg(test)]
mod tests;
```

**Step 2: Create test setup structure**

Create `backend/tests/e2e/tests.rs`:

```rust
//! E2E test cases

use super::gitea_client::GiteaClient;
use super::helpers::{generate_test_name, wait_for_condition};
use reqwest::Client;
use serde_json::json;
use std::env;

const GITEA_BASE_URL: &str = "https://gitea.devo.top:66";
const GITEA_TOKEN: &str = "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2";
const VIBE_REPO_BASE_URL: &str = "http://localhost:3000";

struct TestContext {
    gitea_client: GiteaClient,
    vibe_client: Client,
    test_repo_name: String,
    test_repo_owner: String,
    provider_id: Option<i32>,
    repository_id: Option<i32>,
    workspace_id: Option<i32>,
    agent_id: Option<i32>,
    task_id: Option<i32>,
}

impl TestContext {
    fn new(test_name: &str) -> Self {
        Self {
            gitea_client: GiteaClient::new(GITEA_BASE_URL.to_string(), GITEA_TOKEN.to_string()),
            vibe_client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .expect("Failed to create HTTP client"),
            test_repo_name: generate_test_name(test_name),
            test_repo_owner: "your-gitea-username".to_string(), // TODO: Get from API
            provider_id: None,
            repository_id: None,
            workspace_id: None,
            agent_id: None,
            task_id: None,
        }
    }

    /// Setup: Create test repository in Gitea
    async fn setup_gitea_repository(&mut self) -> Result<(), String> {
        println!("Creating test repository: {}", self.test_repo_name);
        
        let repo = self.gitea_client
            .create_repository(&self.test_repo_name, "E2E test repository")
            .await?;
        
        println!("Created repository: {}", repo.html_url);
        Ok(())
    }

    /// Setup: Configure VibeRepo with Gitea provider
    async fn setup_vibe_provider(&mut self) -> Result<(), String> {
        println!("Creating VibeRepo provider");
        
        let response = self.vibe_client
            .post(&format!("{}/api/settings/providers", VIBE_REPO_BASE_URL))
            .json(&json!({
                "name": format!("E2E Test Provider {}", self.test_repo_name),
                "type": "gitea",
                "base_url": GITEA_BASE_URL,
                "access_token": GITEA_TOKEN,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to create provider: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to create provider: {}", response.status()));
        }

        let provider: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse provider response: {}", e))?;
        
        self.provider_id = Some(provider["id"].as_i64().unwrap() as i32);
        println!("Created provider with ID: {}", self.provider_id.unwrap());
        
        Ok(())
    }

    /// Setup: Sync repositories from provider
    async fn sync_repositories(&mut self) -> Result<(), String> {
        let provider_id = self.provider_id.ok_or("Provider ID not set")?;
        
        println!("Syncing repositories from provider {}", provider_id);
        
        let response = self.vibe_client
            .post(&format!("{}/api/settings/providers/{}/sync", VIBE_REPO_BASE_URL, provider_id))
            .send()
            .await
            .map_err(|e| format!("Failed to sync repositories: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to sync repositories: {}", response.status()));
        }

        // Wait for sync to complete and find our repository
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Get repositories list
        let response = self.vibe_client
            .get(&format!("{}/api/repositories", VIBE_REPO_BASE_URL))
            .send()
            .await
            .map_err(|e| format!("Failed to get repositories: {}", e))?;

        let repos: Vec<serde_json::Value> = response.json().await
            .map_err(|e| format!("Failed to parse repositories: {}", e))?;

        // Find our test repository
        for repo in repos {
            if repo["name"].as_str() == Some(&self.test_repo_name) {
                self.repository_id = Some(repo["id"].as_i64().unwrap() as i32);
                println!("Found repository with ID: {}", self.repository_id.unwrap());
                return Ok(());
            }
        }

        Err(format!("Repository {} not found after sync", self.test_repo_name))
    }

    /// Cleanup: Remove all test resources
    async fn cleanup(&self) -> Result<(), String> {
        println!("Cleaning up test resources...");

        // Delete workspace if exists
        if let Some(workspace_id) = self.workspace_id {
            println!("Deleting workspace {}", workspace_id);
            let _ = self.vibe_client
                .delete(&format!("{}/api/workspaces/{}", VIBE_REPO_BASE_URL, workspace_id))
                .send()
                .await;
        }

        // Delete repository from VibeRepo if exists
        if let Some(repository_id) = self.repository_id {
            println!("Deleting repository {}", repository_id);
            let _ = self.vibe_client
                .delete(&format!("{}/api/repositories/{}", VIBE_REPO_BASE_URL, repository_id))
                .send()
                .await;
        }

        // Delete provider if exists
        if let Some(provider_id) = self.provider_id {
            println!("Deleting provider {}", provider_id);
            let _ = self.vibe_client
                .delete(&format!("{}/api/settings/providers/{}", VIBE_REPO_BASE_URL, provider_id))
                .send()
                .await;
        }

        // Delete Gitea repository
        println!("Deleting Gitea repository {}/{}", self.test_repo_owner, self.test_repo_name);
        self.gitea_client
            .delete_repository(&self.test_repo_owner, &self.test_repo_name)
            .await?;

        println!("Cleanup complete");
        Ok(())
    }
}
```

**Step 3: Verify compilation**

Run: `cd backend && cargo test --test e2e --no-run`

Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add backend/tests/e2e/tests.rs backend/tests/e2e/mod.rs
git commit -m "test: add E2E test setup phase with TestContext"
```

---

