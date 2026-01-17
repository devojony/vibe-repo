//! Health check service for monitoring workspace containers
//!
//! Periodically checks container health and updates workspace status.

use crate::entities::{prelude::*, workspace};
use crate::error::{GitAutoDevError, Result};
use crate::services::DockerService;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

/// Health check service for monitoring workspace containers
pub struct HealthCheckService {
    db: Arc<DatabaseConnection>,
    docker: Option<DockerService>,
    interval: Duration,
}

impl HealthCheckService {
    /// Create a new health check service
    pub fn new(
        db: DatabaseConnection,
        docker: Option<DockerService>,
        interval: Duration,
    ) -> Result<Self> {
        Ok(Self {
            db: Arc::new(db),
            docker,
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
            .map_err(GitAutoDevError::Database)?;

        for workspace in workspaces {
            // Skip workspaces without containers
            let container_id = match &workspace.container_id {
                Some(id) => id,
                None => continue,
            };

            // Check container health
            match docker.check_container_health(container_id).await {
                Ok(health) => {
                    // Update workspace with health status
                    let mut workspace_active: workspace::ActiveModel = workspace.into();
                    workspace_active.health_status = Set(Some(health.status.clone()));
                    workspace_active.container_status = Set(Some(health.status));
                    workspace_active.last_health_check = Set(Some(Utc::now()));
                    workspace_active.updated_at = Set(Utc::now());

                    if let Err(e) = workspace_active.update(self.db.as_ref()).await {
                        tracing::error!("Failed to update workspace health: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to check health for container {}: {}",
                        container_id,
                        e
                    );

                    // Update workspace with error status
                    let mut workspace_active: workspace::ActiveModel = workspace.into();
                    workspace_active.health_status = Set(Some("error".to_string()));
                    workspace_active.last_health_check = Set(Some(Utc::now()));
                    workspace_active.updated_at = Set(Utc::now());

                    if let Err(e) = workspace_active.update(self.db.as_ref()).await {
                        tracing::error!("Failed to update workspace health: {}", e);
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
        let interval = Duration::from_secs(300);

        // Act
        let service = HealthCheckService::new(db, docker, interval);

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
        let service = HealthCheckService::new(db.clone(), docker, Duration::from_secs(300))
            .expect("Failed to create service");

        // Act
        let result = service.check_all_workspaces().await;

        // Assert
        assert!(result.is_ok());

        // Verify workspace was updated
        let updated = Workspace::find_by_id(workspace.id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.last_health_check.is_some());
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
        let service = HealthCheckService::new(db, None, Duration::from_secs(300));

        // Assert: Should succeed
        assert!(service.is_ok());

        // Act: Check workspaces should not fail
        let result = service.unwrap().check_all_workspaces().await;
        assert!(result.is_ok());
    }

    // Helper functions
    async fn create_test_repository(
        db: &DatabaseConnection,
    ) -> crate::entities::repository::Model {
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
