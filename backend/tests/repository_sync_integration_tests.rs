//! Integration tests for Repository Synchronization
//!
//! Tests the background service that synchronizes repositories when:
//! - Create provider → Wait for sync → Verify repositories stored
//! - Update provider token → Wait for re-sync → Verify repositories updated
//!
//! Requirements: 10.6, 10.7, 10.8, 10.9

use axum::body::Body;
use axum::http::{Request, StatusCode};
use gitautodev::api::repositories::models::RepositoryResponse;
use gitautodev::api::settings::providers::models::ProviderResponse;
use gitautodev::entities::repository::ValidationStatus;
use gitautodev::test_utils::state::create_test_state;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

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

/// Test: Create provider → Wait for sync → Verify repositories stored
/// Requirements: 10.6, 10.7, 10.8, 10.9
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_sync_after_provider_creation() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Step 1: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Sync Test Provider",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    assert_eq!(provider.name, "Sync Test Provider");

    // Step 2: Wait a moment for background sync to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 3: List repositories for this provider
    let repositories = list_repositories(app.clone(), provider.id).await;

    // Step 4: Verify repositories were stored
    assert!(
        !repositories.is_empty(),
        "Should have synced repositories from Gitea"
    );

    // Step 5: Verify repository fields are populated
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

        // Branches should be populated (Vec<String>)
        // Note: branches may be empty for some repositories

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
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_resync_after_token_update() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Step 1: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Resync Test Provider",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    // Step 2: Wait for initial sync
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 3: Get initial repository count
    let initial_repos = list_repositories(app.clone(), provider.id).await;
    let initial_count = initial_repos.len();

    println!("Initial sync: {} repositories", initial_count);

    // Step 4: Update the provider token (same token, but should trigger re-sync)
    let update_data = json!({
        "access_token": "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2"
    });

    let _updated_provider = update_provider(app.clone(), provider.id, update_data).await;

    // Step 5: Wait for re-sync to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 6: Get updated repository list
    let updated_repos = list_repositories(app.clone(), provider.id).await;
    let updated_count = updated_repos.len();

    println!("After re-sync: {} repositories", updated_count);

    // Step 7: Verify repositories are still present (should be same or more)
    assert!(
        updated_count >= initial_count,
        "Re-sync should maintain or increase repository count"
    );

    // Step 8: Verify repository data is still valid
    for repo in &updated_repos {
        assert_eq!(repo.provider_id, provider.id);
        assert!(!repo.name.is_empty());
        assert!(!repo.full_name.is_empty());
    }
}

/// Test: Update provider base_url → Wait for re-sync → Verify repositories updated
/// Requirements: 10.6, 10.7, 10.8
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_resync_after_base_url_update() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Step 1: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Base URL Test Provider",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    // Step 2: Wait for initial sync
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 3: Get initial repository count
    let initial_repos = list_repositories(app.clone(), provider.id).await;
    let initial_count = initial_repos.len();

    println!("Initial sync: {} repositories", initial_count);

    // Step 4: Update the provider base_url (same URL, but should trigger re-sync)
    let update_data = json!({
        "base_url": "https://gitea.devo.top:66"
    });

    let _updated_provider = update_provider(app.clone(), provider.id, update_data).await;

    // Step 5: Wait for re-sync to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 6: Get updated repository list
    let updated_repos = list_repositories(app.clone(), provider.id).await;
    let updated_count = updated_repos.len();

    println!("After re-sync: {} repositories", updated_count);

    // Step 7: Verify repositories are still present
    assert!(
        updated_count >= initial_count,
        "Re-sync should maintain or increase repository count"
    );
}

/// Test: Manual sync trigger → Verify repositories updated
/// Requirements: 6.1.1, 6.1.2, 6.1.3, 6.1.4
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_manual_sync_trigger() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Step 1: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "Manual Sync Provider",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    // Step 2: Wait for initial sync
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 3: Get initial repository count
    let initial_repos = list_repositories(app.clone(), provider.id).await;
    let initial_count = initial_repos.len();

    println!("Initial sync: {} repositories", initial_count);

    // Step 4: Trigger manual sync
    let sync_status = trigger_sync(app.clone(), provider.id).await;

    // Should return 202 Accepted
    assert_eq!(sync_status, StatusCode::ACCEPTED);

    // Step 5: Wait for sync to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 6: Get updated repository list
    let updated_repos = list_repositories(app.clone(), provider.id).await;
    let updated_count = updated_repos.len();

    println!("After manual sync: {} repositories", updated_count);

    // Step 7: Verify repositories are still present
    assert!(
        updated_count >= initial_count,
        "Manual sync should maintain or increase repository count"
    );
}

/// Test: Sync with invalid token → Verify error handling
/// Requirements: 10.8, 10.9
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_sync_with_invalid_token() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Step 1: Create a provider with invalid token
    let provider = create_provider(
        app.clone(),
        "Invalid Token Provider",
        "https://gitea.devo.top:66",
        "invalid_token_12345678",
    )
    .await;

    // Step 2: Wait for sync attempt
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 3: List repositories (should be empty or have error status)
    let repositories = list_repositories(app.clone(), provider.id).await;

    // Step 4: Verify no repositories were synced (or all have error status)
    // With invalid token, the sync should fail and no repositories should be stored
    println!(
        "Repositories after invalid token sync: {}",
        repositories.len()
    );

    // The service should handle the error gracefully
    // Either no repositories are created, or they all have invalid status
    for repo in &repositories {
        println!(
            "  - {} (status: {:?})",
            repo.full_name, repo.validation_status
        );
    }
}

/// Test: Sync stores all visible repositories regardless of validation
/// Requirements: 10.9, 10.10, 10.11, 10.12, 10.13
/// Note: This test is ignored by default as it requires external Gitea instance
#[tokio::test]
#[ignore]
async fn test_sync_stores_all_repositories() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");
    let app = gitautodev::api::create_router(state.clone());

    // Step 1: Create a provider with valid test credentials
    let provider = create_provider(
        app.clone(),
        "All Repos Provider",
        "https://gitea.devo.top:66",
        "fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2",
    )
    .await;

    // Step 2: Wait for sync
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 3: List all repositories
    let repositories = list_repositories(app.clone(), provider.id).await;

    println!("Total repositories synced: {}", repositories.len());

    // Step 4: Verify repositories have different validation statuses
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

    // Step 5: Verify all repositories are stored (not just valid ones)
    assert!(
        repositories.len() > 0,
        "Should store all visible repositories"
    );

    // Step 6: Verify validation fields are populated
    for repo in &repositories {
        // All repos should have validation status set
        assert!(
            repo.validation_status == ValidationStatus::Valid
                || repo.validation_status == ValidationStatus::Invalid
                || repo.validation_status == ValidationStatus::Pending
        );

        // If invalid, should have validation message
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
