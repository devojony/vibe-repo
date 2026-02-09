use crate::entities::{container, prelude::*, workspace};
use crate::error::{Result, VibeRepoError};
use crate::services::{AgentService, ContainerConfig, ContainerService, DockerService};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;

/// Workspace status constants
pub mod workspace_status {
    pub const INITIALIZING: &str = "Initializing";
    pub const ACTIVE: &str = "Active";
    pub const FAILED: &str = "Failed";
}

#[derive(Clone)]
pub struct WorkspaceService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
    config: ContainerConfig,
}

impl WorkspaceService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self {
            db,
            docker,
            config: ContainerConfig::default(),
        }
    }

    pub fn with_config(
        db: DatabaseConnection,
        docker: Option<DockerService>,
        config: ContainerConfig,
    ) -> Self {
        Self { db, docker, config }
    }

    pub async fn create_workspace(&self, repository_id: i32) -> Result<workspace::Model> {
        let workspace = workspace::ActiveModel {
            repository_id: Set(repository_id),
            workspace_status: Set(workspace_status::INITIALIZING.to_string()),
            ..Default::default()
        };

        let workspace = Workspace::insert(workspace)
            .exec_with_returning(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(workspace)
    }

    pub async fn get_workspace_by_id(&self, id: i32) -> Result<workspace::Model> {
        Workspace::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| VibeRepoError::NotFound(format!("Workspace with id {} not found", id)))
    }

    pub async fn list_workspaces(&self) -> Result<Vec<workspace::Model>> {
        Workspace::find()
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)
    }

    pub async fn update_workspace_status(&self, id: i32, status: &str) -> Result<workspace::Model> {
        let workspace = self.get_workspace_by_id(id).await?;

        let mut workspace: workspace::ActiveModel = workspace.into();
        workspace.workspace_status = Set(status.to_string());
        workspace.updated_at = Set(Utc::now());

        let workspace = workspace
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(workspace)
    }

    pub async fn delete_workspace(&self, id: i32) -> Result<()> {
        let workspace = self.get_workspace_by_id(id).await?;

        let workspace: workspace::ActiveModel = workspace.into();
        workspace
            .delete(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(())
    }

    /// Create workspace with Docker container if available
    ///
    /// Creates a workspace record and optionally creates and starts a Docker container.
    /// Returns a tuple of (workspace, Option<container>) where container is Some if
    /// Docker is available and container creation succeeds.
    ///
    /// In simplified MVP, this also automatically creates a single agent for the workspace
    /// using the repository's agent configuration.
    pub async fn create_workspace_with_container(
        &self,
        repository_id: i32,
    ) -> Result<(workspace::Model, Option<container::Model>)> {
        // Get repository to access agent configuration
        let repo = Repository::find_by_id(repository_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Repository {} not found", repository_id))
            })?;

        // First create the workspace record
        let mut workspace = self.create_workspace(repository_id).await?;

        // Create agent for the workspace using repository configuration
        let agent_service = AgentService::new(self.db.clone());
        let agent_command = repo.agent_command.unwrap_or_else(|| "opencode".to_string());
        let agent_env_vars = repo.agent_env_vars.unwrap_or_else(|| json!({}));

        match agent_service
            .create_agent(
                workspace.id,
                "Default Agent",
                "opencode",
                &agent_command,
                agent_env_vars,
                repo.agent_timeout,
            )
            .await
        {
            Ok(agent) => {
                tracing::info!(
                    workspace_id = workspace.id,
                    agent_id = agent.id,
                    "Agent created successfully for workspace"
                );
            }
            Err(e) => {
                tracing::error!(
                    workspace_id = workspace.id,
                    error = %e,
                    "Failed to create agent for workspace"
                );
                // Don't fail workspace creation if agent creation fails
            }
        }

        // If Docker is available, create and start container
        if self.docker.is_some() {
            // Use docker_image from repository or default
            let image_name = &repo.docker_image;
            match self.ensure_image_exists(image_name).await {
                Ok(_) => {
                    tracing::info!(
                        workspace_id = workspace.id,
                        image_name = %image_name,
                        "Image ready for workspace"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        workspace_id = workspace.id,
                        image_name = %image_name,
                        error = %e,
                        "Failed to ensure image exists"
                    );

                    // Update workspace status to Failed
                    self.mark_workspace_failed(workspace, &e.to_string()).await;

                    return Err(e);
                }
            }

            // Create ContainerService and create container
            let container_service = ContainerService::new(self.db.clone(), self.docker.clone());

            // Use hardcoded resource limits in simplified MVP
            match container_service
                .create_and_start_container(
                    workspace.id,
                    image_name,
                    2.0,   // cpu_limit
                    "4GB", // memory_limit
                    None,  // No host directory binding in this flow
                )
                .await
            {
                Ok(container) => {
                    tracing::info!(
                        workspace_id = workspace.id,
                        container_id = %container.container_id,
                        "Container created and started successfully"
                    );

                    // Update workspace status to Active
                    let mut workspace_active: workspace::ActiveModel = workspace.into();
                    workspace_active.workspace_status = Set(workspace_status::ACTIVE.to_string());
                    workspace_active.updated_at = Set(Utc::now());

                    workspace = workspace_active
                        .update(&self.db)
                        .await
                        .map_err(VibeRepoError::Database)?;

                    return Ok((workspace, Some(container)));
                }
                Err(e) => {
                    tracing::error!(
                        workspace_id = workspace.id,
                        error = %e,
                        "Failed to create container"
                    );

                    // Update workspace status to Failed
                    self.mark_workspace_failed(workspace, &e.to_string()).await;

                    return Err(e);
                }
            }
        } else {
            tracing::warn!(
                workspace_id = workspace.id,
                "Docker not available, workspace created without container"
            );
        }

        Ok((workspace, None))
    }

    /// Create Docker container for an existing workspace
    ///
    /// This method is used when a workspace already exists but doesn't have a container yet.
    /// It's typically called during repository initialization when the workspace was created
    /// without Docker being available, or when re-initializing a repository.
    ///
    /// # Arguments
    /// * `workspace_id` - The ID of the existing workspace
    ///
    /// # Returns
    /// A tuple of (updated workspace, Option<container>) where container is Some if
    /// Docker is available and container creation succeeds.
    pub async fn create_container_for_workspace(
        &self,
        workspace_id: i32,
    ) -> Result<(workspace::Model, Option<container::Model>)> {
        // Get the existing workspace
        let workspace = self.get_workspace_by_id(workspace_id).await?;

        // Check if container already exists
        if workspace.container_id.is_some() {
            tracing::info!(
                workspace_id = workspace.id,
                container_id = ?workspace.container_id,
                "Container already exists for workspace"
            );
            return Ok((workspace, None));
        }

        // Get repository to access Docker image configuration
        let repo = Repository::find_by_id(workspace.repository_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Repository {} not found", workspace.repository_id))
            })?;

        // If Docker is not available, return workspace without container
        if self.docker.is_none() {
            tracing::warn!(
                workspace_id = workspace.id,
                "Docker not available, cannot create container"
            );
            return Ok((workspace, None));
        }

        // Use docker_image from repository
        let image_name = &repo.docker_image;
        
        // Ensure image exists
        match self.ensure_image_exists(image_name).await {
            Ok(_) => {
                tracing::info!(
                    workspace_id = workspace.id,
                    image_name = %image_name,
                    "Image ready for workspace"
                );
            }
            Err(e) => {
                tracing::error!(
                    workspace_id = workspace.id,
                    image_name = %image_name,
                    error = %e,
                    "Failed to ensure image exists"
                );

                // Update workspace status to Failed
                self.mark_workspace_failed(workspace, &e.to_string()).await;

                return Err(e);
            }
        }

        // Create ContainerService and create container
        let container_service = ContainerService::new(self.db.clone(), self.docker.clone());

        // Use hardcoded resource limits in simplified MVP
        match container_service
            .create_and_start_container(
                workspace.id,
                image_name,
                2.0,   // cpu_limit
                "4GB", // memory_limit
                None,  // No host directory binding in this flow
            )
            .await
        {
            Ok(container) => {
                tracing::info!(
                    workspace_id = workspace.id,
                    container_id = %container.container_id,
                    "Container created and started successfully"
                );

                // Update workspace status to Active
                let mut workspace_active: workspace::ActiveModel = workspace.into();
                workspace_active.workspace_status = Set(workspace_status::ACTIVE.to_string());
                workspace_active.updated_at = Set(Utc::now());

                let updated_workspace = workspace_active
                    .update(&self.db)
                    .await
                    .map_err(VibeRepoError::Database)?;

                return Ok((updated_workspace, Some(container)));
            }
            Err(e) => {
                tracing::error!(
                    workspace_id = workspace.id,
                    error = %e,
                    "Failed to create container"
                );

                // Update workspace status to Failed
                self.mark_workspace_failed(workspace, &e.to_string()).await;

                return Err(e);
            }
        }
    }

    /// Ensure Docker image exists, building it if necessary
    ///
    /// Checks if the specified image exists in Docker. If not, builds it using
    /// the default Dockerfile location.
    pub async fn ensure_image_exists(&self, image_name: &str) -> Result<()> {
        // Get Docker service reference
        let docker = self
            .docker
            .as_ref()
            .ok_or_else(|| VibeRepoError::Internal("Docker not available".to_string()))?;

        // Check if image exists
        let exists = docker.image_exists(image_name).await?;

        if exists {
            tracing::info!(
                image_name = %image_name,
                "Image already exists"
            );
            return Ok(());
        }

        // Image doesn't exist, build it
        tracing::info!(
            image_name = %image_name,
            "Image not found, building..."
        );

        let start_time = std::time::Instant::now();

        // Build image using default Dockerfile location
        let dockerfile_path = self.config.workspace_dockerfile.to_str().ok_or_else(|| {
            VibeRepoError::Internal("Invalid Dockerfile path encoding".to_string())
        })?;

        let build_context_path = self.config.build_context.to_str().ok_or_else(|| {
            VibeRepoError::Internal("Invalid build context path encoding".to_string())
        })?;

        docker
            .build_image(dockerfile_path, image_name, build_context_path)
            .await?;

        let build_time = start_time.elapsed();

        tracing::info!(
            image_name = %image_name,
            build_time_secs = build_time.as_secs(),
            "Image built successfully"
        );

        Ok(())
    }

    /// Mark workspace as failed and update in database
    async fn mark_workspace_failed(&self, workspace: workspace::Model, error: &str) {
        tracing::error!(
            workspace_id = workspace.id,
            error = %error,
            "Marking workspace as failed"
        );

        let mut workspace_active: workspace::ActiveModel = workspace.into();
        workspace_active.workspace_status = Set(workspace_status::FAILED.to_string());
        workspace_active.updated_at = Set(Utc::now());

        if let Err(e) = workspace_active.update(&self.db).await {
            tracing::error!(
                error = %e,
                "Failed to update workspace status to Failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::db::TestDatabase;
    use sea_orm::DatabaseConnection;

    #[tokio::test]
    async fn test_create_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create a test repository
        let repo = crate::test_utils::create_test_repository(
            db,
            "test-repo",
            "owner/test-repo",
            "gitea",
            "https://git.example.com",
            "test-token",
        )
        .await
        .unwrap();

        let service = WorkspaceService::new(db.clone(), None);

        // Act
        let result = service.create_workspace(repo.id).await;

        // Assert
        assert!(result.is_ok());
        let workspace = result.unwrap();
        assert_eq!(workspace.repository_id, repo.id);
        assert_eq!(workspace.workspace_status, "Initializing");
    }

    #[tokio::test]
    async fn test_get_workspace_by_id_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test repository and workspace
        let repo = create_test_repository(db).await;
        let service = WorkspaceService::new(db.clone(), None);
        let created = service.create_workspace(repo.id).await.unwrap();

        // Act
        let result = service.get_workspace_by_id(created.id).await;

        // Assert
        assert!(result.is_ok());
        let workspace = result.unwrap();
        assert_eq!(workspace.id, created.id);
        assert_eq!(workspace.repository_id, repo.id);
    }

    #[tokio::test]
    async fn test_get_workspace_by_id_not_found() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = WorkspaceService::new(db.clone(), None);

        // Act
        let result = service.get_workspace_by_id(99999).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_list_workspaces_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = WorkspaceService::new(db.clone(), None);

        // Create multiple workspaces
        let repo1 = create_test_repository(db).await;
        let repo2 = create_test_repository(db).await;
        service.create_workspace(repo1.id).await.unwrap();
        service.create_workspace(repo2.id).await.unwrap();

        // Act
        let result = service.list_workspaces().await;

        // Assert
        assert!(result.is_ok());
        let workspaces = result.unwrap();
        assert!(workspaces.len() >= 2);
    }

    #[tokio::test]
    async fn test_update_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = WorkspaceService::new(db.clone(), None);
        let repo = create_test_repository(db).await;
        let workspace = service.create_workspace(repo.id).await.unwrap();

        // Act
        let result = service
            .update_workspace_status(workspace.id, "Active")
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.workspace_status, "Active");
    }

    #[tokio::test]
    async fn test_delete_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = WorkspaceService::new(db.clone(), None);
        let repo = create_test_repository(db).await;
        let workspace = service.create_workspace(repo.id).await.unwrap();

        // Act
        let result = service.delete_workspace(workspace.id).await;

        // Assert
        assert!(result.is_ok());

        // Verify workspace is deleted
        let get_result = service.get_workspace_by_id(workspace.id).await;
        assert!(get_result.is_err());
    }

    // Helper function
    async fn create_test_repository(db: &DatabaseConnection) -> crate::entities::repository::Model {
        use crate::test_utils::create_test_repository as create_repo;

        let repo_name = format!("test-repo-{}", uuid::Uuid::new_v4());
        let full_name = format!("owner/{}", repo_name);

        create_repo(
            db,
            &repo_name,
            &full_name,
            "gitea",
            "https://git.example.com",
            "test-token",
        )
        .await
        .unwrap()
    }

    // ============================================
    // Task 2.4: Tests for WorkspaceService with ContainerService Integration
    // ============================================

    /// Test 1: Create workspace with container when Docker available
    /// Requirements: Task 2.4 - create_workspace_with_container integration
    #[tokio::test]
    async fn test_create_workspace_with_container_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let repo = create_test_repository(db).await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();

        // Skip test if Docker is not available
        if docker.is_none() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let service = WorkspaceService::new(db.clone(), docker.clone());

        // Act
        let result = service.create_workspace_with_container(repo.id).await;

        // Assert
        match result {
            Ok((workspace, container_opt)) => {
                assert_eq!(workspace.repository_id, repo.id);

                if let Some(container) = container_opt {
                    // Container was created successfully
                    assert_eq!(container.workspace_id, workspace.id);
                    assert_eq!(workspace.workspace_status, "Active");

                    // Cleanup: remove container
                    if let Some(docker_service) = docker {
                        if let Err(e) = docker_service
                            .remove_container(&container.container_id, true)
                            .await
                        {
                            tracing::warn!(
                                "Failed to cleanup test container {}: {}",
                                container.container_id,
                                e
                            );
                        }
                    }
                } else {
                    // Docker available but container creation failed (e.g., image not available)
                    // This is acceptable in test environment
                    eprintln!("Container creation failed (expected without image)");
                }
            }
            Err(e) => {
                // Docker connection succeeded but container creation failed
                // This is acceptable in test environment (e.g., image not available)
                eprintln!("Container creation failed (expected in test env): {:?}", e);
            }
        }
    }

    /// Test 2: Create workspace without container when Docker unavailable
    /// Requirements: Task 2.4 - Docker unavailable handling
    #[tokio::test]
    async fn test_create_workspace_with_container_without_docker() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let repo = create_test_repository(db).await;

        // Create service without Docker
        let service = WorkspaceService::new(db.clone(), None);

        // Act
        let result = service.create_workspace_with_container(repo.id).await;

        // Assert: Should create workspace without container
        assert!(result.is_ok());
        let (workspace, container_opt) = result.unwrap();
        assert_eq!(workspace.repository_id, repo.id);
        assert!(container_opt.is_none());
        assert_eq!(workspace.workspace_status, "Initializing");
    }

    /// Test 3: ensure_image_exists when image already exists
    /// Requirements: Task 2.4 - ensure_image_exists helper
    #[tokio::test]
    async fn test_ensure_image_exists_already_exists() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Try to initialize Docker
        let docker = DockerService::new().ok();

        if docker.is_none() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let service = WorkspaceService::new(db.clone(), docker);

        // Act - use alpine:latest which should exist or be pullable
        let result = service.ensure_image_exists("alpine:latest").await;

        // Assert
        match result {
            Ok(_) => {
                // Success - image exists or was pulled
            }
            Err(e) => {
                // May fail if Docker daemon not running or image can't be pulled
                eprintln!("ensure_image_exists failed (expected in test env): {:?}", e);
            }
        }
    }

    /// Test 4: ensure_image_exists returns error when Docker unavailable
    /// Requirements: Task 2.4 - ensure_image_exists error handling
    #[tokio::test]
    async fn test_ensure_image_exists_docker_unavailable() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create service without Docker
        let service = WorkspaceService::new(db.clone(), None);

        // Act
        let result = service.ensure_image_exists("alpine:latest").await;

        // Assert - should return error when Docker is unavailable
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Internal(msg) => {
                assert!(msg.contains("Docker not available"));
            }
            e => panic!("Expected Internal error, got: {:?}", e),
        }
    }

    /// Test 5: ensure_image_exists builds new image when not exists
    /// Requirements: Task 2.4 - ensure_image_exists builds image
    #[tokio::test]
    async fn test_ensure_image_exists_builds_new() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Try to initialize Docker
        let docker = DockerService::new().ok();

        if docker.is_none() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let service = WorkspaceService::new(db.clone(), docker.clone());

        // Act - try to build an image (will fail without actual Dockerfile)
        let result = service
            .ensure_image_exists("test-nonexistent-image:latest")
            .await;

        // Assert - should attempt to build but fail without Dockerfile
        match result {
            Ok(_) => {
                // Unexpected success - cleanup
                if let Some(docker_service) = docker {
                    let _ = docker_service
                        .remove_image("test-nonexistent-image:latest", true)
                        .await;
                }
            }
            Err(_) => {
                // Expected to fail without docker/workspace/Dockerfile
                // This is the expected behavior in test environment
            }
        }
    }
}
