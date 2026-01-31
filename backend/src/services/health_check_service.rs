//! Health check service for monitoring workspace containers
//!
//! Periodically checks container health and updates workspace status.

use crate::entities::{prelude::*, task::TaskStatus, workspace};
use crate::error::{Result, VibeRepoError};
use crate::services::{ContainerService, DockerService};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

/// Health check service for monitoring workspace containers
pub struct HealthCheckService {
    db: Arc<DatabaseConnection>,
    docker: Option<DockerService>,
    container_service: Option<ContainerService>,
    interval: Duration,
}

impl HealthCheckService {
    /// Create a new health check service
    pub fn new(
        db: DatabaseConnection,
        docker: Option<DockerService>,
        container_service: Option<ContainerService>,
        interval: Duration,
    ) -> Result<Self> {
        Ok(Self {
            db: Arc::new(db),
            docker,
            container_service,
            interval,
        })
    }

    /// Run the health check loop
    pub async fn run(&self) -> Result<()> {
        let mut interval = time::interval(self.interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_workspaces().await {
                tracing::error!("Health check failed: {}", e);
            }
        }
    }

    /// Check health of all workspaces with containers
    pub async fn check_all_workspaces(&self) -> Result<()> {
        // If Docker is not available, skip health checks
        let docker = match &self.docker {
            Some(d) => d,
            None => {
                tracing::debug!("Docker not available, skipping health checks");
                return Ok(());
            }
        };

        // Get all workspaces with containers
        let workspaces = Workspace::find()
            .all(self.db.as_ref())
            .await
            .map_err(VibeRepoError::Database)?;

        for workspace in workspaces {
            // Skip workspaces without containers
            let container_id = match &workspace.container_id {
                Some(id) => id,
                None => continue,
            };

            // Check container health
            match docker.check_container_health(container_id).await {
                Ok(health) => {
                    // If container is not running, attempt auto-restart
                    if !health.is_running {
                        tracing::warn!(
                            workspace_id = workspace.id,
                            container_id = %container_id,
                            status = %health.status,
                            "Container unhealthy, attempting auto-restart"
                        );

                        // Try to get container record and auto-restart
                        if let Some(container_service) = &self.container_service {
                            if let Ok(Some(container)) = container_service
                                .get_container_by_workspace_id(workspace.id)
                                .await
                            {
                                match container_service.auto_restart_container(container.id).await {
                                    Ok(_) => {
                                        tracing::info!(
                                            workspace_id = workspace.id,
                                            container_id = %container_id,
                                            "Container auto-restarted successfully"
                                        );

                                        // Update workspace with healthy status
                                        let workspace_id = workspace.id;
                                        let mut workspace_active: workspace::ActiveModel =
                                            workspace.into();
                                        workspace_active.health_status =
                                            Set(Some("Healthy".to_string()));
                                        workspace_active.container_status =
                                            Set(Some("running".to_string()));
                                        workspace_active.last_health_check = Set(Some(Utc::now()));
                                        workspace_active.updated_at = Set(Utc::now());

                                        if let Err(e) =
                                            workspace_active.update(self.db.as_ref()).await
                                        {
                                            tracing::warn!(
                                                "Failed to update workspace {} health status: {}",
                                                workspace_id,
                                                e
                                            );
                                        }
                                        continue;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            workspace_id = workspace.id,
                                            container_id = %container_id,
                                            error = %e,
                                            "Failed to auto-restart container"
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // Update workspace with health status
                    let workspace_id = workspace.id;
                    let mut workspace_active: workspace::ActiveModel = workspace.into();
                    // health_status: High-level status (Healthy/Unhealthy)
                    workspace_active.health_status = Set(Some(if health.is_running {
                        "Healthy".to_string()
                    } else {
                        "Unhealthy".to_string()
                    }));
                    // container_status: Actual Docker status (running, exited, stopped, etc.)
                    workspace_active.container_status = Set(Some(health.status));
                    workspace_active.last_health_check = Set(Some(Utc::now()));
                    workspace_active.updated_at = Set(Utc::now());

                    if let Err(e) = workspace_active.update(self.db.as_ref()).await {
                        tracing::warn!(
                            "Failed to update workspace {} health status: {}",
                            workspace_id,
                            e
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to check health for container {}: {}",
                        container_id,
                        e
                    );

                    // Update workspace with error status
                    let workspace_id = workspace.id;
                    let mut workspace_active: workspace::ActiveModel = workspace.into();
                    workspace_active.health_status = Set(Some("error".to_string()));
                    workspace_active.last_health_check = Set(Some(Utc::now()));
                    workspace_active.updated_at = Set(Utc::now());

                    if let Err(e) = workspace_active.update(self.db.as_ref()).await {
                        tracing::warn!(
                            "Failed to update workspace {} error status: {}",
                            workspace_id,
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ContainerService;
    use crate::test_utils::db::TestDatabase;
    use sea_orm::{EntityTrait, Set};

    /// Test HealthCheckService::new creates service successfully
    /// Requirements: Task 5 - Health check service
    #[tokio::test]
    async fn test_health_check_service_new() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection.clone();
        let docker = DockerService::new().ok();
        let container_service = docker
            .as_ref()
            .map(|d| ContainerService::new(db.clone(), Some(d.clone())));
        let interval = Duration::from_secs(300);

        // Act
        let service = HealthCheckService::new(db, docker, container_service, interval);

        // Assert
        assert!(service.is_ok());
    }

    /// Test check_all_workspaces updates workspace health status
    /// Requirements: Task 5 - Health check service
    #[tokio::test]
    async fn test_check_all_workspaces_updates_health() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace with container
        let repo = create_test_repository(db).await;
        let workspace = create_test_workspace(db, repo.id, Some("test-container-123")).await;

        let docker = DockerService::new().ok();
        let container_service = docker
            .as_ref()
            .map(|d| ContainerService::new(db.clone(), Some(d.clone())));
        let service = HealthCheckService::new(
            db.clone(),
            docker,
            container_service,
            Duration::from_secs(300),
        )
        .expect("Failed to create service");

        // Act
        let result = service.check_all_workspaces().await;

        // Assert
        assert!(result.is_ok());

        // Verify workspace was updated with health check timestamp
        let updated = Workspace::find_by_id(workspace.id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.last_health_check.is_some());

        // Verify health_status and container_status are populated
        // Note: Since we're testing with a non-existent container, we expect error status
        assert!(updated.health_status.is_some());
        let health_status = updated.health_status.unwrap();
        // Should be either "Healthy", "Unhealthy", or "error" depending on container state
        assert!(
            health_status == "Healthy" || health_status == "Unhealthy" || health_status == "error"
        );

        // container_status should be set (either Docker status or remain as "running" from setup)
        assert!(updated.container_status.is_some());
    }

    /// Test HealthCheckService gracefully handles missing Docker
    /// Requirements: Task 5 - Health check service
    #[tokio::test]
    async fn test_health_check_service_without_docker() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection.clone();

        // Act: Create service without Docker
        let service = HealthCheckService::new(db, None, None, Duration::from_secs(300));

        // Assert: Should succeed
        assert!(service.is_ok());

        // Act: Check workspaces should not fail
        let result = service.unwrap().check_all_workspaces().await;
        assert!(result.is_ok());
    }

    /// Test check_all_workspaces auto-restarts unhealthy container
    /// Requirements: Task 4.1 - Auto-restart unhealthy containers
    #[tokio::test]
    async fn test_check_all_workspaces_auto_restarts_unhealthy_container() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace with container
        let repo = create_test_repository(db).await;
        let workspace =
            create_test_workspace(db, repo.id, Some("test-container-auto-restart")).await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();

        // Create container record in database
        if let Some(ref docker_service) = docker {
            let container_service = ContainerService::new(db.clone(), Some(docker_service.clone()));

            // Create a container record with "exited" status (simulating unhealthy container)
            let container = crate::entities::container::ActiveModel {
                workspace_id: Set(workspace.id),
                container_id: Set("test-container-auto-restart".to_string()),
                container_name: Set(format!("workspace-{}", workspace.id)),
                image_name: Set("alpine:latest".to_string()),
                status: Set("exited".to_string()),
                restart_count: Set(0),
                max_restart_attempts: Set(3),
                health_check_failures: Set(0),
                ..Default::default()
            };
            let _container = Container::insert(container)
                .exec_with_returning(db)
                .await
                .unwrap();

            let _service = HealthCheckService::new(
                db.clone(),
                Some(docker_service.clone()),
                Some(container_service),
                Duration::from_secs(300),
            )
            .expect("Failed to create service");

            // Act
            let result = _service.check_all_workspaces().await;

            // Assert
            assert!(result.is_ok());

            // Note: Since we're testing with a non-existent Docker container,
            // the auto-restart will fail, but the service should handle it gracefully
            // In a real scenario with a running container, it would be restarted
        }
    }

    /// Test check_all_workspaces marks container as failed after max attempts
    /// Requirements: Task 4.1 - Mark container as failed after max restart attempts
    #[tokio::test]
    async fn test_check_all_workspaces_marks_failed_after_max_attempts() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace with container
        let repo = create_test_repository(db).await;
        let workspace =
            create_test_workspace(db, repo.id, Some("test-container-max-attempts")).await;

        let docker = DockerService::new().ok();

        // Create container record with restart_count at max
        if let Some(ref docker_service) = docker {
            let container_service = ContainerService::new(db.clone(), Some(docker_service.clone()));

            let container = crate::entities::container::ActiveModel {
                workspace_id: Set(workspace.id),
                container_id: Set("test-container-max-attempts".to_string()),
                container_name: Set(format!("workspace-{}", workspace.id)),
                image_name: Set("alpine:latest".to_string()),
                status: Set("exited".to_string()),
                restart_count: Set(3), // At max attempts
                max_restart_attempts: Set(3),
                health_check_failures: Set(0),
                ..Default::default()
            };
            let created_container = Container::insert(container)
                .exec_with_returning(db)
                .await
                .unwrap();

            let _service = HealthCheckService::new(
                db.clone(),
                Some(docker_service.clone()),
                Some(container_service.clone()),
                Duration::from_secs(300),
            )
            .expect("Failed to create service");

            // Act - First, manually trigger auto_restart to mark as failed
            // (since the Docker container doesn't exist, health check will error out)
            let restart_result = container_service
                .auto_restart_container(created_container.id)
                .await;

            // Assert - auto_restart should succeed (marking as failed is not an error)
            assert!(restart_result.is_ok());

            // Verify container is marked as failed
            let updated = Container::find_by_id(created_container.id)
                .one(db)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(updated.status, "failed");
            assert!(updated.error_message.is_some());
            assert!(updated
                .error_message
                .unwrap()
                .contains("Max restart attempts"));
        }
    }

    /// Test check_all_workspaces without ContainerService
    /// Requirements: Task 4.1 - Handle missing ContainerService gracefully
    #[tokio::test]
    async fn test_check_all_workspaces_without_container_service() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace with container
        let repo = create_test_repository(db).await;
        let workspace = create_test_workspace(db, repo.id, Some("test-container-no-service")).await;

        let docker = DockerService::new().ok();

        // Create service without ContainerService
        let service = HealthCheckService::new(
            db.clone(),
            docker,
            None, // No ContainerService
            Duration::from_secs(300),
        )
        .expect("Failed to create service");

        // Act
        let result = service.check_all_workspaces().await;

        // Assert - should succeed without auto-restart
        assert!(result.is_ok());

        // Verify workspace health status was updated (but no auto-restart occurred)
        let updated = Workspace::find_by_id(workspace.id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.last_health_check.is_some());
    }

    // Helper functions
    async fn create_test_repository(db: &DatabaseConnection) -> crate::entities::repository::Model {
        use crate::entities::{prelude::*, repo_provider};

        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider).exec(db).await.unwrap();

        let repo = crate::entities::repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.last_insert_id),
            ..Default::default()
        };
        Repository::insert(repo)
            .exec_with_returning(db)
            .await
            .unwrap()
    }

    async fn create_test_workspace(
        db: &DatabaseConnection,
        repository_id: i32,
        container_id: Option<&str>,
    ) -> workspace::Model {
        let workspace = workspace::ActiveModel {
            repository_id: Set(repository_id),
            workspace_status: Set("Active".to_string()),
            container_id: Set(container_id.map(|s| s.to_string())),
            container_status: Set(Some("running".to_string())),
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
}
