//! Container configuration module
//!
//! Provides configuration for container and image management.

use std::path::PathBuf;

/// Configuration for container operations
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Directory for workspace data (default: /tmp/gitautodev)
    pub workspace_base_dir: PathBuf,

    /// Path to workspace Dockerfile (default: docker/workspace/Dockerfile)
    pub workspace_dockerfile: PathBuf,

    /// Build context path (default: .)
    pub build_context: PathBuf,

    /// Container stop timeout in seconds (default: 10)
    pub stop_timeout_seconds: i64,

    /// Maximum restart attempts (default: 3)
    pub max_restart_attempts: i32,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            workspace_base_dir: PathBuf::from("/tmp/gitautodev"),
            workspace_dockerfile: PathBuf::from("docker/workspace/Dockerfile"),
            build_context: PathBuf::from("."),
            stop_timeout_seconds: 10,
            max_restart_attempts: 3,
        }
    }
}

impl ContainerConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            workspace_base_dir: std::env::var("WORKSPACE_BASE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp/gitautodev")),
            workspace_dockerfile: std::env::var("WORKSPACE_DOCKERFILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("docker/workspace/Dockerfile")),
            build_context: std::env::var("WORKSPACE_BUILD_CONTEXT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".")),
            stop_timeout_seconds: std::env::var("CONTAINER_STOP_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            max_restart_attempts: std::env::var("CONTAINER_MAX_RESTART_ATTEMPTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
        }
    }
}
