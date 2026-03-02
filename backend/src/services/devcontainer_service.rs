//! DevContainer service for managing workspace containers using @devcontainers/cli
//!
//! This service wraps the @devcontainers/cli to create and manage workspace containers
//! with support for standard devcontainer.json configuration.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, trace};

use crate::error::VibeRepoError;

/// DevContainer service for managing workspace containers
#[derive(Debug, Clone)]
pub struct DevContainerService {
    /// Path to devcontainer CLI executable
    cli_path: String,
    /// Base directory for workspaces
    workspace_base_dir: PathBuf,
}

/// Information about a created workspace container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Docker container ID
    pub container_id: String,
    /// Remote user in the container
    pub remote_user: Option<String>,
    /// Remote workspace folder path
    pub remote_workspace_folder: Option<String>,
}

/// DevContainer CLI output structure
#[derive(Debug, Deserialize)]
struct DevContainerOutput {
    #[serde(rename = "containerId")]
    container_id: String,
    #[serde(rename = "remoteUser")]
    remote_user: Option<String>,
    #[serde(rename = "remoteWorkspaceFolder")]
    remote_workspace_folder: Option<String>,
}

/// Agent configuration for installation
#[derive(Debug, Clone)]
pub struct AgentInstallConfig {
    /// Agent type (e.g., "opencode", "claude-code")
    pub agent_type: String,
    /// Installation timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for AgentInstallConfig {
    fn default() -> Self {
        Self {
            agent_type: "opencode".to_string(),
            timeout_seconds: 300, // 5 minutes
        }
    }
}

impl DevContainerService {
    /// Create a new DevContainer service
    pub fn new(cli_path: String, workspace_base_dir: PathBuf) -> Self {
        Self {
            cli_path,
            workspace_base_dir,
        }
    }

    /// Check if devcontainer CLI is available
    pub async fn check_cli_available(&self) -> Result<String, VibeRepoError> {
        debug!("Checking devcontainer CLI availability: {}", self.cli_path);

        let output = Command::new(&self.cli_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| {
                error!("Failed to execute devcontainer CLI: {}", e);
                VibeRepoError::Internal(format!(
                    "DevContainer CLI not found at '{}'. Please install it with: npm install -g @devcontainers/cli\nError: {}",
                    self.cli_path, e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VibeRepoError::Internal(format!(
                "DevContainer CLI check failed: {}",
                stderr
            )));
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!("DevContainer CLI version: {}", version);
        Ok(version)
    }

    /// Check if devcontainer.json exists in repository
    pub fn check_devcontainer_exists(&self, repo_path: &Path) -> bool {
        let devcontainer_path = repo_path.join(".devcontainer").join("devcontainer.json");
        let exists = devcontainer_path.exists();

        if exists {
            info!(
                "Found devcontainer.json at: {}",
                devcontainer_path.display()
            );
        } else {
            debug!(
                "No devcontainer.json found at: {}, will use default configuration",
                devcontainer_path.display()
            );
        }

        exists
    }

    /// Create a workspace container using devcontainer CLI
    pub async fn create_workspace(
        &self,
        workspace_id: &str,
        repo_path: &Path,
    ) -> Result<WorkspaceInfo, VibeRepoError> {
        info!(
            "Creating workspace container: workspace_id={}, repo_path={}",
            workspace_id,
            repo_path.display()
        );

        let start = std::time::Instant::now();

        // Build devcontainer up command
        let mut cmd = Command::new("npx");
        cmd.args(&[
            "--yes",
            "@devcontainers/cli",
            "up",
            "--workspace-folder",
            repo_path.to_str().ok_or_else(|| {
                VibeRepoError::Internal("Invalid repository path".to_string())
            })?,
            "--id-label",
            &format!("vibe-repo.workspace-id={}", workspace_id),
            "--log-format",
            "json",
        ]);

        debug!("Executing command: {:?}", cmd);

        // Execute command
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                error!("Failed to execute devcontainer up: {}", e);
                VibeRepoError::Internal(format!("Failed to execute devcontainer up: {}", e))
            })?;

        let duration = start.elapsed();

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("DevContainer up failed: {}", stderr);
            return Err(VibeRepoError::Internal(format!(
                "DevContainer up failed: {}",
                stderr
            )));
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        trace!("DevContainer up output: {}", stdout);

        // Extract last line (contains result JSON)
        let last_line = stdout
            .lines()
            .last()
            .ok_or_else(|| VibeRepoError::Internal("No output from devcontainer up".to_string()))?;

        debug!("Parsing result JSON: {}", last_line);

        // Parse JSON
        let result: DevContainerOutput = serde_json::from_str(last_line).map_err(|e| {
            error!("Failed to parse devcontainer output: {}", e);
            VibeRepoError::Internal(format!(
                "Failed to parse devcontainer output: {}. Output: {}",
                e, last_line
            ))
        })?;

        let workspace_info = WorkspaceInfo {
            container_id: result.container_id.clone(),
            remote_user: result.remote_user,
            remote_workspace_folder: result.remote_workspace_folder,
        };

        info!(
            "Workspace container created successfully: container_id={}, duration={:.2}s",
            workspace_info.container_id,
            duration.as_secs_f64()
        );

        Ok(workspace_info)
    }

    /// Install agent (Bun + OpenCode) in container
    pub async fn install_agent(
        &self,
        container_id: &str,
        config: &AgentInstallConfig,
    ) -> Result<(), VibeRepoError> {
        info!(
            "Installing agent in container: container_id={}, agent_type={}",
            container_id, config.agent_type
        );

        let start = std::time::Instant::now();

        // Generate installation script
        let script = self.generate_install_script(&config.agent_type)?;
        debug!("Installation script:\n{}", script);

        // Execute installation script
        let output = Command::new("docker")
            .args(&["exec", container_id, "bash", "-c", &script])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                error!("Failed to execute agent installation: {}", e);
                VibeRepoError::Internal(format!("Failed to execute agent installation: {}", e))
            })?;

        let duration = start.elapsed();

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Agent installation failed: {}", stderr);
            return Err(VibeRepoError::Internal(format!(
                "Agent installation failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        trace!("Agent installation output: {}", stdout);

        info!(
            "Agent installed successfully: container_id={}, duration={:.2}s",
            container_id,
            duration.as_secs_f64()
        );

        Ok(())
    }

    /// Remove workspace container
    pub async fn remove_workspace(&self, container_id: &str) -> Result<(), VibeRepoError> {
        info!("Removing workspace container: container_id={}", container_id);

        let output = Command::new("docker")
            .args(&["rm", "-f", container_id])
            .output()
            .await
            .map_err(|e| {
                error!("Failed to remove container: {}", e);
                VibeRepoError::Internal(format!("Failed to remove container: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail if container doesn't exist
            if stderr.contains("No such container") {
                debug!("Container already removed: {}", container_id);
                return Ok(());
            }
            error!("Failed to remove container: {}", stderr);
            return Err(VibeRepoError::Internal(format!(
                "Failed to remove container: {}",
                stderr
            )));
        }

        info!("Workspace container removed: container_id={}", container_id);
        Ok(())
    }

    /// Validate devcontainer.json file
    pub async fn validate_devcontainer_json(
        &self,
        repo_path: &Path,
    ) -> Result<(), VibeRepoError> {
        let devcontainer_path = repo_path.join(".devcontainer").join("devcontainer.json");

        if !devcontainer_path.exists() {
            return Ok(()); // No file to validate
        }

        debug!("Validating devcontainer.json at: {}", devcontainer_path.display());

        // Read file content
        let content = tokio::fs::read_to_string(&devcontainer_path)
            .await
            .map_err(|e| {
                error!("Failed to read devcontainer.json: {}", e);
                VibeRepoError::Validation(format!(
                    "Failed to read devcontainer.json at {}: {}",
                    devcontainer_path.display(),
                    e
                ))
            })?;

        // Validate JSON syntax
        let config: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            error!("Invalid JSON in devcontainer.json: {}", e);
            VibeRepoError::Validation(format!(
                "Invalid JSON syntax in devcontainer.json at {}: {}",
                devcontainer_path.display(),
                e
            ))
        })?;

        // Validate required fields (either "image" or "build" must be present)
        let has_image = config.get("image").is_some();
        let has_build = config.get("build").is_some() || config.get("dockerFile").is_some();

        if !has_image && !has_build {
            return Err(VibeRepoError::Validation(format!(
                "devcontainer.json at {} must contain either 'image' or 'build' property. \
                 Example: {{\"image\": \"ubuntu:22.04\"}}",
                devcontainer_path.display()
            )));
        }

        // Warn about unsupported properties
        let unsupported_properties = vec![
            "dockerComposeFile",
            "runServices",
            "service",
            "shutdownAction",
        ];

        let mut found_unsupported = Vec::new();
        for prop in unsupported_properties {
            if config.get(prop).is_some() {
                found_unsupported.push(prop);
            }
        }

        if !found_unsupported.is_empty() {
            tracing::warn!(
                "devcontainer.json contains unsupported properties: {}. These will be ignored.",
                found_unsupported.join(", ")
            );
        }

        info!("devcontainer.json validation passed");
        Ok(())
    }

    /// Create default devcontainer.json configuration
    fn create_default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "VibeRepo Workspace",
            "image": "ubuntu:22.04",
            "overrideCommand": true,
            "remoteUser": "root",
            "workspaceFolder": "/workspace"
        })
    }

    /// Create temporary devcontainer.json file with default configuration
    async fn create_temp_devcontainer_config(
        &self,
        repo_path: &Path,
    ) -> Result<PathBuf, VibeRepoError> {
        let devcontainer_dir = repo_path.join(".devcontainer");
        let devcontainer_file = devcontainer_dir.join("devcontainer.json");

        // Create .devcontainer directory if it doesn't exist
        tokio::fs::create_dir_all(&devcontainer_dir)
            .await
            .map_err(|e| {
                error!("Failed to create .devcontainer directory: {}", e);
                VibeRepoError::Internal(format!(
                    "Failed to create .devcontainer directory: {}",
                    e
                ))
            })?;

        // Write default configuration
        let config = self.create_default_config();
        let config_str = serde_json::to_string_pretty(&config).map_err(|e| {
            error!("Failed to serialize default config: {}", e);
            VibeRepoError::Internal(format!("Failed to serialize default config: {}", e))
        })?;

        tokio::fs::write(&devcontainer_file, config_str)
            .await
            .map_err(|e| {
                error!("Failed to write devcontainer.json: {}", e);
                VibeRepoError::Internal(format!("Failed to write devcontainer.json: {}", e))
            })?;

        debug!(
            "Created temporary devcontainer.json at: {}",
            devcontainer_file.display()
        );

        Ok(devcontainer_file)
    }

    /// Clean up temporary devcontainer.json file
    async fn cleanup_temp_devcontainer_config(&self, config_path: &Path) -> Result<(), VibeRepoError> {
        if config_path.exists() {
            tokio::fs::remove_file(config_path).await.map_err(|e| {
                error!("Failed to remove temporary devcontainer.json: {}", e);
                VibeRepoError::Internal(format!(
                    "Failed to remove temporary devcontainer.json: {}",
                    e
                ))
            })?;

            debug!(
                "Cleaned up temporary devcontainer.json: {}",
                config_path.display()
            );
        }

        Ok(())
    }

    /// Generate agent installation script
    fn generate_install_script(&self, agent_type: &str) -> Result<String, VibeRepoError> {
        match agent_type {
            "opencode" => Ok(self.generate_opencode_install_script()),
            "claude-code" => Err(VibeRepoError::Internal(
                "Claude Code installation not yet implemented".to_string(),
            )),
            _ => Err(VibeRepoError::Internal(format!(
                "Unsupported agent type: {}",
                agent_type
            ))),
        }
    }

    /// Generate OpenCode installation script
    fn generate_opencode_install_script(&self) -> String {
        r#"
set -e

echo "Installing Bun..."
if ! command -v bun &> /dev/null; then
    curl -fsSL https://bun.sh/install | bash
    export BUN_INSTALL="$HOME/.bun"
    export PATH="$BUN_INSTALL/bin:$PATH"
fi

echo "Verifying Bun installation..."
bun --version

echo "Installing OpenCode..."
bun install -g opencode-ai

echo "Verifying OpenCode installation..."
opencode --version

echo "Verifying OpenCode ACP support..."
opencode acp --help

echo "Agent installation complete!"
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_devcontainer_service_new() {
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        assert_eq!(service.cli_path, "devcontainer");
        assert_eq!(service.workspace_base_dir, PathBuf::from("/tmp/workspaces"));
    }

    #[test]
    fn test_check_devcontainer_exists_returns_false_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let exists = service.check_devcontainer_exists(temp_dir.path());
        assert!(!exists, "Should return false when devcontainer.json is missing");
    }

    #[test]
    fn test_check_devcontainer_exists_returns_true_when_present() {
        let temp_dir = TempDir::new().unwrap();
        let devcontainer_dir = temp_dir.path().join(".devcontainer");
        std::fs::create_dir(&devcontainer_dir).unwrap();
        std::fs::write(
            devcontainer_dir.join("devcontainer.json"),
            r#"{"image": "ubuntu:22.04"}"#,
        )
        .unwrap();

        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let exists = service.check_devcontainer_exists(temp_dir.path());
        assert!(exists, "Should return true when devcontainer.json exists");
    }

    #[test]
    fn test_generate_opencode_install_script() {
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let script = service.generate_opencode_install_script();

        assert!(script.contains("Installing Bun"));
        assert!(script.contains("curl -fsSL https://bun.sh/install"));
        assert!(script.contains("bun install -g opencode-ai"));
        assert!(script.contains("opencode --version"));
        assert!(script.contains("opencode acp --help"));
    }

    #[test]
    fn test_generate_install_script_opencode() {
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let result = service.generate_install_script("opencode");
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.contains("Installing Bun"));
        assert!(script.contains("Installing OpenCode"));
    }

    #[test]
    fn test_generate_install_script_unsupported_agent() {
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let result = service.generate_install_script("unsupported-agent");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unsupported agent type"));
    }

    #[test]
    fn test_agent_install_config_default() {
        let config = AgentInstallConfig::default();

        assert_eq!(config.agent_type, "opencode");
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn test_create_default_config() {
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let config = service.create_default_config();

        assert_eq!(config["name"], "VibeRepo Workspace");
        assert_eq!(config["image"], "ubuntu:22.04");
        assert_eq!(config["overrideCommand"], true);
        assert_eq!(config["remoteUser"], "root");
        assert_eq!(config["workspaceFolder"], "/workspace");
    }

    #[tokio::test]
    async fn test_create_temp_devcontainer_config() {
        let temp_dir = TempDir::new().unwrap();
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let config_path = service
            .create_temp_devcontainer_config(temp_dir.path())
            .await
            .unwrap();

        assert!(config_path.exists());
        assert_eq!(
            config_path,
            temp_dir.path().join(".devcontainer/devcontainer.json")
        );

        // Verify content
        let content = std::fs::read_to_string(&config_path).unwrap();
        let config: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(config["image"], "ubuntu:22.04");
        assert_eq!(config["remoteUser"], "root");
    }

    #[tokio::test]
    async fn test_cleanup_temp_devcontainer_config() {
        let temp_dir = TempDir::new().unwrap();
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let config_path = service
            .create_temp_devcontainer_config(temp_dir.path())
            .await
            .unwrap();

        assert!(config_path.exists());

        service
            .cleanup_temp_devcontainer_config(&config_path)
            .await
            .unwrap();

        assert!(!config_path.exists());
    }

    #[tokio::test]
    async fn test_validate_devcontainer_json_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        // Should succeed when file doesn't exist
        let result = service.validate_devcontainer_json(temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_devcontainer_json_valid_with_image() {
        let temp_dir = TempDir::new().unwrap();
        let devcontainer_dir = temp_dir.path().join(".devcontainer");
        std::fs::create_dir(&devcontainer_dir).unwrap();
        std::fs::write(
            devcontainer_dir.join("devcontainer.json"),
            r#"{"image": "ubuntu:22.04"}"#,
        )
        .unwrap();

        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let result = service.validate_devcontainer_json(temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_devcontainer_json_valid_with_build() {
        let temp_dir = TempDir::new().unwrap();
        let devcontainer_dir = temp_dir.path().join(".devcontainer");
        std::fs::create_dir(&devcontainer_dir).unwrap();
        std::fs::write(
            devcontainer_dir.join("devcontainer.json"),
            r#"{"build": {"dockerfile": "Dockerfile"}}"#,
        )
        .unwrap();

        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let result = service.validate_devcontainer_json(temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_devcontainer_json_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let devcontainer_dir = temp_dir.path().join(".devcontainer");
        std::fs::create_dir(&devcontainer_dir).unwrap();
        std::fs::write(
            devcontainer_dir.join("devcontainer.json"),
            r#"{"image": invalid json}"#,
        )
        .unwrap();

        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let result = service.validate_devcontainer_json(temp_dir.path()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid JSON"));
    }

    #[tokio::test]
    async fn test_validate_devcontainer_json_missing_required_fields() {
        let temp_dir = TempDir::new().unwrap();
        let devcontainer_dir = temp_dir.path().join(".devcontainer");
        std::fs::create_dir(&devcontainer_dir).unwrap();
        std::fs::write(
            devcontainer_dir.join("devcontainer.json"),
            r#"{"name": "test"}"#,
        )
        .unwrap();

        let service = DevContainerService::new(
            "devcontainer".to_string(),
            PathBuf::from("/tmp/workspaces"),
        );

        let result = service.validate_devcontainer_json(temp_dir.path()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("must contain either 'image' or 'build'"));
    }
}
