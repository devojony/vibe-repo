//! E2E test cases
//!
//! This module contains end-to-end tests that interact with a real Gitea instance
//! and the VibeRepo API to test the complete workflow.

use super::gitea_client::GiteaClient;
use super::helpers::{generate_test_name, wait_for_condition};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};

// Test environment constants
const GITEA_BASE_URL: &str = "https://gitea.devo.top:66";
const GITEA_TOKEN: &str = "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2";
const VIBE_REPO_BASE_URL: &str = "http://localhost:3000";

/// Test context that manages test resources and provides setup/cleanup methods
#[derive(Clone)]
struct TestContext {
    gitea_client: Arc<GiteaClient>,
    vibe_client: Arc<Client>,
    test_repo_name: String,
    test_repo_owner: String,
    provider_id: Option<i32>,
    repository_id: Option<i32>,
    workspace_id: Option<i32>,
    agent_id: Option<i32>,
    task_id: Option<i32>,
}

/// Provider response from VibeRepo API
#[derive(Debug, Deserialize)]
struct ProviderResponse {
    id: i32,
    name: String,
    provider_type: String,
    base_url: String,
}

/// Repository response from VibeRepo API
#[derive(Debug, Deserialize)]
struct RepositoryResponse {
    id: i32,
    provider_id: i32,
    name: String,
    full_name: String,
}

/// Request body for creating a provider
#[derive(Debug, Serialize)]
struct CreateProviderRequest {
    name: String,
    provider_type: String,
    base_url: String,
    access_token: String,
}

impl TestContext {
    /// Creates a new test context
    ///
    /// # Arguments
    ///
    /// * `test_name` - Name of the test (used to generate unique resource names)
    ///
    /// # Returns
    ///
    /// Returns a new TestContext instance
    fn new(test_name: &str) -> Self {
        let gitea_client = GiteaClient::new(GITEA_BASE_URL.to_string(), GITEA_TOKEN.to_string())
            .expect("Failed to create Gitea client");

        let vibe_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        let test_repo_name = generate_test_name(test_name);
        // TODO: Get from API or environment variable
        let test_repo_owner = "devo".to_string();

        Self {
            gitea_client: Arc::new(gitea_client),
            vibe_client: Arc::new(vibe_client),
            test_repo_name,
            test_repo_owner,
            provider_id: None,
            repository_id: None,
            workspace_id: None,
            agent_id: None,
            task_id: None,
        }
    }

    /// Sets up a test repository in Gitea
    ///
    /// Creates a new repository with auto-initialization (README, main branch)
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    async fn setup_gitea_repository(&mut self) -> Result<(), String> {
        println!("Creating test repository: {}", self.test_repo_name);

        let repo = self
            .gitea_client
            .create_repository(&self.test_repo_name, "E2E test repository")
            .await
            .map_err(|e| format!("Failed to create Gitea repository: {}", e))?;

        println!("✓ Created Gitea repository: {} (ID: {})", repo.name, repo.id);

        Ok(())
    }

    /// Sets up a provider in VibeRepo
    ///
    /// Creates a new provider configuration pointing to the Gitea instance
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    async fn setup_vibe_provider(&mut self) -> Result<(), String> {
        println!("Creating VibeRepo provider");

        let provider_name = format!("test-provider-{}", self.test_repo_name);
        let request_body = CreateProviderRequest {
            name: provider_name,
            provider_type: "gitea".to_string(),
            base_url: GITEA_BASE_URL.to_string(),
            access_token: GITEA_TOKEN.to_string(),
        };

        let url = format!("{}/api/settings/providers", VIBE_REPO_BASE_URL);
        let response = self
            .vibe_client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to send create provider request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to create provider (status {}): {}",
                status, error_text
            ));
        }

        let provider: ProviderResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse provider response: {}", e))?;

        self.provider_id = Some(provider.id);
        println!("✓ Created VibeRepo provider: {} (ID: {})", provider.name, provider.id);

        Ok(())
    }

    /// Syncs repositories from the provider
    ///
    /// Triggers repository synchronization and waits for it to complete,
    /// then finds and stores the test repository ID
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    async fn sync_repositories(&mut self) -> Result<(), String> {
        let provider_id = self
            .provider_id
            .ok_or_else(|| "Provider ID not set".to_string())?;

        println!("Syncing repositories from provider {}", provider_id);

        // Trigger sync
        let sync_url = format!(
            "{}/api/settings/providers/{}/sync",
            VIBE_REPO_BASE_URL, provider_id
        );
        let sync_response = self
            .vibe_client
            .post(&sync_url)
            .send()
            .await
            .map_err(|e| format!("Failed to send sync request: {}", e))?;

        if !sync_response.status().is_success() {
            let status = sync_response.status();
            let error_text = sync_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to sync repositories (status {}): {}",
                status, error_text
            ));
        }

        println!("✓ Triggered repository sync");

        // Wait for sync to complete
        println!("Waiting for sync to complete...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Fetch repository list
        let repos_url = format!("{}/api/repositories", VIBE_REPO_BASE_URL);
        let repos_response = self
            .vibe_client
            .get(&repos_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch repositories: {}", e))?;

        if !repos_response.status().is_success() {
            let status = repos_response.status();
            let error_text = repos_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to fetch repositories (status {}): {}",
                status, error_text
            ));
        }

        let repositories: Vec<RepositoryResponse> = repos_response
            .json()
            .await
            .map_err(|e| format!("Failed to parse repositories response: {}", e))?;

        // Find test repository
        let test_repo = repositories
            .iter()
            .find(|r| r.name == self.test_repo_name)
            .ok_or_else(|| {
                format!(
                    "Test repository '{}' not found in synced repositories",
                    self.test_repo_name
                )
            })?;

        self.repository_id = Some(test_repo.id);
        println!(
            "✓ Found test repository: {} (ID: {})",
            test_repo.full_name, test_repo.id
        );

        Ok(())
    }

    /// Setup: Initialize repository with branch and labels
    ///
    /// Initializes the repository in VibeRepo by creating a development branch
    /// and setting up labels for issue management
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    async fn initialize_repository(&mut self) -> Result<(), String> {
        let repository_id = self.repository_id.ok_or("Repository ID not set")?;
        
        println!("✓ Initializing repository {}", repository_id);
        
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

        println!("✓ Repository initialized successfully");
        Ok(())
    }

    /// Setup: Create workspace with Docker container
    async fn create_workspace(&mut self) -> Result<(), String> {
        let repository_id = self.repository_id.ok_or("Repository ID not set")?;
        
        println!("✓ Creating workspace for repository {}", repository_id);
        
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
        println!("✓ Created workspace with ID: {}", self.workspace_id.unwrap());
        
        // Wait for container to be ready
        println!("✓ Waiting for container to be ready...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        Ok(())
    }

    /// Setup: Create AI agent configuration
    async fn create_agent(&mut self) -> Result<(), String> {
        let workspace_id = self.workspace_id.ok_or("Workspace ID not set")?;
        
        println!("✓ Creating agent for workspace {}", workspace_id);
        
        let response = self.vibe_client
            .post(&format!("{}/api/agents", VIBE_REPO_BASE_URL))
            .json(&json!({
                "workspace_id": workspace_id,
                "name": "E2E Test Agent",
                "tool_type": "OpenCode",
                "model_name": "glm-4-flash",
                "timeout_seconds": 600,
                "environment_variables": {
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
        println!("✓ Created agent with ID: {}", self.agent_id.unwrap());
        
        Ok(())
    }

    /// Create a task from issue
    async fn create_task(&mut self, issue_number: i64, issue_title: &str, issue_body: &str, issue_url: &str) -> Result<(), String> {
        let workspace_id = self.workspace_id.ok_or("Workspace ID not set")?;
        
        println!("✓ Creating task for issue #{}", issue_number);
        
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
        println!("✓ Created task with ID: {}", self.task_id.unwrap());
        
        Ok(())
    }

    /// Assign agent to task
    async fn assign_agent_to_task(&self) -> Result<(), String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        let agent_id = self.agent_id.ok_or("Agent ID not set")?;
        
        println!("✓ Assigning agent {} to task {}", agent_id, task_id);
        
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

        println!("✓ Agent assigned successfully");
        Ok(())
    }

    /// Execute task
    async fn execute_task(&self) -> Result<(), String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        
        println!("✓ Executing task {}", task_id);
        
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

        println!("✓ Task execution started");
        Ok(())
    }

    /// Wait for task to complete
    async fn wait_for_task_completion(&self, timeout_secs: u64) -> Result<serde_json::Value, String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        
        println!("✓ Waiting for task {} to complete (timeout: {}s)", task_id, timeout_secs);
        
        wait_for_condition(
            || async {
                let response = self.vibe_client
                    .get(&format!("{}/api/tasks/{}", VIBE_REPO_BASE_URL, task_id))
                    .send()
                    .await;
                
                if let Ok(resp) = response {
                    if let Ok(task) = resp.json::<serde_json::Value>().await {
                        let status = task["status"].as_str().unwrap_or("");
                        println!("  Task status: {}", status);
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

    /// Monitor task execution via WebSocket
    async fn monitor_task_logs(&self, duration_secs: u64) -> Result<Vec<String>, String> {
        let task_id = self.task_id.ok_or("Task ID not set")?;
        
        // Get WebSocket auth token from environment or use empty string
        let ws_token = std::env::var("WEBSOCKET_AUTH_TOKEN").unwrap_or_default();
        let ws_url = if ws_token.is_empty() {
            format!("ws://localhost:3000/api/tasks/{}/logs/stream", task_id)
        } else {
            format!("ws://localhost:3000/api/tasks/{}/logs/stream?token={}", task_id, ws_token)
        };
        
        println!("✓ Connecting to WebSocket: {}", ws_url);
        
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| format!("Failed to connect to WebSocket: {}", e))?;
        
        println!("✓ WebSocket connected");
        
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
                            println!("✓ WebSocket closed");
                            break;
                        }
                        Some(Err(e)) => {
                            println!("⚠ WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            println!("✓ WebSocket stream ended");
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
        
        println!("✓ Received {} log messages", logs.len());
        Ok(logs)
    }

    /// Cleans up all test resources
    ///
    /// Deletes workspace, repository, provider, and Gitea repository.
    /// Uses best-effort approach - ignores errors for optional cleanup steps.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    async fn cleanup(&self) -> Result<(), String> {
        println!("Cleaning up test resources...");

        // Delete workspace (if exists)
        if let Some(workspace_id) = self.workspace_id {
            println!("Deleting workspace {}", workspace_id);
            let url = format!("{}/api/workspaces/{}", VIBE_REPO_BASE_URL, workspace_id);
            let _ = self.vibe_client.delete(&url).send().await;
            println!("✓ Deleted workspace");
        }

        // Delete repository from VibeRepo (if exists)
        if let Some(repository_id) = self.repository_id {
            println!("Deleting repository {}", repository_id);
            let url = format!("{}/api/repositories/{}", VIBE_REPO_BASE_URL, repository_id);
            let _ = self.vibe_client.delete(&url).send().await;
            println!("✓ Deleted repository from VibeRepo");
        }

        // Delete provider (if exists)
        if let Some(provider_id) = self.provider_id {
            println!("Deleting provider {}", provider_id);
            let url = format!("{}/api/settings/providers/{}", VIBE_REPO_BASE_URL, provider_id);
            let _ = self.vibe_client.delete(&url).send().await;
            println!("✓ Deleted provider");
        }

        // Delete Gitea repository
        println!("Deleting Gitea repository: {}/{}", self.test_repo_owner, self.test_repo_name);
        self.gitea_client
            .delete_repository(&self.test_repo_owner, &self.test_repo_name)
            .await
            .map_err(|e| format!("Failed to delete Gitea repository: {}", e))?;
        println!("✓ Deleted Gitea repository");

        println!("✓ Cleanup complete");
        Ok(())
    }
}

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
    
    println!("✓ Created issue #{}: {}", issue.number, issue.html_url);
    
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
    println!("✓ Task completed with status: {}", status);
    
    // Phase 5: Verify PR was created
    println!("\n=== Phase 5: Verify PR ===");
    assert_eq!(status, "Completed", "Task should complete successfully");
    
    let pr_number = task["pr_number"].as_i64().expect("PR number not found");
    let pr_url = task["pr_url"].as_str().expect("PR URL not found");
    let branch_name = task["branch_name"].as_str().expect("Branch name not found");
    
    println!("✓ PR created: #{} - {}", pr_number, pr_url);
    println!("✓ Branch: {}", branch_name);
    
    // Verify PR exists in Gitea
    let pr = ctx.gitea_client
        .get_pull_request(&ctx.test_repo_owner, &ctx.test_repo_name, pr_number)
        .await
        .expect("Failed to get PR from Gitea");
    
    assert_eq!(pr.title, issue_title);
    assert_eq!(pr.state, "open");
    assert!(pr.body.contains(&format!("#{}", issue.number)), "PR should reference issue");
    
    println!("✓ PR verified in Gitea");
    
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
    let ctx_clone = ctx.clone();
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
    let has_log_messages = logs.iter().any(|log| log.contains("\"type\":\"log\"") || log.contains("stream"));
    
    println!("✓ Received {} log messages via WebSocket", logs.len());
    if has_log_messages {
        println!("✓ Log messages contain expected patterns");
    }
    
    // Cleanup
    println!("\n=== Cleanup ===");
    ctx.cleanup().await.expect("Failed to cleanup");
    
    println!("\n✅ E2E WebSocket log monitoring test passed");
}
