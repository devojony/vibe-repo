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

    /// Execute command in container using docker exec
    async fn exec_in_container(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
    ) -> Result<std::process::Output> {
        Command::new("docker")
            .arg("exec")
            .arg(container_id)
            .args(&cmd)
            .output()
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to execute docker command: {}", e)))
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
        let check_output = self
            .exec_in_container(
                container_id,
                vec!["bash", "-c", "test -d /workspace/source && echo 'exists' || echo 'not_exists'"],
            )
            .await?;

        let stdout = String::from_utf8_lossy(&check_output.stdout);

        if stdout.trim() == "exists" {
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
        let clone_output = self
            .exec_in_container(
                container_id,
                vec!["git", "clone", "--depth", "1", &clone_url, "/workspace/source"],
            )
            .await?;

        if !clone_output.status.success() {
            let stderr = String::from_utf8_lossy(&clone_output.stderr);
            error!(
                workspace_id = workspace.id,
                container_id = %container_id,
                stderr = %stderr,
                "Git clone failed in container"
            );
            return Err(VibeRepoError::Internal(format!(
                "Git clone failed: {}",
                stderr
            )));
        }

        // Configure git user in the repository
        let config_output = self
            .exec_in_container(
                container_id,
                vec!["bash", "-c", "cd /workspace/source && git config user.name 'VibeRepo Bot' && git config user.email 'bot@vibe-repo.local'"],
            )
            .await?;

        if !config_output.status.success() {
            let stderr = String::from_utf8_lossy(&config_output.stderr);
            warn!(
                workspace_id = workspace.id,
                stderr = %stderr,
                "Failed to configure git user"
            );
        }

        // Unshallow the repository to allow full git operations
        info!(
            workspace_id = workspace.id,
            "Unshallowing repository in container"
        );
        let unshallow_output = self
            .exec_in_container(
                container_id,
                vec!["bash", "-c", "cd /workspace/source && git fetch --unshallow"],
            )
            .await?;

        if !unshallow_output.status.success() {
            let stderr = String::from_utf8_lossy(&unshallow_output.stderr);
            warn!(
                workspace_id = workspace.id,
                stderr = %stderr,
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

        let worktree_path = format!("/workspace/tasks/task-{}", task_id);

        // Check if source directory exists in container
        let check_output = self
            .exec_in_container(
                container_id,
                vec!["bash", "-c", "test -d /workspace/source && echo 'exists' || echo 'not_exists'"],
            )
            .await?;

        let stdout = String::from_utf8_lossy(&check_output.stdout);
        if stdout.trim() != "exists" {
            return Err(VibeRepoError::NotFound(format!(
                "Source directory not found in container for workspace {}",
                workspace_id
            )));
        }

        // Remove worktree if it already exists
        let remove_cmd = format!(
            "cd /workspace/source && git worktree remove {} --force 2>/dev/null || true && rm -rf {}",
            worktree_path, worktree_path
        );
        let remove_output = self
            .exec_in_container(container_id, vec!["bash", "-c", &remove_cmd])
            .await?;

        if !remove_output.status.success() {
            let stderr = String::from_utf8_lossy(&remove_output.stderr);
            warn!(
                workspace_id = workspace_id,
                task_id = task_id,
                stderr = %stderr,
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

        let create_cmd = format!(
            "cd /workspace/source && git worktree add -b {} {}",
            branch_name, worktree_path
        );
        let create_output = self
            .exec_in_container(container_id, vec!["bash", "-c", &create_cmd])
            .await?;

        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr);
            error!(
                workspace_id = workspace_id,
                task_id = task_id,
                stderr = %stderr,
                "Git worktree add failed in container"
            );
            return Err(VibeRepoError::Internal(format!(
                "Git worktree add failed: {}",
                stderr
            )));
        }

        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            worktree_path = %worktree_path,
            "Git worktree created successfully in container"
        );

        // Return the container path (not host path) since agent runs in container
        Ok(PathBuf::from(worktree_path))
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

    /// Push branch to remote repository (executed in container)
    pub async fn push_branch(
        &self,
        workspace_id: i32,
        task_id: i32,
        branch_name: &str,
        container_id: &str,
    ) -> Result<()> {
        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            branch_name = branch_name,
            container_id = %container_id,
            "Pushing branch to remote repository"
        );

        let worktree_path = format!("/workspace/tasks/task-{}", task_id);

        // Push branch to remote with --set-upstream
        let push_cmd = format!(
            "cd {} && git push -u origin {}",
            worktree_path, branch_name
        );

        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            branch_name = branch_name,
            "Executing git push in container"
        );

        let push_output = self
            .exec_in_container(container_id, vec!["bash", "-c", &push_cmd])
            .await?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            let stdout = String::from_utf8_lossy(&push_output.stdout);
            error!(
                workspace_id = workspace_id,
                task_id = task_id,
                branch_name = branch_name,
                stderr = %stderr,
                stdout = %stdout,
                "Git push failed in container"
            );
            return Err(VibeRepoError::Internal(format!(
                "Git push failed: {}",
                stderr
            )));
        }

        info!(
            workspace_id = workspace_id,
            task_id = task_id,
            branch_name = branch_name,
            "Branch pushed successfully to remote"
        );

        Ok(())
    }
}
