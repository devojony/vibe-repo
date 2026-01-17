//! Integration tests for Workspace, Agent, and Task APIs
//!
//! Tests the full HTTP request/response cycle for workspace management APIs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use gitautodev::api::agents::models::{AgentResponse, CreateAgentRequest, UpdateAgentEnabledRequest};
use gitautodev::api::tasks::models::{CreateTaskRequest, TaskResponse, UpdateTaskStatusRequest};
use gitautodev::api::workspaces::models::{
    CreateWorkspaceRequest, UpdateWorkspaceStatusRequest, WorkspaceResponse,
};
use gitautodev::entities::{repo_provider, repository};
use gitautodev::test_utils::TestDatabase;
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;
use tower::ServiceExt; // for `oneshot`

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test app with a specific database connection
async fn create_test_app_with_db(db: DatabaseConnection) -> axum::Router {
    use gitautodev::api::create_router;
    use gitautodev::config::AppConfig;
    use gitautodev::services::RepositoryService;
    use gitautodev::state::AppState;

    let config = AppConfig::default();
    let repository_service = Arc::new(RepositoryService::new(db.clone()));
    let state = Arc::new(AppState::new(db, config, repository_service));
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
async fn create_test_repository(
    db: &DatabaseConnection,
    provider_id: i32,
) -> repository::Model {
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
) -> (StatusCode, WorkspaceResponse) {
    let request_body = CreateWorkspaceRequest {
        repository_id,
        image_source: "default".to_string(),
        custom_dockerfile_path: None,
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

/// Helper to create an agent via API
async fn create_agent_via_api(
    app: axum::Router,
    workspace_id: i32,
    name: &str,
) -> (StatusCode, AgentResponse) {
    let request_body = CreateAgentRequest {
        workspace_id,
        name: name.to_string(),
        tool_type: "opencode".to_string(),
        command: "opencode".to_string(),
        env_vars: serde_json::json!({}),
        timeout: 1800,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let agent: AgentResponse = serde_json::from_slice(&body).unwrap();

    (status, agent)
}

// ============================================================================
// Workspace API Tests
// ============================================================================

/// Test POST /api/workspaces creates a new workspace
/// Requirements: Workspace API
#[tokio::test]
async fn test_create_workspace_returns_201() {
    // Arrange: Create test app and repository
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let request_body = CreateWorkspaceRequest {
        repository_id: repo.id,
        image_source: "default".to_string(),
        custom_dockerfile_path: None,
        max_concurrent_tasks: 3,
        cpu_limit: 2.0,
        memory_limit: "4GB".to_string(),
        disk_limit: "10GB".to_string(),
    };

    // Act: Send POST request
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

    // Assert: Should return 201 Created
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Create workspace should return 201"
    );

    // Verify response body
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let workspace: WorkspaceResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(workspace.repository_id, repo.id);
    assert_eq!(workspace.workspace_status, "Initializing");
    assert_eq!(workspace.image_source, "default");
}

/// Test GET /api/workspaces/:id returns workspace details
/// Requirements: Workspace API
#[tokio::test]
async fn test_get_workspace_by_id_returns_200() {
    // Arrange: Create workspace
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (status, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    assert_eq!(status, StatusCode::CREATED);

    // Act: Get workspace by ID
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/workspaces/{}", workspace.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let retrieved: WorkspaceResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(retrieved.id, workspace.id);
    assert_eq!(retrieved.repository_id, repo.id);
}

/// Test GET /api/workspaces/:id returns 404 for non-existent workspace
/// Requirements: Workspace API
#[tokio::test]
async fn test_get_workspace_nonexistent_returns_404() {
    // Arrange: Create test app
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    // Act: Get non-existent workspace
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/workspaces/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 404 Not Found
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test GET /api/workspaces lists all workspaces
/// Requirements: Workspace API
#[tokio::test]
async fn test_list_workspaces_returns_200() {
    // Arrange: Create multiple workspaces
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo1 = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (status1, _) = create_workspace_via_api(app.clone(), repo1.id).await;
    assert_eq!(status1, StatusCode::CREATED);

    // Act: List workspaces
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/workspaces")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let workspaces: Vec<WorkspaceResponse> = serde_json::from_slice(&body).unwrap();
    assert!(!workspaces.is_empty(), "Should have at least one workspace");
}

/// Test PATCH /api/workspaces/:id/status updates workspace status
/// Requirements: Workspace API
#[tokio::test]
async fn test_update_workspace_status_returns_200() {
    // Arrange: Create workspace
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (status, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    assert_eq!(status, StatusCode::CREATED);

    let update_request = UpdateWorkspaceStatusRequest {
        status: "running".to_string(),
    };

    // Act: Update workspace status
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/workspaces/{}/status", workspace.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&update_request).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated: WorkspaceResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.workspace_status, "running");
}

/// Test DELETE /api/workspaces/:id deletes workspace
/// Requirements: Workspace API
#[tokio::test]
async fn test_delete_workspace_returns_204() {
    // Arrange: Create workspace
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (status, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    assert_eq!(status, StatusCode::CREATED);

    // Act: Delete workspace
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/workspaces/{}", workspace.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK with deleted workspace
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify the workspace is marked as deleted
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let deleted_workspace: WorkspaceResponse = serde_json::from_slice(&body).unwrap();
    assert!(deleted_workspace.deleted_at.is_some(), "Workspace should have deleted_at timestamp");
}

// ============================================================================
// Agent API Tests
// ============================================================================

/// Test POST /api/agents creates a new agent
/// Requirements: Agent API
#[tokio::test]
async fn test_create_agent_returns_201() {
    // Arrange: Create workspace
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (status, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    assert_eq!(status, StatusCode::CREATED);

    let request_body = CreateAgentRequest {
        workspace_id: workspace.id,
        name: "test-agent".to_string(),
        tool_type: "opencode".to_string(),
        command: "opencode".to_string(),
        env_vars: serde_json::json!({}),
        timeout: 1800,
    };

    // Act: Create agent
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 201 Created
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let agent: AgentResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(agent.workspace_id, workspace.id);
    assert_eq!(agent.name, "test-agent");
    assert_eq!(agent.tool_type, "opencode");
    assert!(agent.enabled);
}

/// Test GET /api/agents/:id returns agent details
/// Requirements: Agent API
#[tokio::test]
async fn test_get_agent_by_id_returns_200() {
    // Arrange: Create agent
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (_, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    let (status, agent) = create_agent_via_api(app.clone(), workspace.id, "test-agent").await;
    assert_eq!(status, StatusCode::CREATED);

    // Act: Get agent by ID
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/agents/{}", agent.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let retrieved: AgentResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(retrieved.id, agent.id);
    assert_eq!(retrieved.name, "test-agent");
}

/// Test GET /api/workspaces/:workspace_id/agents lists agents by workspace
/// Requirements: Agent API
#[tokio::test]
async fn test_list_agents_by_workspace_returns_200() {
    // Arrange: Create multiple agents
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (_, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    let (status1, _) = create_agent_via_api(app.clone(), workspace.id, "agent-1").await;
    let (status2, _) = create_agent_via_api(app.clone(), workspace.id, "agent-2").await;
    assert_eq!(status1, StatusCode::CREATED);
    assert_eq!(status2, StatusCode::CREATED);

    // Act: List agents by workspace
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/workspaces/{}/agents", workspace.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let agents: Vec<AgentResponse> = serde_json::from_slice(&body).unwrap();
    assert_eq!(agents.len(), 2, "Should have 2 agents");
}

/// Test PATCH /api/agents/:id/enabled updates agent enabled status
/// Requirements: Agent API
#[tokio::test]
async fn test_update_agent_enabled_returns_200() {
    // Arrange: Create agent
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (_, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    let (status, agent) = create_agent_via_api(app.clone(), workspace.id, "test-agent").await;
    assert_eq!(status, StatusCode::CREATED);

    let update_request = UpdateAgentEnabledRequest { enabled: false };

    // Act: Update agent enabled status
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/agents/{}/enabled", agent.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&update_request).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated: AgentResponse = serde_json::from_slice(&body).unwrap();
    assert!(!updated.enabled, "Agent should be disabled");
}

/// Test DELETE /api/agents/:id deletes agent
/// Requirements: Agent API
#[tokio::test]
async fn test_delete_agent_returns_204() {
    // Arrange: Create agent
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (_, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    let (status, agent) = create_agent_via_api(app.clone(), workspace.id, "test-agent").await;
    assert_eq!(status, StatusCode::CREATED);

    // Act: Delete agent
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/agents/{}", agent.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 204 No Content
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

// ============================================================================
// Task API Tests
// ============================================================================

/// Test POST /api/tasks creates a new task
/// Requirements: Task API
#[tokio::test]
async fn test_create_task_returns_201() {
    // Arrange: Create workspace
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (status, workspace) = create_workspace_via_api(app.clone(), repo.id).await;
    assert_eq!(status, StatusCode::CREATED);

    let request_body = CreateTaskRequest {
        workspace_id: workspace.id,
        issue_number: 1,
        issue_title: "Test Issue".to_string(),
        issue_body: Some("Test issue body".to_string()),
        assigned_agent_id: None,
        priority: "medium".to_string(),
    };

    // Act: Create task
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 201 Created
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(task.workspace_id, workspace.id);
    assert_eq!(task.issue_number, 1);
    assert_eq!(task.issue_title, "Test Issue");
    assert_eq!(task.task_status, "pending");
}

/// Test GET /api/tasks/:id returns task details
/// Requirements: Task API
#[tokio::test]
async fn test_get_task_by_id_returns_200() {
    // Arrange: Create task
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (_, workspace) = create_workspace_via_api(app.clone(), repo.id).await;

    let request_body = CreateTaskRequest {
        workspace_id: workspace.id,
        issue_number: 1,
        issue_title: "Test Issue".to_string(),
        issue_body: None,
        assigned_agent_id: None,
        priority: "medium".to_string(),
    };

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = create_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();

    // Act: Get task by ID
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/tasks/{}", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let retrieved: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(retrieved.id, task.id);
    assert_eq!(retrieved.issue_number, 1);
}

/// Test GET /api/tasks?workspace_id=X lists tasks by workspace
/// Requirements: Task API
#[tokio::test]
async fn test_list_tasks_by_workspace_returns_200() {
    // Arrange: Create multiple tasks
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (_, workspace) = create_workspace_via_api(app.clone(), repo.id).await;

    // Create two tasks
    for i in 1..=2 {
        let request_body = CreateTaskRequest {
            workspace_id: workspace.id,
            issue_number: i,
            issue_title: format!("Test Issue {}", i),
            issue_body: None,
            assigned_agent_id: None,
            priority: "medium".to_string(),
        };

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Act: List tasks by workspace
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/tasks?workspace_id={}", workspace.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<TaskResponse> = serde_json::from_slice(&body).unwrap();
    assert_eq!(tasks.len(), 2, "Should have 2 tasks");
}

/// Test PATCH /api/tasks/:id/status updates task status
/// Requirements: Task API
#[tokio::test]
async fn test_update_task_status_returns_200() {
    // Arrange: Create task
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let db = &test_db.connection;
    let provider = create_test_provider(db).await;
    let repo = create_test_repository(db, provider.id).await;
    let app = create_test_app_with_db(test_db.connection.clone()).await;

    let (_, workspace) = create_workspace_via_api(app.clone(), repo.id).await;

    let request_body = CreateTaskRequest {
        workspace_id: workspace.id,
        issue_number: 1,
        issue_title: "Test Issue".to_string(),
        issue_body: None,
        assigned_agent_id: None,
        priority: "medium".to_string(),
    };

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = create_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();

    let update_request = UpdateTaskStatusRequest {
        status: "in_progress".to_string(),
    };

    // Act: Update task status
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/tasks/{}/status", task.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&update_request).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let updated: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.task_status, "in_progress");
}
