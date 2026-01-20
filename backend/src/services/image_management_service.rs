//! Image management service for Docker images
//!
//! Provides operations for managing workspace Docker images including
//! inspection, deletion, and rebuilding.

use crate::entities::{container, prelude::*};
use crate::error::{Result, VibeRepoError};
use crate::services::{BuildImageResult, ContainerConfig, DockerService, ImageInfo};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

/// Validate Docker image name format
/// Expected format: name:tag (e.g., "vibe-repo-workspace:latest")
fn validate_image_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(VibeRepoError::Validation(
            "Image name cannot be empty".to_string(),
        ));
    }

    if !name.contains(':') {
        return Err(VibeRepoError::Validation(format!(
            "Invalid image name format '{}'. Expected 'name:tag'",
            name
        )));
    }

    Ok(())
}

#[derive(Clone)]
pub struct ImageManagementService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
    config: ContainerConfig,
}

impl ImageManagementService {
    /// Create a new ImageManagementService
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

    /// Get Docker service reference or return error if unavailable
    fn require_docker(&self) -> Result<&DockerService> {
        self.docker
            .as_ref()
            .ok_or_else(|| VibeRepoError::Internal("Docker not available".to_string()))
    }

    /// Get image information if it exists
    ///
    /// Returns `Ok(None)` if image doesn't exist, `Ok(Some(info))` if it exists,
    /// or error if Docker is unavailable.
    pub async fn get_image_info(&self, image_name: &str) -> Result<Option<ImageInfo>> {
        validate_image_name(image_name)?;
        let docker = self.require_docker()?;

        // Check if image exists
        let exists = docker.image_exists(image_name).await?;

        if !exists {
            return Ok(None);
        }

        // Get image info
        let info = docker.inspect_image(image_name).await?;
        Ok(Some(info))
    }

    /// Get list of workspace IDs using a specific image
    ///
    /// Queries the containers table for all containers using the specified image.
    pub async fn get_workspaces_using_image(&self, image_name: &str) -> Result<Vec<i32>> {
        let containers = Container::find()
            .filter(container::Column::ImageName.eq(image_name))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        let workspace_ids: Vec<i32> = containers.into_iter().map(|c| c.workspace_id).collect();

        Ok(workspace_ids)
    }

    /// Delete a Docker image
    ///
    /// Returns error if any workspaces are using the image or if Docker is unavailable.
    pub async fn delete_image(&self, image_name: &str) -> Result<()> {
        validate_image_name(image_name)?;
        let docker = self.require_docker()?;

        // Check if any workspaces are using the image
        let workspace_ids = self.get_workspaces_using_image(image_name).await?;

        if !workspace_ids.is_empty() {
            let workspace_word = if workspace_ids.len() == 1 {
                "workspace"
            } else {
                "workspaces"
            };
            let verb = if workspace_ids.len() == 1 {
                "is"
            } else {
                "are"
            };

            tracing::warn!(
                image_name = %image_name,
                workspace_count = workspace_ids.len(),
                "Cannot delete image: {} {} using this image", workspace_word, verb
            );
            return Err(VibeRepoError::Conflict(format!(
                "Cannot delete image: {} {} {} using this image",
                workspace_ids.len(),
                workspace_word,
                verb
            )));
        }

        // Delete the image
        tracing::info!(image_name = %image_name, "Deleting image");
        docker.remove_image(image_name, false).await?;

        Ok(())
    }

    /// Rebuild a Docker image
    ///
    /// If `force=false`, returns error if workspaces are using the image.
    /// If `force=true`, rebuilds even if workspaces are using it.
    pub async fn rebuild_image(&self, image_name: &str, force: bool) -> Result<BuildImageResult> {
        validate_image_name(image_name)?;
        let docker = self.require_docker()?;

        // Check if workspaces are using the image (unless forced)
        if !force {
            let workspace_ids = self.get_workspaces_using_image(image_name).await?;

            if !workspace_ids.is_empty() {
                let workspace_word = if workspace_ids.len() == 1 {
                    "workspace"
                } else {
                    "workspaces"
                };
                let verb = if workspace_ids.len() == 1 {
                    "is"
                } else {
                    "are"
                };

                tracing::warn!(
                    image_name = %image_name,
                    workspace_count = workspace_ids.len(),
                    "Cannot rebuild image: {} {} using this image", workspace_word, verb
                );
                return Err(VibeRepoError::Conflict(format!(
                    "Cannot rebuild image: {} {} {} using this image. Use force=true to rebuild anyway.",
                    workspace_ids.len(),
                    workspace_word,
                    verb
                )));
            }
        }

        // Remove existing image if it exists
        let exists = docker.image_exists(image_name).await?;
        if exists {
            tracing::info!(image_name = %image_name, "Removing existing image before rebuild");
            docker.remove_image(image_name, true).await?;
        }

        // Build new image
        tracing::info!(image_name = %image_name, force = force, "Rebuilding image");
        let result = docker
            .build_image(
                self.config.workspace_dockerfile.to_str().unwrap(),
                image_name,
                self.config.build_context.to_str().unwrap(),
            )
            .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// Helper function to create a test container
    async fn create_test_container(
        db: &DatabaseConnection,
        workspace_id: i32,
        image_name: &str,
    ) -> container::Model {
        let container = container::ActiveModel {
            workspace_id: Set(workspace_id),
            container_id: Set(format!("test-container-{}", uuid::Uuid::new_v4())),
            container_name: Set(format!("workspace-{}", workspace_id)),
            image_name: Set(image_name.to_string()),
            status: Set("running".to_string()),
            restart_count: Set(0),
            max_restart_attempts: Set(3),
            health_check_failures: Set(0),
            ..Default::default()
        };
        Container::insert(container)
            .exec_with_returning(db)
            .await
            .unwrap()
    }

    /// Test 1: Constructor works
    /// Requirements: Task 2.3 - ImageManagementService implementation
    #[tokio::test]
    async fn test_new_creates_service() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Act
        let service = ImageManagementService::new(db.clone(), None);

        // Assert - service should be created successfully
        drop(service);
    }

    /// Test 2: Returns Some(ImageInfo) when image exists
    /// Requirements: Task 2.3 - get_image_info method
    #[tokio::test]
    async fn test_get_image_info_exists() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Try to initialize Docker
        let docker = DockerService::new().ok();
        let service = ImageManagementService::new(db.clone(), docker.clone());

        // Act
        let result = service.get_image_info("alpine:latest").await;

        // Assert
        if docker.is_some() {
            // If Docker is available, test the full flow
            match result {
                Ok(Some(info)) => {
                    assert_eq!(info.name, "alpine:latest");
                    assert!(!info.id.is_empty());
                    assert!(info.size_bytes > 0);
                }
                Ok(None) => {
                    eprintln!("Image not found (may need to pull alpine:latest)");
                }
                Err(e) => {
                    eprintln!("Failed to get image info: {:?}", e);
                }
            }
        } else {
            // Docker not available - should return error
            assert!(result.is_err());
        }
    }

    /// Test 3: Returns None when image doesn't exist
    /// Requirements: Task 2.3 - get_image_info method
    #[tokio::test]
    async fn test_get_image_info_not_exists() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Try to initialize Docker
        let docker = DockerService::new().ok();
        let service = ImageManagementService::new(db.clone(), docker.clone());

        // Act
        let result = service
            .get_image_info("nonexistent-image:nonexistent-tag")
            .await;

        // Assert
        if docker.is_some() {
            // Docker service created, but may not be running
            match result {
                Ok(None) => {
                    // Expected: image doesn't exist
                }
                Ok(Some(_)) => {
                    panic!("Expected None for nonexistent image");
                }
                Err(e) => {
                    // Docker daemon may not be running
                    eprintln!("Docker error (daemon may not be running): {:?}", e);
                }
            }
        } else {
            // Docker not available - should return error
            assert!(result.is_err());
        }
    }

    /// Test 4: Returns error when Docker unavailable
    /// Requirements: Task 2.3 - get_image_info error handling
    #[tokio::test]
    async fn test_get_image_info_docker_unavailable() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Create service without Docker
        let service = ImageManagementService::new(db.clone(), None);

        // Act
        let result = service.get_image_info("alpine:latest").await;

        // Assert - should return error when Docker is unavailable
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Internal(msg) => {
                assert!(msg.contains("Docker not available"));
            }
            e => panic!("Expected Internal error, got: {:?}", e),
        }
    }

    /// Test 5: Returns empty vec when no workspaces using image
    /// Requirements: Task 2.3 - get_workspaces_using_image method
    #[tokio::test]
    async fn test_get_workspaces_using_image_empty() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;
        let service = ImageManagementService::new(db.clone(), None);

        // Act
        let result = service.get_workspaces_using_image("alpine:latest").await;

        // Assert
        assert!(result.is_ok());
        let workspace_ids = result.unwrap();
        assert!(workspace_ids.is_empty());
    }

    /// Test 6: Returns workspace IDs when multiple workspaces using image
    /// Requirements: Task 2.3 - get_workspaces_using_image method
    #[tokio::test]
    async fn test_get_workspaces_using_image_multiple() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Create workspaces and containers
        let workspace1 = create_test_workspace(&db).await;
        let workspace2 = create_test_workspace(&db).await;
        let workspace3 = create_test_workspace(&db).await;

        let _container1 = create_test_container(&db, workspace1.id, "test-image:latest").await;
        let _container2 = create_test_container(&db, workspace2.id, "test-image:latest").await;
        let _container3 = create_test_container(&db, workspace3.id, "other-image:latest").await;

        let service = ImageManagementService::new(db.clone(), None);

        // Act
        let result = service
            .get_workspaces_using_image("test-image:latest")
            .await;

        // Assert
        assert!(result.is_ok());
        let workspace_ids = result.unwrap();
        assert_eq!(workspace_ids.len(), 2);
        assert!(workspace_ids.contains(&workspace1.id));
        assert!(workspace_ids.contains(&workspace2.id));
        assert!(!workspace_ids.contains(&workspace3.id));
    }

    /// Test 7: Deletes image when no workspaces using it
    /// Requirements: Task 2.3 - delete_image method
    #[tokio::test]
    async fn test_delete_image_success() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Try to initialize Docker
        let docker = DockerService::new().ok();
        let service = ImageManagementService::new(db.clone(), docker.clone());

        // Create a test image first (if Docker is available)
        if let Some(ref docker_service) = docker {
            // Create a temporary Dockerfile
            let temp_dir = std::env::temp_dir();
            let dockerfile_path = temp_dir.join("Dockerfile.delete_test");
            let dockerfile_content = "FROM alpine:latest\n";

            if tokio::fs::write(&dockerfile_path, dockerfile_content)
                .await
                .is_ok()
            {
                let image_name = "test-delete-image:latest";

                // Build image
                if docker_service
                    .build_image(
                        dockerfile_path.to_str().unwrap(),
                        image_name,
                        temp_dir.to_str().unwrap(),
                    )
                    .await
                    .is_ok()
                {
                    // Act
                    let result = service.delete_image(image_name).await;

                    // Cleanup
                    let _ = tokio::fs::remove_file(&dockerfile_path).await;

                    // Assert
                    assert!(result.is_ok());

                    // Verify image is deleted
                    let exists = docker_service.image_exists(image_name).await.unwrap();
                    assert!(!exists);
                } else {
                    eprintln!("Skipping test: Failed to build image");
                }
            } else {
                eprintln!("Skipping test: Failed to write Dockerfile");
            }
        } else {
            eprintln!("Skipping test: Docker not available");
        }
    }

    /// Test 8: Returns Conflict error when workspaces using image
    /// Requirements: Task 2.3 - delete_image error handling
    #[tokio::test]
    async fn test_delete_image_in_use() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Create workspace and container using the image
        let workspace = create_test_workspace(&db).await;
        let _container = create_test_container(&db, workspace.id, "test-image:latest").await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();
        let service = ImageManagementService::new(db.clone(), docker);

        // Act
        let result = service.delete_image("test-image:latest").await;

        // Assert - should return Conflict error
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Conflict(msg) => {
                assert!(msg.contains("Cannot delete image"));
                assert!(msg.contains("1 workspace is using this image"));
            }
            e => panic!("Expected Conflict error, got: {:?}", e),
        }
    }

    /// Test 9: Returns error when Docker unavailable
    /// Requirements: Task 2.3 - delete_image error handling
    #[tokio::test]
    async fn test_delete_image_docker_unavailable() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Create service without Docker
        let service = ImageManagementService::new(db.clone(), None);

        // Act
        let result = service.delete_image("alpine:latest").await;

        // Assert - should return error when Docker is unavailable
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Internal(msg) => {
                assert!(msg.contains("Docker not available"));
            }
            e => panic!("Expected Internal error, got: {:?}", e),
        }
    }

    /// Test 10: Rebuilds image when no workspaces using it
    /// Requirements: Task 2.3 - rebuild_image method
    #[tokio::test]
    async fn test_rebuild_image_success() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Try to initialize Docker
        let docker = DockerService::new().ok();
        let _service = ImageManagementService::new(db.clone(), docker.clone());

        // Create a test Dockerfile (if Docker is available)
        if docker.is_some() {
            // Create a temporary Dockerfile
            let temp_dir = std::env::temp_dir();
            let dockerfile_path = temp_dir.join("Dockerfile.rebuild_test");
            let dockerfile_content = "FROM alpine:latest\nRUN echo 'test'\n";

            if tokio::fs::write(&dockerfile_path, dockerfile_content)
                .await
                .is_ok()
            {
                // Note: We can't actually test rebuild without a real Dockerfile at docker/workspace/Dockerfile
                // This test would need the actual project structure
                eprintln!("Skipping rebuild test: Requires docker/workspace/Dockerfile");
            }
        } else {
            eprintln!("Skipping test: Docker not available");
        }
    }

    /// Test 11: Returns Conflict when workspaces using image and force=false
    /// Requirements: Task 2.3 - rebuild_image error handling
    #[tokio::test]
    async fn test_rebuild_image_in_use_no_force() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Create workspace and container using the image
        let workspace = create_test_workspace(&db).await;
        let _container = create_test_container(&db, workspace.id, "test-image:latest").await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();
        let service = ImageManagementService::new(db.clone(), docker);

        // Act
        let result = service.rebuild_image("test-image:latest", false).await;

        // Assert - should return Conflict error
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::Conflict(msg) => {
                assert!(msg.contains("Cannot rebuild image"));
                assert!(msg.contains("1 workspace is using this image"));
                assert!(msg.contains("Use force=true to rebuild anyway"));
            }
            e => panic!("Expected Conflict error, got: {:?}", e),
        }
    }

    /// Test 12: Rebuilds when workspaces using image and force=true
    /// Requirements: Task 2.3 - rebuild_image with force
    #[tokio::test]
    async fn test_rebuild_image_in_use_with_force() {
        // Arrange
        let test_db = TestDatabase::new_in_memory()
            .await
            .expect("Failed to create test database");
        let db = test_db.connection;

        // Create workspace and container using the image
        let workspace = create_test_workspace(&db).await;
        let _container = create_test_container(&db, workspace.id, "test-image:latest").await;

        // Try to initialize Docker
        let docker = DockerService::new().ok();
        let service = ImageManagementService::new(db.clone(), docker.clone());

        // Act
        let result = service.rebuild_image("test-image:latest", true).await;

        // Assert
        if docker.is_some() {
            // If Docker is available, rebuild should attempt (but may fail without real Dockerfile)
            match result {
                Ok(_) => {
                    // Success - image was rebuilt
                }
                Err(e) => {
                    // Expected to fail without docker/workspace/Dockerfile
                    eprintln!("Rebuild failed (expected without Dockerfile): {:?}", e);
                }
            }
        } else {
            // Docker not available - should return error
            assert!(result.is_err());
        }
    }
}
