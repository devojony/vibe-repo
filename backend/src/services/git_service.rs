//! Git Service
//!
//! Handles Git operations for workspaces including cloning, worktree management, and branch operations.

use crate::entities::{repository, workspace};
use crate::error::{Result, VibeRepoError};
use sea_orm::DatabaseConnection;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct GitService {
    #[allow(dead_code)]
    db: DatabaseConnection,
    workspace_base_dir: String,
}

impl GitService {
    pub fn new(db: DatabaseConnection, workspace_base_dir: String) -> Self {
        // Convert to absolute path if it's relative
        let absolute_base_dir = if std::path::Path::new(&workspace_base_dir).is_absolute() {
            workspace_base_dir
        } else {
            // Get current directory and join with relative path
            std::env::current_dir()
                .ok()
                .and_then(|cwd| {
                    cwd.join(&workspace_base_dir)
                        .to_str()
                        .map(|s| s.to_string())
                })
                .unwrap_or(workspace_base_dir)
        };

        Self {
            db,
            workspace_base_dir: absolute_base_dir,
        }
    }

    /// Get workspace directory path
    pub fn get_workspace_dir(&self, workspace_id: i32) -> PathBuf {
        Path::new(&self.workspace_base_dir).join(format!("workspace-{}", workspace_id))
    }

    /// Get source directory path (main repository)
    pub fn get_source_dir(&self, workspace_id: i32) -> PathBuf {
        self.get_workspace_dir(workspace_id).join("source")
    }

    /// Get tasks directory path (worktrees)
    pub fn get_tasks_dir(&self, workspace_id: i32) -> PathBuf {
        self.get_workspace_dir(workspace_id).join("tasks")
    }

    /// Get task worktree directory path
    pub fn get_task_worktree_dir(&self, workspace_id: i32, task_id: i32) -> PathBuf {
        self.get_tasks_dir(workspace_id)
            .join(format!("task-{}", task_id))
    }

    /// Clone repository to workspace source directory (executed in container)
    pub async fn clone_repository(
        &self,
        workspace: &workspace::Model,
        repository: &repository::Model,
    ) -> Result<()> {
        info!(
            workspace_id = workspace.id,
            repository_id = repository.id,
            "Cloning repository to workspace (in container)"
        );

        // Create workspace directory structure on host
        let workspace_dir = self.get_workspace_dir(workspace.id);
        let tasks_dir = self.get_tasks_dir(workspace.id);

        // Create directories on host
        fs::create_dir_all(&workspace_dir).await.map_err(|e| {
            VibeRepoError::Internal(format!("Failed to create workspace dir: {}", e))
        })?;

        fs::create_dir_all(&tasks_dir)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create tasks dir: {}", e)))?;

        // Check if container_id exists
        let container_id = workspace.container_id.as_ref().ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Container ID not found for workspace {}",
                workspace.id
            ))
        })?;

        // Check if source directory already exists in container
        let check_cmd = vec![
            "bash".to_string(),
            "-c".to_string(),
            "test -d /workspace/source && echo 'exists' || echo 'not_exists'".to_string(),
        ];

        let docker_service = crate::services::DockerService::new()?;
        let check_output = docker_service
            .exec_in_container(container_id, check_cmd, 10)
            .await?;

        if check_output.stdout.trim() == "exists" {
            warn!(
                workspace_id = workspace.id,
                container_id = %container_id,
                "Source directory already exists in container, skipping clone"
            );
            return Ok(());
        }

        // Build clone URL with authentication using repository fields
        let clone_url = self.build_authenticated_clone_url(&repository.clone_url, repository)?;

        info!(
            workspace_id = workspace.id,
            container_id = %container_id,
            clone_url_masked = self.mask_url(&clone_url),
            "Cloning repository in container"
        );

        // Clone repository in container
        let clone_cmd = vec![
            "git".to_string(),
            "clone".to_string(),
            "--depth".to_string(),
            "1".to_string(),
            clone_url.clone(),
            "/workspace/source".to_string(),
        ];

        let clone_output = docker_service
            .exec_in_container(container_id, clone_cmd, 300) // 5 minutes timeout
            .await?;

        if clone_output.exit_code != 0 {
            error!(
                workspace_id = workspace.id,
                container_id = %container_id,
                stderr = %clone_output.stderr,
                "Git clone failed in container"
            );
            return Err(VibeRepoError::Internal(format!(
                "Git clone failed: {}",
                clone_output.stderr
            )));
        }

        // Configure git user in the repository
        let config_user_cmd = vec![
            "bash".to_string(),
            "-c".to_string(),
            "cd /workspace/source && git config user.name 'VibeRepo Bot' && git config user.email 'bot@vibe-repo.local'".to_string(),
        ];

        let config_output = docker_service
            .exec_in_container(container_id, config_user_cmd, 10)
            .await?;

        if config_output.exit_code != 0 {
            warn!(
                workspace_id = workspace.id,
                stderr = %config_output.stderr,
                "Failed to configure git user"
            );
        }

        // Unshallow the repository to allow full git operations
        info!(
            workspace_id = workspace.id,
            "Unshallowing repository in container"
        );
        let unshallow_cmd = vec![
            "bash".to_string(),
            "-c".to_string(),
            "cd /workspace/source && git fetch --unshallow".to_string(),
        ];

        let unshallow_output = docker_service
            .exec_in_container(container_id, unshallow_cmd, 300)
            .await?;

        if unshallow_output.exit_code != 0 {
            warn!(
                workspace_id = workspace.id,
                stderr = %unshallow_output.stderr,
                "Git unshallow failed (may already be complete)"
            );
        }

        info!(
            workspace_id = workspace.id,
            container_id = %container_id,
            "Repository cloned successfully in container"
        );

        Ok(())
    }

    /// Create a git worktree for a task (executed in container)
    pub async fn create_task_worktree(
        &self,
        workspace_id: i32,
        task_id: i32,
        branch_name: &str,
        container_id: &str,
    ) -> Result<PathBuf> {
        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            branch_name = branch_name,
            container_id = %container_id,
            "Creating git worktree for task in container"
        );

        let docker_service = crate::services::DockerService::new()?;
        let worktree_path = format!("/workspace/tasks/task-{}", task_id);

        // Check if source directory exists in container
        let check_cmd = vec![
            "bash".to_string(),
            "-c".to_string(),
            "test -d /workspace/source && echo 'exists' || echo 'not_exists'".to_string(),
        ];

        let check_output = docker_service
            .exec_in_container(container_id, check_cmd, 10)
            .await?;

        if check_output.stdout.trim() != "exists" {
            return Err(VibeRepoError::NotFound(format!(
                "Source directory not found in container for workspace {}",
                workspace_id
            )));
        }

        // Remove worktree if it already exists
        let remove_cmd = vec![
            "bash".to_string(),
            "-c".to_string(),
            format!(
                "cd /workspace/source && git worktree remove {} --force 2>/dev/null || true && rm -rf {}",
                worktree_path, worktree_path
            ),
        ];

        let remove_output = docker_service
            .exec_in_container(container_id, remove_cmd, 30)
            .await?;

        if remove_output.exit_code != 0 {
            warn!(
                workspace_id = workspace_id,
                task_id = task_id,
                stderr = %remove_output.stderr,
                "Failed to remove existing worktree (may not exist)"
            );
        }

        // Create worktree with new branch
        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            branch_name = branch_name,
            worktree_path = %worktree_path,
            "Creating git worktree in container"
        );

        let create_cmd = vec![
            "bash".to_string(),
            "-c".to_string(),
            format!(
                "cd /workspace/source && git worktree add -b {} {}",
                branch_name, worktree_path
            ),
        ];

        let create_output = docker_service
            .exec_in_container(container_id, create_cmd, 30)
            .await?;

        if create_output.exit_code != 0 {
            error!(
                workspace_id = workspace_id,
                task_id = task_id,
                stderr = %create_output.stderr,
                "Git worktree add failed in container"
            );
            return Err(VibeRepoError::Internal(format!(
                "Git worktree add failed: {}",
                create_output.stderr
            )));
        }

        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            worktree_path = %worktree_path,
            "Git worktree created successfully in container"
        );

        // Return the host path for compatibility
        Ok(self.get_task_worktree_dir(workspace_id, task_id))
    }

    /// Remove a git worktree for a task
    pub async fn remove_task_worktree(&self, workspace_id: i32, task_id: i32) -> Result<()> {
        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            "Removing git worktree for task"
        );

        let source_dir = self.get_source_dir(workspace_id);
        let worktree_dir = self.get_task_worktree_dir(workspace_id, task_id);

        if !worktree_dir.exists() {
            info!(
                workspace_id = workspace_id,
                task_id = task_id,
                "Worktree directory does not exist, nothing to remove"
            );
            return Ok(());
        }

        // Remove worktree from git
        let source_dir_str = source_dir.to_str().ok_or_else(|| {
            VibeRepoError::Internal("Invalid source directory path encoding".to_string())
        })?;

        let worktree_dir_str = worktree_dir.to_str().ok_or_else(|| {
            VibeRepoError::Internal("Invalid worktree directory path encoding".to_string())
        })?;

        let output = Command::new("git")
            .args([
                "-C",
                source_dir_str,
                "worktree",
                "remove",
                worktree_dir_str,
                "--force",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                VibeRepoError::Internal(format!("Failed to execute git worktree remove: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                workspace_id = workspace_id,
                task_id = task_id,
                stderr = %stderr,
                "Git worktree remove failed, will try to remove directory manually"
            );
        }

        // Remove directory if it still exists
        if worktree_dir.exists() {
            fs::remove_dir_all(&worktree_dir).await.map_err(|e| {
                VibeRepoError::Internal(format!("Failed to remove worktree directory: {}", e))
            })?;
        }

        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            "Git worktree removed successfully"
        );

        Ok(())
    }

    /// Build authenticated clone URL using repository access token
    fn build_authenticated_clone_url(
        &self,
        clone_url: &str,
        repository: &crate::entities::repository::Model,
    ) -> Result<String> {
        // Parse URL
        let url = url::Url::parse(clone_url)
            .map_err(|e| VibeRepoError::Validation(format!("Invalid clone URL: {}", e)))?;

        // Build authenticated URL with token
        let mut auth_url = url.clone();
        auth_url
            .set_username(&repository.access_token)
            .map_err(|_| VibeRepoError::Internal("Failed to set username in URL".to_string()))?;
        auth_url
            .set_password(Some(""))
            .map_err(|_| VibeRepoError::Internal("Failed to set password in URL".to_string()))?;

        Ok(auth_url.to_string())
    }

    /// Mask sensitive information in URL for logging
    fn mask_url(&self, url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            let mut masked = parsed.clone();
            if masked.username() != "" {
                let _ = masked.set_username("***");
            }
            if masked.password().is_some() {
                let _ = masked.set_password(Some("***"));
            }
            masked.to_string()
        } else {
            "***".to_string()
        }
    }
}
