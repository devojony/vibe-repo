//! E2E test cases
//!
//! This module contains end-to-end tests that interact with a real Gitea instance
//! and the VibeRepo API to test the complete workflow.

use super::gitea_client::GiteaClient;
use super::helpers::generate_test_name;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

// Test environment constants
const GITEA_BASE_URL: &str = "https://gitea.devo.top:66";
const GITEA_TOKEN: &str = "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2";
const VIBE_REPO_BASE_URL: &str = "http://localhost:3000";

/// Test context that manages test resources and provides setup/cleanup methods
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
            gitea_client,
            vibe_client,
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
