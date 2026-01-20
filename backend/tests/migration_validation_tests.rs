//! Migration validation tests for repository status and soft delete
//!
//! Tests the database migration that adds status, has_workspace, and deleted_at fields.
//! Requirements: Phase 1.1, Phase 1.3

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use vibe_repo::entities::{prelude::*, repository};
use vibe_repo::test_utils::state::create_test_state;

/// Test migration adds status field with default value 'uninitialized'
/// Requirements: 1.1
#[tokio::test]
async fn test_migration_adds_status_field_with_default() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");

    // Create a test provider first
    use vibe_repo::entities::repo_provider;
    let provider = repo_provider::ActiveModel {
        name: Set("Test Provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test_token_12345678".to_string()),
        locked: Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&state.db).await.unwrap();

    // Create a repository without explicitly setting status
    let repo = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("test-repo".to_string()),
        full_name: Set("owner/test-repo".to_string()),
        clone_url: Set("https://gitea.example.com/owner/test-repo.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Pending),
        has_required_branches: Set(false),
        has_required_labels: Set(false),
        can_manage_prs: Set(false),
        can_manage_issues: Set(false),
        validation_message: Set(None),
        ..Default::default()
    };

    let created = repo.insert(&state.db).await.unwrap();

    // Verify status field exists and has default value
    assert_eq!(created.status, repository::RepositoryStatus::Uninitialized);
}

/// Test migration adds has_workspace field with default value false
/// Requirements: 1.1
#[tokio::test]
async fn test_migration_adds_has_workspace_field_with_default() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");

    // Create a test provider first
    use vibe_repo::entities::repo_provider;
    let provider = repo_provider::ActiveModel {
        name: Set("Test Provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test_token_12345678".to_string()),
        locked: Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&state.db).await.unwrap();

    // Create a repository without explicitly setting has_workspace
    let repo = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("test-repo".to_string()),
        full_name: Set("owner/test-repo".to_string()),
        clone_url: Set("https://gitea.example.com/owner/test-repo.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Pending),
        has_required_branches: Set(false),
        has_required_labels: Set(false),
        can_manage_prs: Set(false),
        can_manage_issues: Set(false),
        validation_message: Set(None),
        ..Default::default()
    };

    let created = repo.insert(&state.db).await.unwrap();

    // Verify has_workspace field exists and has default value false
    assert!(!created.has_workspace);
}

/// Test migration adds deleted_at field that can be null
/// Requirements: 1.1
#[tokio::test]
async fn test_migration_adds_deleted_at_field_nullable() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");

    // Create a test provider first
    use vibe_repo::entities::repo_provider;
    let provider = repo_provider::ActiveModel {
        name: Set("Test Provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test_token_12345678".to_string()),
        locked: Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&state.db).await.unwrap();

    // Create a repository without explicitly setting deleted_at
    let repo = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("test-repo".to_string()),
        full_name: Set("owner/test-repo".to_string()),
        clone_url: Set("https://gitea.example.com/owner/test-repo.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Pending),
        has_required_branches: Set(false),
        has_required_labels: Set(false),
        can_manage_prs: Set(false),
        can_manage_issues: Set(false),
        validation_message: Set(None),
        ..Default::default()
    };

    let created = repo.insert(&state.db).await.unwrap();

    // Verify deleted_at field exists and is null by default
    assert!(created.deleted_at.is_none());
}

/// Test can query repositories by status
/// Requirements: 1.1
#[tokio::test]
async fn test_can_query_by_status() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");

    // Create a test provider first
    use vibe_repo::entities::repo_provider;
    let provider = repo_provider::ActiveModel {
        name: Set("Test Provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test_token_12345678".to_string()),
        locked: Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&state.db).await.unwrap();

    // Create repositories with different statuses
    let repo1 = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("repo1".to_string()),
        full_name: Set("owner/repo1".to_string()),
        clone_url: Set("https://gitea.example.com/owner/repo1.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Pending),
        status: Set(repository::RepositoryStatus::Uninitialized),
        has_required_branches: Set(false),
        has_required_labels: Set(false),
        can_manage_prs: Set(false),
        can_manage_issues: Set(false),
        validation_message: Set(None),
        ..Default::default()
    };
    repo1.insert(&state.db).await.unwrap();

    let repo2 = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("repo2".to_string()),
        full_name: Set("owner/repo2".to_string()),
        clone_url: Set("https://gitea.example.com/owner/repo2.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Idle),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        ..Default::default()
    };
    repo2.insert(&state.db).await.unwrap();

    // Query by status
    let uninitialized_repos = Repository::find()
        .filter(repository::Column::Status.eq(repository::RepositoryStatus::Uninitialized))
        .all(&state.db)
        .await
        .unwrap();

    let idle_repos = Repository::find()
        .filter(repository::Column::Status.eq(repository::RepositoryStatus::Idle))
        .all(&state.db)
        .await
        .unwrap();

    // Verify query results
    assert_eq!(uninitialized_repos.len(), 1);
    assert_eq!(uninitialized_repos[0].name, "repo1");
    assert_eq!(idle_repos.len(), 1);
    assert_eq!(idle_repos[0].name, "repo2");
}

/// Test can query repositories by has_workspace
/// Requirements: 1.1
#[tokio::test]
async fn test_can_query_by_has_workspace() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");

    // Create a test provider first
    use vibe_repo::entities::repo_provider;
    let provider = repo_provider::ActiveModel {
        name: Set("Test Provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test_token_12345678".to_string()),
        locked: Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&state.db).await.unwrap();

    // Create repositories with different has_workspace values
    let repo1 = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("repo1".to_string()),
        full_name: Set("owner/repo1".to_string()),
        clone_url: Set("https://gitea.example.com/owner/repo1.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Idle),
        has_workspace: Set(false),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        ..Default::default()
    };
    repo1.insert(&state.db).await.unwrap();

    let repo2 = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("repo2".to_string()),
        full_name: Set("owner/repo2".to_string()),
        clone_url: Set("https://gitea.example.com/owner/repo2.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Active),
        has_workspace: Set(true),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        ..Default::default()
    };
    repo2.insert(&state.db).await.unwrap();

    // Query by has_workspace
    let repos_without_workspace = Repository::find()
        .filter(repository::Column::HasWorkspace.eq(false))
        .all(&state.db)
        .await
        .unwrap();

    let repos_with_workspace = Repository::find()
        .filter(repository::Column::HasWorkspace.eq(true))
        .all(&state.db)
        .await
        .unwrap();

    // Verify query results
    assert_eq!(repos_without_workspace.len(), 1);
    assert_eq!(repos_without_workspace[0].name, "repo1");
    assert_eq!(repos_with_workspace.len(), 1);
    assert_eq!(repos_with_workspace[0].name, "repo2");
}

/// Test can filter out soft-deleted repositories
/// Requirements: 1.1
#[tokio::test]
async fn test_can_filter_soft_deleted_repositories() {
    let state = create_test_state()
        .await
        .expect("Failed to create test state");

    // Create a test provider first
    use vibe_repo::entities::repo_provider;
    let provider = repo_provider::ActiveModel {
        name: Set("Test Provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test_token_12345678".to_string()),
        locked: Set(false),
        ..Default::default()
    };
    let provider = provider.insert(&state.db).await.unwrap();

    // Create a normal repository
    let repo1 = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("repo1".to_string()),
        full_name: Set("owner/repo1".to_string()),
        clone_url: Set("https://gitea.example.com/owner/repo1.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Idle),
        has_workspace: Set(false),
        deleted_at: Set(None),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        ..Default::default()
    };
    repo1.insert(&state.db).await.unwrap();

    // Create a soft-deleted repository
    use chrono::Utc;
    let repo2 = repository::ActiveModel {
        provider_id: Set(provider.id),
        name: Set("repo2".to_string()),
        full_name: Set("owner/repo2".to_string()),
        clone_url: Set("https://gitea.example.com/owner/repo2.git".to_string()),
        default_branch: Set("main".to_string()),
        branches: Set(serde_json::json!(["main"])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Idle),
        has_workspace: Set(false),
        deleted_at: Set(Some(Utc::now())),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        ..Default::default()
    };
    repo2.insert(&state.db).await.unwrap();

    // Query active repositories (deleted_at is null)
    let active_repos = Repository::find()
        .filter(repository::Column::DeletedAt.is_null())
        .all(&state.db)
        .await
        .unwrap();

    // Query deleted repositories (deleted_at is not null)
    let deleted_repos = Repository::find()
        .filter(repository::Column::DeletedAt.is_not_null())
        .all(&state.db)
        .await
        .unwrap();

    // Verify query results
    assert_eq!(active_repos.len(), 1);
    assert_eq!(active_repos[0].name, "repo1");
    assert_eq!(deleted_repos.len(), 1);
    assert_eq!(deleted_repos[0].name, "repo2");
}
