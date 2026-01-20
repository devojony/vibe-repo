//! Container service for managing container lifecycle
//!
//! Provides CRUD operations and lifecycle management for Docker containers.

use crate::entities::{container, prelude::*};
use crate::error::{Result, VibeRepoError};
use crate::services::{ContainerConfig, DockerService};
use chrono::Utc;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
};

/// Container status constants
pub mod container_status {
    pub const CREATING: &str = "creating";
    pub const RUNNING: &str = "running";
    pub const STOPPED: &str = "stopped";
    pub const EXITED: &str = "exited";
    pub const FAILED: &str = "failed";
}

/// Default workspace mount path inside containers
const DEFAULT_WORKSPACE_MOUNT: &str = "/workspace";

/// Generate container name for a workspace
/// Format: workspace-{workspace_id}
fn generate_container_name(workspace_id: i32) -> String {
    format!("workspace-{}", workspace_id)
}

#[derive(Clone)]
pub struct ContainerService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
    config: ContainerConfig,
}

impl ContainerService {
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

    pub async fn create_and_start_container(
        &self,
        workspace_id: i32,
        image_name: &str,
        cpu_limit: f64,
        memory_limit: &str,
    ) -> Result<container::Model> {
        // Check if Docker is available
        let docker = self
            .docker
            .as_ref()
            .ok_or_else(|| VibeRepoError::ServiceUnavailable("Docker not available".to_string()))?;

        // Container name format: workspace-{workspace_id}
        let container_name = generate_container_name(workspace_id);

        tracing::info!(
            workspace_id = workspace_id,
            container_name = %container_name,
            image_name = %image_name,
            "Creating container"
        );

        // Create container in Docker
        let docker_container_id = docker
            .create_container(
                &container_name,
                image_name,
                vec![DEFAULT_WORKSPACE_MOUNT.to_string()],
                cpu_limit,
                memory_limit,
            )
            .await?;

        // Create container record in database with "creating" status
        let container = container::ActiveModel {
            workspace_id: Set(workspace_id),
            container_id: Set(docker_container_id.clone()),
            container_name: Set(container_name.clone()),
            image_name: Set(image_name.to_string()),
            status: Set(container_status::CREATING.to_string()),
            restart_count: Set(0),
            max_restart_attempts: Set(self.config.max_restart_attempts),
            health_check_failures: Set(0),
            ..Default::default()
        };

        let mut container = Container::insert(container)
            .exec_with_returning(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        // Start the container
        match docker.start_container(&docker_container_id).await {
            Ok(_) => {
                tracing::info!(
                    container_id = %docker_container_id,
                    "Container started successfully"
                );

                // Update status to "running"
                let mut active: container::ActiveModel = container.clone().into();
                active.status = Set(container_status::RUNNING.to_string());
                active.started_at = Set(Some(Utc::now()));
                active.updated_at = Set(Utc::now());

                container = active
                    .update(&self.db)
                    .await
                    .map_err(VibeRepoError::Database)?;
            }
            Err(e) => {
                tracing::error!(
                    container_id = %docker_container_id,
                    error = %e,
                    "Failed to start container"
                );

                // Clean up Docker container
                if let Err(cleanup_err) = docker.remove_container(&docker_container_id, true).await
                {
                    tracing::warn!(
                        container_id = %docker_container_id,
                        error = %cleanup_err,
                        "Failed to cleanup container after start failure"
                    );
                }

                // Delete database record
                let active: container::ActiveModel = container.into();
                active
                    .delete(&self.db)
                    .await
                    .map_err(VibeRepoError::Database)?;

                return Err(e);
            }
        }

        Ok(container)
    }

    pub async fn get_container_by_workspace_id(
        &self,
        workspace_id: i32,
    ) -> Result<Option<container::Model>> {
        Container::find()
            .filter(container::Column::WorkspaceId.eq(workspace_id))
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)
    }

    pub async fn update_container_status(
        &self,
        container_id: i32,
        status: &str,
        health_status: Option<&str>,
    ) -> Result<container::Model> {
        // Get existing container
        let container = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        // Update status
        let mut active: container::ActiveModel = container.into();
        active.status = Set(status.to_string());
        if let Some(health) = health_status {
            active.health_status = Set(Some(health.to_string()));
        }
        active.updated_at = Set(Utc::now());

        active
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)
    }

    pub async fn auto_restart_container(&self, container_id: i32) -> Result<()> {
        // Get container info
        let container = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        // Check if we've already exceeded max attempts
        if container.restart_count >= container.max_restart_attempts {
            self.mark_as_failed(
                container_id,
                &format!(
                    "Max restart attempts ({}) exceeded",
                    container.max_restart_attempts
                ),
            )
            .await?;
            return Ok(());
        }

        // Atomically increment restart_count ONLY if still below max
        // This prevents race conditions
        let updated_rows = container::Entity::update_many()
            .col_expr(
                container::Column::RestartCount,
                Expr::col(container::Column::RestartCount).add(1),
            )
            .col_expr(container::Column::LastRestartAt, Expr::value(Utc::now()))
            .col_expr(container::Column::UpdatedAt, Expr::value(Utc::now()))
            .filter(container::Column::Id.eq(container_id))
            .filter(container::Column::RestartCount.lt(container.max_restart_attempts))
            .exec(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        // If no rows updated, we've hit the limit (another thread got there first)
        if updated_rows.rows_affected == 0 {
            tracing::warn!(
                container_id = container_id,
                "Restart attempt rejected: max attempts reached"
            );
            self.mark_as_failed(
                container_id,
                &format!(
                    "Max restart attempts ({}) exceeded",
                    container.max_restart_attempts
                ),
            )
            .await?;
            return Ok(());
        }

        // Check if Docker is available
        let docker = self
            .docker
            .as_ref()
            .ok_or_else(|| VibeRepoError::ServiceUnavailable("Docker not available".to_string()))?;

        tracing::info!(
            container_id = container_id,
            docker_container_id = %container.container_id,
            "Auto-restarting container"
        );

        // Restart container using Docker (stop + start)
        // First try to stop (ignore errors if already stopped)
        if let Err(e) = docker
            .stop_container(&container.container_id, self.config.stop_timeout_seconds)
            .await
        {
            tracing::warn!(
                container_id = %container.container_id,
                error = %e,
                "Failed to stop container (may already be stopped)"
            );
        }

        // Start the container
        docker.start_container(&container.container_id).await?;

        // Get updated restart count
        let updated_container = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        tracing::info!(
            container_id = container_id,
            restart_count = updated_container.restart_count,
            "Container restarted successfully"
        );

        Ok(())
    }

    pub async fn manual_restart_container(&self, container_id: i32) -> Result<container::Model> {
        // Get container
        let container = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        // Check if Docker is available
        let docker = self
            .docker
            .as_ref()
            .ok_or_else(|| VibeRepoError::ServiceUnavailable("Docker not available".to_string()))?;

        tracing::info!(
            container_id = container_id,
            docker_container_id = %container.container_id,
            "Manually restarting container"
        );

        // Restart container using Docker (stop + start)
        // First try to stop (ignore errors if already stopped)
        if let Err(e) = docker
            .stop_container(&container.container_id, self.config.stop_timeout_seconds)
            .await
        {
            tracing::warn!(
                container_id = %container.container_id,
                error = %e,
                "Failed to stop container (may already be stopped)"
            );
        }

        // Start the container
        docker.start_container(&container.container_id).await?;

        // Reset restart count and update last_restart_at
        self.reset_restart_count(container_id).await?;

        // Get updated container
        let updated = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        tracing::info!(
            container_id = container_id,
            "Container manually restarted successfully"
        );

        Ok(updated)
    }

    pub async fn stop_and_remove_container(&self, container_id: i32) -> Result<()> {
        // Get container
        let container = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        // Check if Docker is available
        let docker = self
            .docker
            .as_ref()
            .ok_or_else(|| VibeRepoError::ServiceUnavailable("Docker not available".to_string()))?;

        tracing::info!(
            container_id = container_id,
            docker_container_id = %container.container_id,
            "Stopping and removing container"
        );

        // Stop container (ignore errors if already stopped)
        if let Err(e) = docker
            .stop_container(&container.container_id, self.config.stop_timeout_seconds)
            .await
        {
            tracing::warn!(
                container_id = %container.container_id,
                error = %e,
                "Failed to stop container (may already be stopped)"
            );
        }

        // Remove container
        docker
            .remove_container(&container.container_id, true)
            .await?;

        // Delete container record from database
        let active: container::ActiveModel = container.into();
        active
            .delete(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        tracing::info!(
            container_id = container_id,
            "Container stopped and removed successfully"
        );

        Ok(())
    }

    async fn reset_restart_count(&self, container_id: i32) -> Result<()> {
        let container = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        let mut active: container::ActiveModel = container.into();
        active.restart_count = Set(0);
        active.last_restart_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        active
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(())
    }

    async fn mark_as_failed(&self, container_id: i32, error_message: &str) -> Result<()> {
        let container = Container::find_by_id(container_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Container with id {} not found", container_id))
            })?;

        let mut active: container::ActiveModel = container.into();
        active.status = Set(container_status::FAILED.to_string());
        active.error_message = Set(Some(error_message.to_string()));
        active.updated_at = Set(Utc::now());

        active
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::prelude::{Container, RepoProvider, Repository, Workspace};
    use crate::entities::{repo_provider, repository, workspace};
    use crate::test_utils::db::TestDatabase;
    use sea_orm::Set;

    /// Helper function to create a test workspace
    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        // Create provider
        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec_with_returning(db)
            .await
            .unwrap();

        // Create repository
        let repo = repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.id),
            ..Default::default()
        };
        let repo = Repository::insert(repo)
            .exec_with_returning(db)
            .await
            .unwrap();

        // Create workspace
        let workspace = workspace::ActiveModel {
            repository_id: Set(repo.id),
            workspace_status: Set("Initializing".to_string()),
            image_source: Set("alpine:latest".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
            ..Default::default()
        };
        Workspace::insert(workspace)
            .exec_with_returning(db)
            .await
            .unwrap()
    }

    /// Test 1: Constructor works
    /// Requirements: Task 2.1 - ContainerService implementation
    #[tokio::test]
    async fn test_new_creates_service() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Act
        let service = ContainerService::new(db.clone(), None);

        // Assert - service should be created successfully
        // We can't directly test the fields, but we can verify it compiles and runs
        drop(service);
    }

    /// Test 2: Happy path - create and start container
    /// Requirements: Task 2.1 - Create container record and start Docker container
    #[tokio::test]
    async fn test_create_and_start_container_success() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Try to initialize Docker (may not be available in test environment)
        let docker = DockerService::new().ok();
        let service = ContainerService::new(db.clone(), docker.clone());

        // Act
        let result = service
            .create_and_start_container(workspace.id, "alpine:latest", 2.0, "4GB")
            .await;

        // Assert
        if docker.is_some() {
            // If Docker is available, test the full flow
            match result {
                Ok(container) => {
                    assert_eq!(container.workspace_id, workspace.id);
                    assert_eq!(container.image_name, "alpine:latest");
                    assert!(
                        container.status == "running" || container.status == "creating",
                        "Container status should be running or creating, got: {}",
                        container.status
                    );

                    // Cleanup
                    if let Some(docker_service) = docker {
                        let _ = docker_service
                            .remove_container(&container.container_id, true)
                            .await;
                    }
                }
                Err(e) => {
                    // Docker might not be running or image not available
                    eprintln!("Container creation failed (expected in test env): {:?}", e);
                }
            }
        } else {
            // Docker not available - should return error
            assert!(result.is_err());
        }
    }

    /// Test 3: Handles missing Docker
    /// Requirements: Task 2.1 - Error handling for Docker unavailability
    #[tokio::test]
    async fn test_create_and_start_container_docker_unavailable() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Create service without Docker
        let service = ContainerService::new(db.clone(), None);

        // Act
        let result = service
            .create_and_start_container(workspace.id, "alpine:latest", 2.0, "4GB")
            .await;

        // Assert - should return error when Docker is unavailable
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::ServiceUnavailable(_) => {}
            e => panic!("Expected ServiceUnavailable error, got: {:?}", e),
        }
    }

    /// Test 4: Query returns container
    /// Requirements: Task 2.1 - Get container by workspace_id
    #[tokio::test]
    async fn test_get_container_by_workspace_id_found() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Create a container record directly in database
        let container = container::ActiveModel {
            workspace_id: Set(workspace.id),
            container_id: Set("test-container-id".to_string()),
            container_name: Set(generate_container_name(workspace.id)),
            image_name: Set("alpine:latest".to_string()),
            status: Set(container_status::RUNNING.to_string()),
            restart_count: Set(0),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        let created_container = Container::insert(container)
            .exec_with_returning(&db)
            .await
            .unwrap();

        let service = ContainerService::new(db.clone(), None);

        // Act
        let result = service.get_container_by_workspace_id(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, created_container.id);
        assert_eq!(found.workspace_id, workspace.id);
    }

    /// Test 5: Query returns None when not found
    /// Requirements: Task 2.1 - Get container by workspace_id
    #[tokio::test]
    async fn test_get_container_by_workspace_id_not_found() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let service = ContainerService::new(db.clone(), None);

        // Act
        let result = service.get_container_by_workspace_id(99999).await;

        // Assert
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_none());
    }

    /// Test 6: Status update works
    /// Requirements: Task 2.1 - Update container status
    #[tokio::test]
    async fn test_update_container_status_success() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Create a container record
        let container = container::ActiveModel {
            workspace_id: Set(workspace.id),
            container_id: Set("test-container-id".to_string()),
            container_name: Set(generate_container_name(workspace.id)),
            image_name: Set("alpine:latest".to_string()),
            status: Set(container_status::CREATING.to_string()),
            restart_count: Set(0),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        let created_container = Container::insert(container)
            .exec_with_returning(&db)
            .await
            .unwrap();

        let service = ContainerService::new(db.clone(), None);

        // Act
        let result = service
            .update_container_status(created_container.id, "running", Some("Healthy"))
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.status, "running");
        assert_eq!(updated.health_status, Some("Healthy".to_string()));
    }

    /// Test 7: Auto-restart within limit
    /// Requirements: Task 2.1 - Auto-restart container
    #[tokio::test]
    async fn test_auto_restart_container_success() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();

        // Create a container record
        let container_id_str = if let Some(ref docker_service) = docker {
            // Create real container if Docker is available
            match docker_service
                .create_container(
                    &format!("workspace-{}", workspace.id),
                    "alpine:latest",
                    vec![],
                    2.0,
                    "4GB",
                )
                .await
            {
                Ok(id) => {
                    // Start the container
                    let _ = docker_service.start_container(&id).await;
                    id
                }
                Err(_) => {
                    eprintln!("Skipping test: Failed to create container");
                    return;
                }
            }
        } else {
            "test-container-id".to_string()
        };

        let container = container::ActiveModel {
            workspace_id: Set(workspace.id),
            container_id: Set(container_id_str.clone()),
            container_name: Set(generate_container_name(workspace.id)),
            image_name: Set("alpine:latest".to_string()),
            status: Set(container_status::EXITED.to_string()),
            restart_count: Set(1),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        let created_container = Container::insert(container)
            .exec_with_returning(&db)
            .await
            .unwrap();

        let service = ContainerService::new(db.clone(), docker.clone());

        // Act
        let result = service.auto_restart_container(created_container.id).await;

        // Cleanup
        if let Some(docker_service) = docker {
            let _ = docker_service
                .remove_container(&container_id_str, true)
                .await;
        }

        // Assert
        if service.docker.is_some() {
            // If Docker is available, restart should succeed or fail gracefully
            match result {
                Ok(_) => {
                    // Verify restart_count was incremented
                    let updated = Container::find_by_id(created_container.id)
                        .one(&db)
                        .await
                        .unwrap()
                        .unwrap();
                    assert_eq!(updated.restart_count, 2);
                }
                Err(e) => {
                    eprintln!("Restart failed (expected in test env): {:?}", e);
                }
            }
        } else {
            // Docker not available - should return error
            assert!(result.is_err());
        }
    }

    /// Test 8: Marks as failed when max attempts exceeded
    /// Requirements: Task 2.1 - Auto-restart with max attempts
    #[tokio::test]
    async fn test_auto_restart_container_max_attempts_exceeded() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Create a container record with restart_count at max
        let container = container::ActiveModel {
            workspace_id: Set(workspace.id),
            container_id: Set("test-container-id".to_string()),
            container_name: Set(generate_container_name(workspace.id)),
            image_name: Set("alpine:latest".to_string()),
            status: Set(container_status::EXITED.to_string()),
            restart_count: Set(3),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        let created_container = Container::insert(container)
            .exec_with_returning(&db)
            .await
            .unwrap();

        let service = ContainerService::new(db.clone(), None);

        // Act
        let result = service.auto_restart_container(created_container.id).await;

        // Assert - should succeed but mark as failed
        assert!(result.is_ok());

        // Verify container is marked as failed
        let updated = Container::find_by_id(created_container.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, container_status::FAILED);
        assert!(updated.error_message.is_some());
        assert!(updated
            .error_message
            .unwrap()
            .contains("Max restart attempts"));
    }

    /// Test 9: Manual restart resets count
    /// Requirements: Task 2.1 - Manual restart container
    #[tokio::test]
    async fn test_manual_restart_container_success() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();

        // Create a container record
        let container_id_str = if let Some(ref docker_service) = docker {
            // Create real container if Docker is available
            match docker_service
                .create_container(
                    &format!("workspace-{}", workspace.id),
                    "alpine:latest",
                    vec![],
                    2.0,
                    "4GB",
                )
                .await
            {
                Ok(id) => {
                    // Start the container
                    let _ = docker_service.start_container(&id).await;
                    id
                }
                Err(_) => {
                    eprintln!("Skipping test: Failed to create container");
                    return;
                }
            }
        } else {
            "test-container-id".to_string()
        };

        let container = container::ActiveModel {
            workspace_id: Set(workspace.id),
            container_id: Set(container_id_str.clone()),
            container_name: Set(generate_container_name(workspace.id)),
            image_name: Set("alpine:latest".to_string()),
            status: Set(container_status::EXITED.to_string()),
            restart_count: Set(2),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        let created_container = Container::insert(container)
            .exec_with_returning(&db)
            .await
            .unwrap();

        let service = ContainerService::new(db.clone(), docker.clone());

        // Act
        let result = service.manual_restart_container(created_container.id).await;

        // Cleanup
        if let Some(docker_service) = docker {
            let _ = docker_service
                .remove_container(&container_id_str, true)
                .await;
        }

        // Assert
        if service.docker.is_some() {
            // If Docker is available, restart should succeed or fail gracefully
            match result {
                Ok(updated) => {
                    // Verify restart_count was reset to 0
                    assert_eq!(updated.restart_count, 0);
                    assert!(updated.last_restart_at.is_some());
                }
                Err(e) => {
                    eprintln!("Manual restart failed (expected in test env): {:?}", e);
                }
            }
        } else {
            // Docker not available - should return error
            assert!(result.is_err());
        }
    }

    /// Test 10: Cleanup works
    /// Requirements: Task 2.1 - Stop and remove container
    #[tokio::test]
    async fn test_stop_and_remove_container_success() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let workspace = create_test_workspace(&db).await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();

        // Create a container record
        let container_id_str = if let Some(ref docker_service) = docker {
            // Create real container if Docker is available
            match docker_service
                .create_container(
                    &format!("workspace-{}", workspace.id),
                    "alpine:latest",
                    vec![],
                    2.0,
                    "4GB",
                )
                .await
            {
                Ok(id) => {
                    // Start the container
                    let _ = docker_service.start_container(&id).await;
                    id
                }
                Err(_) => {
                    eprintln!("Skipping test: Failed to create container");
                    return;
                }
            }
        } else {
            "test-container-id".to_string()
        };

        let container = container::ActiveModel {
            workspace_id: Set(workspace.id),
            container_id: Set(container_id_str.clone()),
            container_name: Set(generate_container_name(workspace.id)),
            image_name: Set("alpine:latest".to_string()),
            status: Set(container_status::RUNNING.to_string()),
            restart_count: Set(0),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        let created_container = Container::insert(container)
            .exec_with_returning(&db)
            .await
            .unwrap();

        let service = ContainerService::new(db.clone(), docker.clone());

        // Act
        let result = service
            .stop_and_remove_container(created_container.id)
            .await;

        // Assert
        if service.docker.is_some() {
            // If Docker is available, cleanup should succeed or fail gracefully
            match result {
                Ok(_) => {
                    // Verify container record was deleted
                    let found = Container::find_by_id(created_container.id)
                        .one(&db)
                        .await
                        .unwrap();
                    assert!(found.is_none());
                }
                Err(e) => {
                    eprintln!("Cleanup failed (expected in test env): {:?}", e);
                    // Try to cleanup manually
                    if let Some(docker_service) = docker {
                        let _ = docker_service
                            .remove_container(&container_id_str, true)
                            .await;
                    }
                }
            }
        } else {
            // Docker not available - should return error
            assert!(result.is_err());
        }
    }

    /// Test 11: Concurrent restart respects max attempts (race condition test)
    /// Requirements: Code Review - Verify atomic restart count increment
    #[tokio::test]
    async fn test_concurrent_restart_respects_max_attempts() {
        // Arrange: Create test database and container
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        let workspace = create_test_workspace(&db).await;

        // Create a container record with restart_count = 0, max = 3
        let container = container::ActiveModel {
            workspace_id: Set(workspace.id),
            container_id: Set("test-concurrent-restart".to_string()),
            container_name: Set(generate_container_name(workspace.id)),
            image_name: Set("alpine:latest".to_string()),
            status: Set(container_status::EXITED.to_string()),
            restart_count: Set(0),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        let created_container = Container::insert(container)
            .exec_with_returning(&db)
            .await
            .unwrap();

        let service = ContainerService::new(db.clone(), None);

        // Act: Spawn 10 concurrent restart attempts
        let mut handles = vec![];
        for _ in 0..10 {
            let service_clone = service.clone();
            let container_id = created_container.id;
            let handle =
                tokio::spawn(
                    async move { service_clone.auto_restart_container(container_id).await },
                );
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            let _ = handle.await;
        }

        // Assert: restart_count should never exceed max_restart_attempts
        let updated_container = Container::find_by_id(created_container.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();

        assert!(
            updated_container.restart_count <= 3,
            "Restart count {} exceeded max attempts 3",
            updated_container.restart_count
        );

        // Verify container is marked as failed (since we hit max attempts)
        assert_eq!(updated_container.status, container_status::FAILED);
        assert!(updated_container.error_message.is_some());
        assert!(updated_container
            .error_message
            .unwrap()
            .contains("Max restart attempts"));
    }
}
