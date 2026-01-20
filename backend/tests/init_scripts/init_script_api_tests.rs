//! Integration tests for Init Script API endpoints
//!
//! Tests the full HTTP request/response cycle for init script management APIs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;
use tower::ServiceExt;
use vibe_repo::api::workspaces::models::{
    CreateWorkspaceRequest, ExecuteScriptRequest, InitScriptLogsResponse, InitScriptResponse,
    UpdateInitScriptRequest, WorkspaceResponse,
};
use vibe_repo::entities::{repo_provider, repository};
use vibe_repo::test_utils::TestDatabase; // for `oneshot`

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test app with a specific database connection
async fn create_test_app_with_db(db: DatabaseConnection) -> axum::Router {
    use vibe_repo::api::create_router;
    use vibe_repo::config::AppConfig;
    use vibe_repo::services::RepositoryService;
    use vibe_repo::state::AppState;

    let config = Arc::new(AppConfig::default());
    let repository_service = Arc::new(RepositoryService::new(db.clone(), config.clone()));
    let state = Arc::new(AppState::new(db, (*config).clone(), repository_service));
    create_router(state)
}

/// Create a test provider
async fn create_test_provider(db: &DatabaseConnection) -> repo_provider::Model {
    repo_provider::ActiveModel {
        name: Set("test-provider".to_string()),
        provider_type: Set(repo_provider::ProviderType::Gitea),
        base_url: Set("https://gitea.example.com".to_string()),
        access_token: Set("test-token".to_string()),
        locked: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test provider")
}

/// Create a test repository
async fn create_test_repository(db: &DatabaseConnection, provider_id: i32) -> repository::Model {
    repository::ActiveModel {
        provider_id: Set(provider_id),
        name: Set("test-repo".to_string()),
        full_name: Set("org/test-repo".to_string()),
        clone_url: Set("https://gitea.example.com/org/test-repo.git".to_string()),
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
        deleted_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to create test repository")
}

/// Helper to create a workspace via API
async fn create_workspace_via_api(
    app: axum::Router,
    repository_id: i32,
    init_script: Option<String>,
) -> (StatusCode, WorkspaceResponse) {
    let request_body = CreateWorkspaceRequest {
        repository_id,
        init_script,
        script_timeout_seconds: 300,
        image_source: "default".to_string(),
        max_concurrent_tasks: 3,
        cpu_limit: 2.0,
        memory_limit: "4GB".to_string(),
        disk_limit: "10GB".to_string(),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspaces")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let workspace: WorkspaceResponse = serde_json::from_slice(&body).unwrap();

    (status, workspace)
}

// ============================================================================
// Test Cases
// ============================================================================

#[tokio::test]
async fn test_create_workspace_with_init_script() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Act: Create workspace with init script
    let init_script = "#!/bin/bash\necho 'Hello from init script'".to_string();
    let (status, workspace) =
        create_workspace_via_api(app, repository.id, Some(init_script.clone())).await;

    // Assert
    assert_eq!(status, StatusCode::CREATED);
    assert!(workspace.init_script.is_some());

    let script = workspace.init_script.unwrap();
    assert_eq!(script.workspace_id, workspace.id);
    assert_eq!(script.script_content, init_script);
    assert_eq!(script.timeout_seconds, 300);
    assert_eq!(script.status, "Pending");
}

#[tokio::test]
async fn test_create_workspace_without_init_script() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Act: Create workspace without init script
    let (status, workspace) = create_workspace_via_api(app, repository.id, None).await;

    // Assert
    assert_eq!(status, StatusCode::CREATED);
    assert!(workspace.init_script.is_none());
}

#[tokio::test]
async fn test_update_init_script_creates_new() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace without init script
    let (_, workspace) = create_workspace_via_api(app.clone(), repository.id, None).await;

    // Act: Update (create) init script
    let update_request = UpdateInitScriptRequest {
        script_content: "#!/bin/bash\necho 'New script'".to_string(),
        timeout_seconds: 600,
        execute_immediately: false,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{}/init-script", workspace.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let script: InitScriptResponse = serde_json::from_slice(&body).unwrap();

    // Assert
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(script.workspace_id, workspace.id);
    assert_eq!(script.script_content, update_request.script_content);
    assert_eq!(script.timeout_seconds, 600);
    assert_eq!(script.status, "Pending");
}

#[tokio::test]
async fn test_update_init_script_updates_existing() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace with init script
    let init_script = "#!/bin/bash\necho 'Original'".to_string();
    let (_, workspace) =
        create_workspace_via_api(app.clone(), repository.id, Some(init_script)).await;

    // Act: Update existing init script
    let update_request = UpdateInitScriptRequest {
        script_content: "#!/bin/bash\necho 'Updated'".to_string(),
        timeout_seconds: 900,
        execute_immediately: false,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{}/init-script", workspace.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let script: InitScriptResponse = serde_json::from_slice(&body).unwrap();

    // Assert
    assert_eq!(status, StatusCode::OK);
    assert_eq!(script.workspace_id, workspace.id);
    assert_eq!(script.script_content, update_request.script_content);
    assert_eq!(script.timeout_seconds, 900);
    assert_eq!(script.status, "Pending"); // Status should be reset
}

#[tokio::test]
async fn test_get_init_script_logs_not_found() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace without init script
    let (_, workspace) = create_workspace_via_api(app.clone(), repository.id, None).await;

    // Act: Try to get logs for non-existent init script
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/workspaces/{}/init-script/logs", workspace.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();

    // Assert
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_init_script_logs_success() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace with init script
    let init_script = "#!/bin/bash\necho 'Test'".to_string();
    let (_, workspace) =
        create_workspace_via_api(app.clone(), repository.id, Some(init_script)).await;

    // Act: Get logs
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/workspaces/{}/init-script/logs", workspace.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let logs: InitScriptLogsResponse = serde_json::from_slice(&body).unwrap();

    // Assert
    assert_eq!(status, StatusCode::OK);
    assert_eq!(logs.status, "Pending");
    assert!(logs.output_summary.is_none());
    assert!(!logs.has_full_log);
    assert!(logs.executed_at.is_none());
}

#[tokio::test]
async fn test_execute_script_without_container() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace with init script
    let init_script = "#!/bin/bash\necho 'Test'".to_string();
    let (_, workspace) =
        create_workspace_via_api(app.clone(), repository.id, Some(init_script)).await;

    // Act: Try to execute script without container
    let execute_request = ExecuteScriptRequest { force: false };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/workspaces/{}/init-script/execute",
                    workspace.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&execute_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();

    // Assert
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_execute_script_not_found() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace without init script
    let (_, workspace) = create_workspace_via_api(app.clone(), repository.id, None).await;

    // Act: Try to execute non-existent script
    let execute_request = ExecuteScriptRequest { force: false };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/workspaces/{}/init-script/execute",
                    workspace.id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&execute_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();

    // Assert: Returns 400 because workspace has no container (checked before init script existence)
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_download_full_log_not_found() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace with init script (but no execution, so no log file)
    let init_script = "#!/bin/bash\necho 'Test'".to_string();
    let (_, workspace) =
        create_workspace_via_api(app.clone(), repository.id, Some(init_script)).await;

    // Act: Try to download full log when none exists
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/workspaces/{}/init-script/logs/full",
                    workspace.id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();

    // Assert
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_init_script_for_nonexistent_workspace() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let app = create_test_app_with_db(db.clone()).await;

    // Act: Try to update init script for non-existent workspace
    let update_request = UpdateInitScriptRequest {
        script_content: "#!/bin/bash\necho 'Test'".to_string(),
        timeout_seconds: 300,
        execute_immediately: false,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/workspaces/99999/init-script")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();

    // Assert
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_init_script_response_fields() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Act: Create workspace with init script
    let init_script = "#!/bin/bash\necho 'Test'".to_string();
    let (_, workspace) =
        create_workspace_via_api(app, repository.id, Some(init_script.clone())).await;

    // Assert: Verify all fields in init_script response
    let script = workspace.init_script.unwrap();
    assert!(script.id > 0);
    assert_eq!(script.workspace_id, workspace.id);
    assert_eq!(script.script_content, init_script);
    assert_eq!(script.timeout_seconds, 300);
    assert_eq!(script.status, "Pending");
    assert!(script.output_summary.is_none());
    assert!(!script.has_full_log);
    assert!(script.executed_at.is_none());
    assert!(!script.created_at.is_empty());
    assert!(!script.updated_at.is_empty());
}

#[tokio::test]
async fn test_update_init_script_with_custom_timeout() {
    // Setup
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let db = &test_db.connection;

    let provider = create_test_provider(db).await;
    let repository = create_test_repository(db, provider.id).await;

    let app = create_test_app_with_db(db.clone()).await;

    // Create workspace without init script
    let (_, workspace) = create_workspace_via_api(app.clone(), repository.id, None).await;

    // Act: Create init script with custom timeout
    let update_request = UpdateInitScriptRequest {
        script_content: "#!/bin/bash\nsleep 10".to_string(),
        timeout_seconds: 1800, // 30 minutes
        execute_immediately: false,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workspaces/{}/init-script", workspace.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let script: InitScriptResponse = serde_json::from_slice(&body).unwrap();

    // Assert
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(script.timeout_seconds, 1800);
}
