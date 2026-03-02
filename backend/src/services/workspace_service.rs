use crate::entities::{prelude::*, workspace};
use crate::error::{Result, VibeRepoError};
use crate::services::{AgentInstallConfig, AgentService, DevContainerService};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;
use std::path::PathBuf;

/// Workspace status constants
pub mod workspace_status {
    pub const INITIALIZING: &str = "Initializing";
    pub const ACTIVE: &str = "Active";
    pub const FAILED: &str = "Failed";
}

#[derive(Clone)]
pub struct WorkspaceService {
    db: DatabaseConnection,
    devcontainer: DevContainerService,
}

impl WorkspaceService {
    pub fn new(db: DatabaseConnection, devcontainer: DevContainerService) -> Self {
        Self { db, devcontainer }
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

    /// Create workspace using DevContainer CLI
    ///
    /// Creates a workspace record, creates a container using devcontainer CLI,
    /// and installs the agent. Returns the workspace model and container ID.
    pub async fn create_workspace_with_container(
        &self,
        repository_id: i32,
        repo_path: PathBuf,
    ) -> Result<(workspace::Model, String)> {
        // Get repository to access agent configuration
        let repo = Repository::find_by_id(repository_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Repository {} not found", repository_id))
            })?;

        // Create workspace record
        let mut workspace = self.create_workspace(repository_id).await?;

        // Check if devcontainer.json exists
        let has_devcontainer = self.devcontainer.check_devcontainer_exists(&repo_path);
        if has_devcontainer {
            tracing::info!(
                workspace_id = workspace.id,
                "Found devcontainer.json, validating..."
            );

            // Validate devcontainer.json
            if let Err(e) = self.devcontainer.validate_devcontainer_json(&repo_path).await {
                tracing::error!(
                    workspace_id = workspace.id,
                    error = %e,
                    "devcontainer.json validation failed"
                );
                self.mark_workspace_failed(workspace.clone(), &e.to_string())
                    .await;
                return Err(e);
            }
        } else {
            tracing::info!(
                workspace_id = workspace.id,
                "No devcontainer.json found, will use default configuration"
            );
        }

        // Create workspace container
        let workspace_info = match self
            .devcontainer
            .create_workspace(&workspace.id.to_string(), &repo_path)
            .await
        {
            Ok(info) => info,
            Err(e) => {
                tracing::error!(
                    workspace_id = workspace.id,
                    error = %e,
                    "Failed to create workspace container"
                );
                self.mark_workspace_failed(workspace.clone(), &e.to_string())
                    .await;
                return Err(e);
            }
        };

        tracing::info!(
            workspace_id = workspace.id,
            container_id = %workspace_info.container_id,
            "Workspace container created successfully"
        );

        // Install agent in container
        let agent_config = AgentInstallConfig {
            agent_type: repo
                .agent_command
                .clone()
                .unwrap_or_else(|| "opencode".to_string()),
            timeout_seconds: repo.agent_timeout.max(60) as u64,
        };

        if let Err(e) = self
            .devcontainer
            .install_agent(&workspace_info.container_id, &agent_config)
            .await
        {
            tracing::error!(
                workspace_id = workspace.id,
                container_id = %workspace_info.container_id,
                error = %e,
                "Failed to install agent"
            );

            // Clean up container
            let _ = self
                .devcontainer
                .remove_workspace(&workspace_info.container_id)
                .await;

            self.mark_workspace_failed(workspace.clone(), &e.to_string())
                .await;
            return Err(e);
        }

        // Create agent record in database
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
                    "Failed to create agent record"
                );
                // Don't fail workspace creation if agent record creation fails
            }
        }

        // Update workspace with container_id and status
        let mut workspace_active: workspace::ActiveModel = workspace.into();
        workspace_active.container_id = Set(Some(workspace_info.container_id.clone()));
        workspace_active.workspace_status = Set(workspace_status::ACTIVE.to_string());
        
        workspace = workspace_active
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok((workspace, workspace_info.container_id))
    }

    /// Delete workspace and its container
    pub async fn delete_workspace_with_container(
        &self,
        workspace_id: i32,
        container_id: &str,
    ) -> Result<()> {
        // Remove container
        self.devcontainer.remove_workspace(container_id).await?;

        // Delete workspace record
        self.delete_workspace(workspace_id).await?;

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

    #[tokio::test]
    async fn test_create_workspace_success() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

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

        let devcontainer = DevContainerService::new(
            "devcontainer".to_string(),
            std::path::PathBuf::from("/tmp/workspaces"),
        );
        let service = WorkspaceService::new(db.clone(), devcontainer);

        let result = service.create_workspace(repo.id).await;

        assert!(result.is_ok());
        let workspace = result.unwrap();
        assert_eq!(workspace.repository_id, repo.id);
        assert_eq!(workspace.workspace_status, "Initializing");
    }

    #[tokio::test]
    async fn test_get_workspace_by_id_success() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

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

        let devcontainer = DevContainerService::new(
            "devcontainer".to_string(),
            std::path::PathBuf::from("/tmp/workspaces"),
        );
        let service = WorkspaceService::new(db.clone(), devcontainer);

        let workspace = service.create_workspace(repo.id).await.unwrap();
        let result = service.get_workspace_by_id(workspace.id).await;

        assert!(result.is_ok());
        let fetched = result.unwrap();
        assert_eq!(fetched.id, workspace.id);
    }

    #[tokio::test]
    async fn test_delete_workspace_success() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

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

        let devcontainer = DevContainerService::new(
            "devcontainer".to_string(),
            std::path::PathBuf::from("/tmp/workspaces"),
        );
        let service = WorkspaceService::new(db.clone(), devcontainer);

        let workspace = service.create_workspace(repo.id).await.unwrap();
        let result = service.delete_workspace(workspace.id).await;

        assert!(result.is_ok());

        let get_result = service.get_workspace_by_id(workspace.id).await;
        assert!(get_result.is_err());
    }
}
