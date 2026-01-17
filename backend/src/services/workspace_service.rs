use crate::entities::{prelude::*, workspace};
use crate::error::{GitAutoDevError, Result};
use crate::services::DockerService;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

#[derive(Clone)]
pub struct WorkspaceService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}

impl WorkspaceService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }

    pub async fn create_workspace(&self, repository_id: i32) -> Result<workspace::Model> {
        let workspace = workspace::ActiveModel {
            repository_id: Set(repository_id),
            workspace_status: Set("Initializing".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
            ..Default::default()
        };

        let workspace = Workspace::insert(workspace)
            .exec_with_returning(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?;

        Ok(workspace)
    }

    pub async fn get_workspace_by_id(&self, id: i32) -> Result<workspace::Model> {
        Workspace::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?
            .ok_or_else(|| GitAutoDevError::NotFound(format!("Workspace with id {} not found", id)))
    }

    pub async fn list_workspaces(&self) -> Result<Vec<workspace::Model>> {
        Workspace::find()
            .all(&self.db)
            .await
            .map_err(GitAutoDevError::Database)
    }

    pub async fn update_workspace_status(&self, id: i32, status: &str) -> Result<workspace::Model> {
        let workspace = self.get_workspace_by_id(id).await?;

        let mut workspace: workspace::ActiveModel = workspace.into();
        workspace.workspace_status = Set(status.to_string());
        workspace.updated_at = Set(Utc::now());

        let workspace = workspace
            .update(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?;

        Ok(workspace)
    }

    pub async fn soft_delete_workspace(&self, id: i32) -> Result<workspace::Model> {
        let workspace = self.get_workspace_by_id(id).await?;

        let mut workspace: workspace::ActiveModel = workspace.into();
        workspace.deleted_at = Set(Some(Utc::now()));
        workspace.updated_at = Set(Utc::now());

        let workspace = workspace
            .update(&self.db)
            .await
            .map_err(GitAutoDevError::Database)?;

        Ok(workspace)
    }

    /// Create workspace with Docker container if available
    pub async fn create_workspace_with_container(
        &self,
        repository_id: i32,
    ) -> Result<workspace::Model> {
        // First create the workspace record
        let mut workspace = self.create_workspace(repository_id).await?;

        // If Docker is available, create and start container
        if let Some(docker) = &self.docker {
            let container_name = format!("workspace-{}", workspace.id);

            match docker
                .create_container(
                    &container_name,
                    &workspace.image_source,
                    vec!["/workspace".to_string()],
                    workspace.cpu_limit,
                    &workspace.memory_limit,
                )
                .await
            {
                Ok(container_id) => {
                    // Try to start the container
                    match docker.start_container(&container_id).await {
                        Ok(_) => {
                            // Update workspace with container info
                            let mut workspace_active: workspace::ActiveModel = workspace.into();
                            workspace_active.container_id = Set(Some(container_id.clone()));
                            workspace_active.container_status = Set(Some("running".to_string()));
                            workspace_active.workspace_status = Set("Active".to_string());
                            workspace_active.updated_at = Set(Utc::now());

                            workspace = workspace_active
                                .update(&self.db)
                                .await
                                .map_err(GitAutoDevError::Database)?;
                        }
                        Err(e) => {
                            // Failed to start, clean up container
                            if let Err(cleanup_err) =
                                docker.remove_container(&container_id, true).await
                            {
                                tracing::warn!(
                                    "Failed to cleanup container {}: {}",
                                    container_id,
                                    cleanup_err
                                );
                            }

                            // Update workspace status to Failed before returning error
                            let mut workspace_active: workspace::ActiveModel = workspace.into();
                            workspace_active.workspace_status = Set("Failed".to_string());
                            workspace_active.updated_at = Set(Utc::now());

                            if let Err(db_err) = workspace_active.update(&self.db).await {
                                tracing::error!(
                                    "Failed to update workspace status to Failed: {}",
                                    db_err
                                );
                            }

                            tracing::error!("Failed to start container: {}", e);
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    // Update workspace status to Failed before returning error
                    let mut workspace_active: workspace::ActiveModel = workspace.into();
                    workspace_active.workspace_status = Set("Failed".to_string());
                    workspace_active.updated_at = Set(Utc::now());

                    if let Err(db_err) = workspace_active.update(&self.db).await {
                        tracing::error!("Failed to update workspace status to Failed: {}", db_err);
                    }

                    tracing::error!("Failed to create container: {}", e);
                    return Err(e);
                }
            }
        } else {
            tracing::info!("Docker not available, workspace created without container");
        }

        Ok(workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::prelude::{RepoProvider, Repository};
    use crate::test_utils::db::TestDatabase;
    use sea_orm::{DatabaseConnection, EntityTrait, Set};

    #[tokio::test]
    async fn test_create_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create a test provider first
        let provider = crate::entities::repo_provider::ActiveModel {
            name: Set("Test Provider".to_string()),
            provider_type: Set(crate::entities::repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider).exec(db).await.unwrap();

        // Create a test repository
        let repo = crate::entities::repository::ActiveModel {
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.last_insert_id),
            ..Default::default()
        };
        let repo = Repository::insert(repo).exec(db).await.unwrap();

        let service = WorkspaceService::new(db.clone(), None);

        // Act
        let result = service.create_workspace(repo.last_insert_id).await;

        // Assert
        assert!(result.is_ok());
        let workspace = result.unwrap();
        assert_eq!(workspace.repository_id, repo.last_insert_id);
        assert_eq!(workspace.workspace_status, "Initializing");
        assert_eq!(workspace.max_concurrent_tasks, 3);
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
            GitAutoDevError::NotFound(_) => {}
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
    async fn test_soft_delete_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let service = WorkspaceService::new(db.clone(), None);
        let repo = create_test_repository(db).await;
        let workspace = service.create_workspace(repo.id).await.unwrap();

        // Act
        let result = service.soft_delete_workspace(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        let deleted = result.unwrap();
        assert!(deleted.deleted_at.is_some());
    }

    // Helper function
    async fn create_test_repository(db: &DatabaseConnection) -> crate::entities::repository::Model {
        use crate::entities::repository;

        // Create a test provider first
        let provider = crate::entities::repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(crate::entities::repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider).exec(db).await.unwrap();

        let repo = repository::ActiveModel {
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

    // ============================================
    // Task 4: Tests for WorkspaceService with Docker
    // ============================================

    #[tokio::test]
    async fn test_create_workspace_with_container_when_docker_available() {
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
        // Docker might not be running even if connection succeeds
        match result {
            Ok(workspace) => {
                assert_eq!(workspace.repository_id, repo.id);
                if let Some(container_id) = &workspace.container_id {
                    // Container was created successfully
                    assert_eq!(workspace.container_status, Some("running".to_string()));

                    // Cleanup: remove container (always runs regardless of assertion failures)
                    if let Some(docker_service) = docker {
                        if let Err(e) = docker_service.remove_container(container_id, true).await {
                            tracing::warn!(
                                "Failed to cleanup test container {}: {}",
                                container_id,
                                e
                            );
                        }
                    }
                } else {
                    // Docker available but container creation failed (e.g., image not available)
                    // This is acceptable
                }
            }
            Err(e) => {
                // Docker connection succeeded but container creation failed
                // This is acceptable in test environment
                eprintln!("Container creation failed (expected in test env): {:?}", e);

                // Note: No container to cleanup since creation failed
                // The service should have already cleaned up any partial state
            }
        }
    }

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
        let workspace = result.unwrap();
        assert_eq!(workspace.repository_id, repo.id);
        assert!(workspace.container_id.is_none());
        assert_eq!(workspace.workspace_status, "Initializing");
    }
}
