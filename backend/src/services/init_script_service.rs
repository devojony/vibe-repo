use crate::entities::{prelude::*, init_script};
use crate::error::{VibeRepoError, Result};
use crate::services::DockerService;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, QueryFilter, ColumnTrait};

#[derive(Clone)]
pub struct InitScriptService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}

impl InitScriptService {
    const MAX_SUMMARY_SIZE: usize = 4096; // 4KB

    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }

    pub async fn create_init_script(
        &self,
        workspace_id: i32,
        script_content: String,
        timeout_seconds: i32,
    ) -> Result<init_script::Model> {
        // Verify workspace exists
        let _workspace = Workspace::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Workspace {} not found", workspace_id))
            })?;

        // Create script
        let script = init_script::ActiveModel {
            workspace_id: Set(workspace_id),
            script_content: Set(script_content),
            timeout_seconds: Set(timeout_seconds),
            status: Set("Pending".to_string()),
            ..Default::default()
        };

        let script = InitScript::insert(script)
            .exec_with_returning(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        tracing::info!(
            workspace_id = workspace_id,
            script_id = script.id,
            "Created init script for workspace"
        );

        Ok(script)
    }

    pub async fn get_init_script_by_workspace_id(
        &self,
        workspace_id: i32,
    ) -> Result<Option<init_script::Model>> {
        let script = InitScript::find()
            .filter(init_script::Column::WorkspaceId.eq(workspace_id))
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(script)
    }

    pub async fn update_init_script(
        &self,
        workspace_id: i32,
        script_content: String,
        timeout_seconds: i32,
    ) -> Result<init_script::Model> {
        let script = self
            .get_init_script_by_workspace_id(workspace_id)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!(
                    "Init script for workspace {} not found",
                    workspace_id
                ))
            })?;

        let mut script: init_script::ActiveModel = script.into();
        script.script_content = Set(script_content);
        script.timeout_seconds = Set(timeout_seconds);
        script.status = Set("Pending".to_string()); // Reset status
        script.updated_at = Set(Utc::now());

        let script = script.update(&self.db).await.map_err(VibeRepoError::Database)?;

        tracing::info!(
            workspace_id = workspace_id,
            script_id = script.id,
            "Updated init script"
        );

        Ok(script)
    }

    pub async fn execute_script(
        &self,
        workspace_id: i32,
        container_id: &str,
    ) -> Result<init_script::Model> {
        let docker = self.docker.as_ref().ok_or_else(|| {
            VibeRepoError::ServiceUnavailable("Docker service is not available".to_string())
        })?;

        let script = self
            .get_init_script_by_workspace_id(workspace_id)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!(
                    "Init script for workspace {} not found",
                    workspace_id
                ))
            })?;

        // Update status to Running
        let mut script_active: init_script::ActiveModel = script.clone().into();
        script_active.status = Set("Running".to_string());
        script_active.updated_at = Set(Utc::now());
        let script = script_active.update(&self.db).await.map_err(VibeRepoError::Database)?;

        tracing::info!(
            workspace_id = workspace_id,
            script_id = script.id,
            container_id = container_id,
            "Starting init script execution"
        );

        // Execute script in container
        let cmd = vec!["/bin/bash".to_string(), "-c".to_string(), script.script_content.clone()];
        let timeout = script.timeout_seconds as u64;

        let result = docker.exec_in_container(container_id, cmd, timeout).await;

        // Process result and update script
        match result {
            Ok(output) => {
                let (summary, file_path) = Self::save_script_output(
                    script.id,
                    workspace_id,
                    output.stdout,
                    output.stderr,
                )
                .await?;

                let status = if output.exit_code == 0 {
                    "Success"
                } else {
                    "Failed"
                };

                let mut script_active: init_script::ActiveModel = script.into();
                script_active.status = Set(status.to_string());
                script_active.output_summary = Set(summary);
                script_active.output_file_path = Set(file_path);
                script_active.executed_at = Set(Some(Utc::now()));
                script_active.updated_at = Set(Utc::now());

                let script = script_active.update(&self.db).await.map_err(VibeRepoError::Database)?;

                tracing::info!(
                    workspace_id = workspace_id,
                    script_id = script.id,
                    exit_code = output.exit_code,
                    status = status,
                    "Init script execution completed"
                );

                Ok(script)
            }
            Err(e) => {
                // Update status to Failed
                let error_msg = format!("Execution error: {}", e);
                let mut script_active: init_script::ActiveModel = script.into();
                script_active.status = Set("Failed".to_string());
                script_active.output_summary = Set(Some(error_msg.clone()));
                script_active.executed_at = Set(Some(Utc::now()));
                script_active.updated_at = Set(Utc::now());

                let script = script_active.update(&self.db).await.map_err(VibeRepoError::Database)?;

                tracing::error!(
                    workspace_id = workspace_id,
                    script_id = script.id,
                    error = %e,
                    "Init script execution failed"
                );

                Err(VibeRepoError::Internal(error_msg))
            }
        }
    }

    async fn save_script_output(
        script_id: i32,
        workspace_id: i32,
        stdout: String,
        stderr: String,
    ) -> Result<(Option<String>, Option<String>)> {
        let full_output = format!("=== STDOUT ===\n{}\n\n=== STDERR ===\n{}", stdout, stderr);

        if full_output.len() <= Self::MAX_SUMMARY_SIZE {
            // Small output: store in database only
            Ok((Some(full_output), None))
        } else {
            // Large output: store summary in DB, full in file
            let summary = Self::extract_last_4kb(&full_output);
            let file_path = Self::write_to_file(script_id, workspace_id, &full_output).await?;
            Ok((Some(summary), Some(file_path)))
        }
    }

    fn extract_last_4kb(output: &str) -> String {
        if output.len() <= Self::MAX_SUMMARY_SIZE {
            output.to_string()
        } else {
            let start = output.len() - Self::MAX_SUMMARY_SIZE;
            format!(
                "... [Output truncated, showing last 4KB]\n\n{}",
                &output[start..]
            )
        }
    }

    async fn write_to_file(
        script_id: i32,
        workspace_id: i32,
        content: &str,
    ) -> Result<String> {
        use tokio::fs;

        let base_dir = "/data/gitautodev/init-script-logs";
        let workspace_dir = format!("{}/workspace-{}", base_dir, workspace_id);

        // Create directory
        fs::create_dir_all(&workspace_dir)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create log directory: {}", e)))?;

        // Generate filename
        let timestamp = Utc::now().timestamp();
        let filename = format!("script-{}-{}.log", script_id, timestamp);
        let file_path = format!("{}/{}", workspace_dir, filename);

        // Write file
        fs::write(&file_path, content)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to write log file: {}", e)))?;

        Ok(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestDatabase;
    use crate::entities::prelude::{RepoProvider, Repository};
    use crate::entities::workspace;

    #[tokio::test]
    async fn test_create_init_script_success() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace
        let workspace = create_test_workspace(db).await;

        let service = InitScriptService::new(db.clone(), None);

        // Act
        let result = service
            .create_init_script(
                workspace.id,
                "#!/bin/bash\necho 'test'".to_string(),
                300,
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.workspace_id, workspace.id);
        assert_eq!(script.status, "Pending");
        assert_eq!(script.timeout_seconds, 300);
    }

    #[tokio::test]
    async fn test_get_init_script_by_workspace_id() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let workspace = create_test_workspace(db).await;
        let service = InitScriptService::new(db.clone(), None);

        // Create script
        let created = service
            .create_init_script(workspace.id, "test".to_string(), 300)
            .await
            .unwrap();

        // Act
        let result = service.get_init_script_by_workspace_id(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.is_some());
        assert_eq!(script.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_update_init_script() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let workspace = create_test_workspace(db).await;
        let service = InitScriptService::new(db.clone(), None);

        // Create script
        let created = service
            .create_init_script(workspace.id, "original".to_string(), 300)
            .await
            .unwrap();

        // Act: Update the script
        let result = service
            .update_init_script(workspace.id, "updated".to_string(), 600)
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.script_content, "updated");
        assert_eq!(updated.timeout_seconds, 600);
        assert_eq!(updated.status, "Pending"); // Status should be reset
    }

    #[tokio::test]
    async fn test_execute_script_success() {
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = &test_db.connection;
        let workspace = create_test_workspace_with_container(db).await;

        let docker = DockerService::new().ok();
        if docker.is_none() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        // Skip test if workspace doesn't have a container
        if workspace.container_id.is_none() {
            eprintln!("Skipping test: Failed to create container");
            return;
        }

        let service = InitScriptService::new(db.clone(), docker);

        // Create script
        let _script = service
            .create_init_script(workspace.id, "echo 'test'".to_string(), 10)
            .await
            .unwrap();

        // Act
        let result = service
            .execute_script(workspace.id, workspace.container_id.as_ref().unwrap())
            .await;

        // Assert
        assert!(result.is_ok());

        // Verify status updated
        let updated = service
            .get_init_script_by_workspace_id(workspace.id)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.status == "Success" || updated.status == "Running");
    }

    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        // Create a test provider first
        let provider = crate::entities::repo_provider::ActiveModel {
            name: Set("Test Provider".to_string()),
            provider_type: Set(crate::entities::repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec(db)
            .await
            .expect("Failed to create test provider");

        // Create a test repository
        let repo = crate::entities::repository::ActiveModel {
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.last_insert_id),
            ..Default::default()
        };
        let repo = Repository::insert(repo)
            .exec(db)
            .await
            .expect("Failed to create test repository");

        // Create a test workspace
        let workspace = workspace::ActiveModel {
            repository_id: Set(repo.last_insert_id),
            workspace_status: Set("Active".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(5),
            cpu_limit: Set(1.0),
            memory_limit: Set("512m".to_string()),
            disk_limit: Set("10g".to_string()),
            ..Default::default()
        };

        Workspace::insert(workspace)
            .exec_with_returning(db)
            .await
            .expect("Failed to create test workspace")
    }

    async fn create_test_workspace_with_container(db: &DatabaseConnection) -> workspace::Model {
        let docker = match DockerService::new() {
            Ok(d) => d,
            Err(_) => {
                // Return workspace without container if Docker not available
                return create_test_workspace(db).await;
            }
        };

        // Create workspace first
        let mut workspace = create_test_workspace(db).await;

        // Try to create and start container
        let container_name = format!("test-init-script-{}", workspace.id);
        match docker.create_container(&container_name, "alpine:latest", vec![], 1.0, "1GB").await {
            Ok(container_id) => {
                if docker.start_container(&container_id).await.is_ok() {
                    // Update workspace with container_id
                    let mut workspace_active: workspace::ActiveModel = workspace.into();
                    workspace_active.container_id = Set(Some(container_id));
                    workspace = workspace_active.update(db).await.expect("Failed to update workspace");
                }
            }
            Err(_) => {
                // Container creation failed, continue without it
            }
        }

        workspace
    }
}

