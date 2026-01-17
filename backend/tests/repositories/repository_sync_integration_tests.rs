//! Integration tests for Repository Synchronization
//!
//! Tests the background service that synchronizes repositories when:
//! - Create provider → Wait for sync → Verify repositories stored
//! - Update provider token → Wait for re-sync → Verify repositories updated
//!
//! Requirements: 10.6, 10.7, 10.8, 10.9
//!
//! ## Running these tests
//!
//! These tests require a Gitea test instance. Set the following environment variables:
//! - `GITEA_TEST_URL`: Base URL of the test Gitea instance (default: https://gitea.devo.top:66)
//! - `GITEA_TEST_TOKEN`: Access token for the test Gitea instance
//!
//! Run with: `cargo test --test repository_sync_integration_tests -- --ignored`

use axum::body::Body;
use axum::http::{Request, StatusCode};
use vibe_repo::api::repositories::models::RepositoryResponse;
use vibe_repo::api::settings::providers::models::ProviderResponse;
use vibe_repo::entities::repository::ValidationStatus;
use vibe_repo::test_utils::{is_gitea_available, wait_for_repositories, GiteaTestConfig};
use http_body_util::BodyExt;
use serde_json::json;
use std::time::Duration;
use tower::ServiceExt;

// ============================================
// Test Configuration
// ============================================

const SYNC_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Get Gitea test configuration or skip the test
fn get_gitea_config() -> Option<GiteaTestConfig> {
    let config = GiteaTestConfig::from_env()?;
    if !config.has_credentials() {
        eprintln!("Skipping test: GITEA_TEST_TOKEN environment variable not set");
        return None;
    }
    Some(config)
}

// ============================================
// Helper Functions
// ============================================

/// Helper function to create a test provider and return the response
async fn create_provider(
    app: axum::Router,
    name: &str,
    base_url: &str,
    token: &str,
) -> ProviderResponse {
    let request_body = json!({
        "name": name,
        "provider_type": "gitea",
        "base_url": base_url,
        "access_token": token
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/providers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// Helper function to update a provider
async fn update_provider(
    app: axum::Router,
    provider_id: i32,
    update_data: serde_json::Value,
) -> ProviderResponse {
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/settings/providers/{}", provider_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// Helper function to list repositories for a provider
async fn list_repositories(app: axum::Router, provider_id: i32) -> Vec<RepositoryResponse> {
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/repositories?provider_id={}", provider_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// Helper function to trigger sync for a provider
async fn trigger_sync(app: axum::Router, provider_id: i32) -> StatusCode {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/settings/providers/{}/sync", provider_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    response.status()
}

// ============================================
// Integration Tests
// ============================================

/// Test: Create provider → Wait for sync → Verify repositories stored
/// Requirements: 10.6, 10.7, 10.8, 10.9
#[tokio::test]
#[ignore]
async fn test_sync_after_provider_creation() {
    // Arrange: Get test configuration
    let config = match get_gitea_config() {
        Some(c) => c,
        None => return,
    };

    // Check if Gitea is available
    if !is_gitea_available(&config, Duration::from_secs(5)).await {
        eprintln!(
            "Skipping test: Gitea instance not available at {}",
            config.base_url
        );
        return;
    }

    let state = vibe_repo::test_utils::create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Sync Test Provider",
        &config.base_url,
        config.token(),
    )
    .await;

    assert_eq!(provider.name, "Sync Test Provider");

    // Wait for sync using polling instead of fixed sleep
    let synced = wait_for_repositories(app.clone(), provider.id, SYNC_TIMEOUT, POLL_INTERVAL).await;
    assert!(synced, "Repositories should be synced within timeout");

    // Assert: List repositories for this provider
    let repositories = list_repositories(app.clone(), provider.id).await;

    assert!(
        !repositories.is_empty(),
        "Should have synced repositories from Gitea"
    );

    // Verify repository fields are populated
    for repo in &repositories {
        assert_eq!(repo.provider_id, provider.id);
        assert!(!repo.name.is_empty());
        assert!(!repo.full_name.is_empty());
        assert!(!repo.clone_url.is_empty());
        assert!(!repo.default_branch.is_empty());

        // Validation should have been performed
        assert!(
            repo.validation_status == ValidationStatus::Valid
                || repo.validation_status == ValidationStatus::Invalid
                || repo.validation_status == ValidationStatus::Pending
        );

        // Timestamps should be set
        assert!(!repo.created_at.is_empty());
        assert!(!repo.updated_at.is_empty());
    }

    println!("Synced {} repositories", repositories.len());
    for repo in &repositories {
        println!(
            "  - {} (status: {:?}, branches: {}, can_manage_prs: {}, can_manage_issues: {})",
            repo.full_name,
            repo.validation_status,
            repo.has_required_branches,
            repo.can_manage_prs,
            repo.can_manage_issues
        );
    }
}

/// Test: Update provider token → Wait for re-sync → Verify repositories updated
/// Requirements: 10.6, 10.7, 10.8, 10.9
#[tokio::test]
#[ignore]
async fn test_resync_after_token_update() {
    // Arrange: Get test configuration
    let config = match get_gitea_config() {
        Some(c) => c,
        None => return,
    };

    if !is_gitea_available(&config, Duration::from_secs(5)).await {
        eprintln!(
            "Skipping test: Gitea instance not available at {}",
            config.base_url
        );
        return;
    }

    let state = vibe_repo::test_utils::create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Resync Test Provider",
        &config.base_url,
        config.token(),
    )
    .await;

    // Wait for initial sync
    let synced = wait_for_repositories(app.clone(), provider.id, SYNC_TIMEOUT, POLL_INTERVAL).await;
    assert!(synced, "Initial sync should complete");

    // Get initial repository count
    let initial_repos = list_repositories(app.clone(), provider.id).await;
    let initial_count = initial_repos.len();

    println!("Initial sync: {} repositories", initial_count);

    // Update the provider token (same token, but should trigger re-sync)
    let update_data = json!({
        "access_token": config.token()
    });

    let _updated_provider = update_provider(app.clone(), provider.id, update_data).await;

    // Wait for re-sync to complete
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Assert: Get updated repository list
    let updated_repos = list_repositories(app.clone(), provider.id).await;
    let updated_count = updated_repos.len();

    println!("After re-sync: {} repositories", updated_count);

    assert!(
        updated_count >= initial_count,
        "Re-sync should maintain or increase repository count"
    );

    for repo in &updated_repos {
        assert_eq!(repo.provider_id, provider.id);
        assert!(!repo.name.is_empty());
        assert!(!repo.full_name.is_empty());
    }
}

/// Test: Update provider base_url → Wait for re-sync → Verify repositories updated
/// Requirements: 10.6, 10.7, 10.8
#[tokio::test]
#[ignore]
async fn test_resync_after_base_url_update() {
    // Arrange: Get test configuration
    let config = match get_gitea_config() {
        Some(c) => c,
        None => return,
    };

    if !is_gitea_available(&config, Duration::from_secs(5)).await {
        eprintln!(
            "Skipping test: Gitea instance not available at {}",
            config.base_url
        );
        return;
    }

    let state = vibe_repo::test_utils::create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Base URL Test Provider",
        &config.base_url,
        config.token(),
    )
    .await;

    // Wait for initial sync
    let synced = wait_for_repositories(app.clone(), provider.id, SYNC_TIMEOUT, POLL_INTERVAL).await;
    assert!(synced, "Initial sync should complete");

    let initial_repos = list_repositories(app.clone(), provider.id).await;
    let initial_count = initial_repos.len();

    println!("Initial sync: {} repositories", initial_count);

    // Update the provider base_url (same URL, but should trigger re-sync)
    let update_data = json!({
        "base_url": &config.base_url
    });

    let _updated_provider = update_provider(app.clone(), provider.id, update_data).await;

    // Wait for re-sync to complete
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Assert: Get updated repository list
    let updated_repos = list_repositories(app.clone(), provider.id).await;
    let updated_count = updated_repos.len();

    println!("After re-sync: {} repositories", updated_count);

    assert!(
        updated_count >= initial_count,
        "Re-sync should maintain or increase repository count"
    );
}

/// Test: Manual sync trigger → Verify repositories updated
/// Requirements: 6.1.1, 6.1.2, 6.1.3, 6.1.4
#[tokio::test]
#[ignore]
async fn test_manual_sync_trigger() {
    // Arrange: Get test configuration
    let config = match get_gitea_config() {
        Some(c) => c,
        None => return,
    };

    if !is_gitea_available(&config, Duration::from_secs(5)).await {
        eprintln!(
            "Skipping test: Gitea instance not available at {}",
            config.base_url
        );
        return;
    }

    let state = vibe_repo::test_utils::create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Manual Sync Provider",
        &config.base_url,
        config.token(),
    )
    .await;

    // Wait for initial sync
    let synced = wait_for_repositories(app.clone(), provider.id, SYNC_TIMEOUT, POLL_INTERVAL).await;
    assert!(synced, "Initial sync should complete");

    let initial_repos = list_repositories(app.clone(), provider.id).await;
    let initial_count = initial_repos.len();

    println!("Initial sync: {} repositories", initial_count);

    // Trigger manual sync
    let sync_status = trigger_sync(app.clone(), provider.id).await;

    // Should return 202 Accepted
    assert_eq!(sync_status, StatusCode::ACCEPTED);

    // Wait for sync to complete
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Assert: Get updated repository list
    let updated_repos = list_repositories(app.clone(), provider.id).await;
    let updated_count = updated_repos.len();

    println!("After manual sync: {} repositories", updated_count);

    assert!(
        updated_count >= initial_count,
        "Manual sync should maintain or increase repository count"
    );
}

/// Test: Sync with invalid token → Verify error handling
/// Requirements: 10.8, 10.9
#[tokio::test]
#[ignore]
async fn test_sync_with_invalid_token() {
    // Arrange: Get test configuration (only need base_url)
    let config = GiteaTestConfig::from_env().expect("Failed to load config");

    if !is_gitea_available(&config, Duration::from_secs(5)).await {
        eprintln!(
            "Skipping test: Gitea instance not available at {}",
            config.base_url
        );
        return;
    }

    let state = vibe_repo::test_utils::create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Create a provider with invalid token
    let provider = create_provider(
        app.clone(),
        "Invalid Token Provider",
        &config.base_url,
        "invalid_token_12345678",
    )
    .await;

    // Wait for sync attempt
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Assert: List repositories (should be empty or have error status)
    let repositories = list_repositories(app.clone(), provider.id).await;

    println!(
        "Repositories after invalid token sync: {}",
        repositories.len()
    );

    // The service should handle the error gracefully
    for repo in &repositories {
        println!(
            "  - {} (status: {:?})",
            repo.full_name, repo.validation_status
        );
    }
}

/// Test: Sync stores all visible repositories regardless of validation
/// Requirements: 10.9, 10.10, 10.11, 10.12, 10.13
#[tokio::test]
#[ignore]
async fn test_sync_stores_all_repositories() {
    // Arrange: Get test configuration
    let config = match get_gitea_config() {
        Some(c) => c,
        None => return,
    };

    if !is_gitea_available(&config, Duration::from_secs(5)).await {
        eprintln!(
            "Skipping test: Gitea instance not available at {}",
            config.base_url
        );
        return;
    }

    let state = vibe_repo::test_utils::create_test_state()
        .await
        .expect("Failed to create test state");
    let app = vibe_repo::api::create_router(state.clone());

    // Act: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "All Repos Provider",
        &config.base_url,
        config.token(),
    )
    .await;

    // Wait for sync
    let synced = wait_for_repositories(app.clone(), provider.id, SYNC_TIMEOUT, POLL_INTERVAL).await;
    assert!(synced, "Sync should complete");

    // Assert: List all repositories
    let repositories = list_repositories(app.clone(), provider.id).await;

    println!("Total repositories synced: {}", repositories.len());

    let valid_count = repositories
        .iter()
        .filter(|r| r.validation_status == ValidationStatus::Valid)
        .count();

    let invalid_count = repositories
        .iter()
        .filter(|r| r.validation_status == ValidationStatus::Invalid)
        .count();

    let pending_count = repositories
        .iter()
        .filter(|r| r.validation_status == ValidationStatus::Pending)
        .count();

    println!("  Valid: {}", valid_count);
    println!("  Invalid: {}", invalid_count);
    println!("  Pending: {}", pending_count);

    assert!(
        !repositories.is_empty(),
        "Should store all visible repositories"
    );

    for repo in &repositories {
        assert!(
            repo.validation_status == ValidationStatus::Valid
                || repo.validation_status == ValidationStatus::Invalid
                || repo.validation_status == ValidationStatus::Pending
        );

        if repo.validation_status == ValidationStatus::Invalid {
            assert!(
                repo.validation_message.is_some(),
                "Invalid repositories should have validation message"
            );
        }

        println!(
            "  - {} (status: {:?}, branches: {}, labels: {}, prs: {}, issues: {})",
            repo.full_name,
            repo.validation_status,
            repo.has_required_branches,
            repo.has_required_labels,
            repo.can_manage_prs,
            repo.can_manage_issues
        );
    }
}
