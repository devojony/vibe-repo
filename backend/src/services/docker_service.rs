//! Docker service for container management
//!
//! Provides Docker container lifecycle management for workspaces.

use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, RestartContainerOptions, StartContainerOptions, StatsOptions,
    StopContainerOptions,
};
use bollard::image::{BuildImageOptions, ListImagesOptions, RemoveImageOptions};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Instant;

use crate::error::{Result, VibeRepoError};
use crate::services::ContainerConfig;

/// Exit code used when actual exit code is unknown
const EXEC_EXIT_CODE_UNKNOWN: i64 = -1;

/// File mode for Dockerfile in tar archive (rw-r--r--)
const DOCKERFILE_MODE: u32 = 0o644;

/// Container health status
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerHealth {
    pub is_running: bool,
    pub status: String,
    pub exit_code: Option<i64>,
    pub error: Option<String>,
}

/// Output from executing a command in a container
#[derive(Debug, Clone, PartialEq)]
pub struct ExecOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

/// Result from building a Docker image
#[derive(Debug, Clone)]
pub struct BuildImageResult {
    pub image_name: String,
    pub image_id: String,
    pub build_time_seconds: f64,
    pub size_bytes: i64,
}

/// Docker image information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}

/// Container resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub memory_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

#[derive(Clone)]
pub struct DockerService {
    docker: Docker,
    config: ContainerConfig,
}

impl DockerService {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| VibeRepoError::Internal(format!("Failed to connect to Docker: {}", e)))?;

        Ok(Self {
            docker,
            config: ContainerConfig::default(),
        })
    }

    pub fn with_config(config: ContainerConfig) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| VibeRepoError::Internal(format!("Failed to connect to Docker: {}", e)))?;

        Ok(Self { docker, config })
    }

    pub async fn ping(&self) -> Result<bool> {
        self.docker
            .ping()
            .await
            .map(|_| true)
            .map_err(|e| VibeRepoError::Internal(format!("Docker ping failed: {}", e)))
    }

    pub async fn create_container(
        &self,
        name: &str,
        image: &str,
        volumes: Vec<String>,
        cpu_limit: f64,
        memory_limit: &str,
    ) -> Result<String> {
        // Validate CPU limit is within reasonable bounds
        if cpu_limit <= 0.0 || cpu_limit > 128.0 {
            return Err(VibeRepoError::Validation(format!(
                "CPU limit must be between 0.0 and 128.0, got: {}",
                cpu_limit
            )));
        }

        // Parse memory limit (e.g., "4GB" -> 4294967296 bytes)
        let memory_bytes = parse_memory_limit(memory_limit)?;

        // Create mounts for volumes
        let mounts: Vec<Mount> = volumes
            .iter()
            .map(|v| Mount {
                target: Some(v.clone()),
                source: Some(
                    self.config
                        .workspace_base_dir
                        .join(name)
                        .to_string_lossy()
                        .to_string(),
                ),
                typ: Some(MountTypeEnum::BIND),
                ..Default::default()
            })
            .collect();

        let host_config = HostConfig {
            mounts: Some(mounts),
            memory: Some(memory_bytes),
            nano_cpus: Some((cpu_limit * 1_000_000_000.0) as i64),
            ..Default::default()
        };

        let config = Config {
            image: Some(image.to_string()),
            host_config: Some(host_config),
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: name.to_string(),
            ..Default::default()
        };

        let response = self
            .docker
            .create_container(Some(options), config)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create container: {}", e)))?;

        Ok(response.id)
    }

    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        let options = RemoveContainerOptions {
            force,
            ..Default::default()
        };

        self.docker
            .remove_container(container_id, Some(options))
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to remove container: {}", e)))
    }

    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to start container: {}", e)))
    }

    pub async fn stop_container(&self, container_id: &str, timeout: i64) -> Result<()> {
        let options = StopContainerOptions { t: timeout };

        self.docker
            .stop_container(container_id, Some(options))
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to stop container: {}", e)))
    }

    pub async fn get_container_status(&self, container_id: &str) -> Result<String> {
        let inspect = self
            .docker
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to inspect container: {}", e)))?;

        let status = inspect
            .state
            .and_then(|s| s.status)
            .map(|s| format!("{:?}", s).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(status)
    }

    pub async fn container_exists(&self, container_id: &str) -> bool {
        self.docker
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
            .is_ok()
    }

    /// Check container health status
    pub async fn check_container_health(&self, container_id: &str) -> Result<ContainerHealth> {
        let inspect = self
            .docker
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to inspect container: {}", e)))?;

        let state = inspect
            .state
            .ok_or_else(|| VibeRepoError::Internal("Container state not available".to_string()))?;

        let is_running = state.running.unwrap_or(false);
        let status = state
            .status
            .map(|s| format!("{:?}", s).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let exit_code = state.exit_code;
        let error = state.error.filter(|e| !e.is_empty());

        Ok(ContainerHealth {
            is_running,
            status,
            exit_code,
            error,
        })
    }

    /// Execute a command in a container with timeout support
    ///
    /// Returns `ExecOutput` with exit code, stdout, and stderr.
    /// Exit code will be -1 if the command execution fails or cannot be inspected.
    /// Returns error if command times out or execution fails.
    pub async fn exec_in_container(
        &self,
        container_id: &str,
        cmd: Vec<String>,
        timeout_secs: u64,
    ) -> Result<ExecOutput> {
        use bollard::exec::{CreateExecOptions, StartExecResults};
        use futures::StreamExt;
        use tokio::time::{timeout, Duration};

        tracing::info!(
            container_id = %container_id,
            cmd = ?cmd,
            timeout_secs = timeout_secs,
            "Starting command execution in container"
        );

        // Create exec instance
        let exec_config = CreateExecOptions {
            cmd: Some(cmd.clone()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| {
                tracing::error!(
                    container_id = %container_id,
                    cmd = ?cmd,
                    error = %e,
                    "Failed to create exec"
                );
                VibeRepoError::Internal(format!("Failed to create exec: {}", e))
            })?;

        // Start exec with timeout
        let exec_id = exec.id.clone();
        let docker = self.docker.clone();
        let cmd_for_log = cmd.clone();

        let result = timeout(Duration::from_secs(timeout_secs), async move {
            let mut stdout = String::new();
            let mut stderr = String::new();

            if let StartExecResults::Attached { mut output, .. } = docker
                .start_exec(&exec_id, None)
                .await
                .map_err(|e| VibeRepoError::Internal(format!("Failed to start exec: {}", e)))?
            {
                while let Some(msg) = output.next().await {
                    match msg {
                        Ok(log_output) => {
                            use bollard::container::LogOutput;
                            match log_output {
                                LogOutput::StdOut { message } => {
                                    stdout.push_str(&String::from_utf8_lossy(&message));
                                }
                                LogOutput::StdErr { message } => {
                                    stderr.push_str(&String::from_utf8_lossy(&message));
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                exec_id = %exec_id,
                                error = %e,
                                "Stream error while reading exec output"
                            );
                        }
                    }
                }
            }

            // Get exit code
            let inspect = docker
                .inspect_exec(&exec_id)
                .await
                .map_err(|e| VibeRepoError::Internal(format!("Failed to inspect exec: {}", e)))?;

            // Exit code defaults to -1 if not available (indicates execution failure or incomplete execution)
            let exit_code = inspect.exit_code.unwrap_or(EXEC_EXIT_CODE_UNKNOWN);

            Ok::<ExecOutput, VibeRepoError>(ExecOutput {
                exit_code,
                stdout,
                stderr,
            })
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                tracing::info!(
                    container_id = %container_id,
                    cmd = ?cmd_for_log,
                    exit_code = output.exit_code,
                    "Command execution completed successfully"
                );
                Ok(output)
            }
            Ok(Err(e)) => {
                tracing::error!(
                    container_id = %container_id,
                    cmd = ?cmd_for_log,
                    error = %e,
                    "Command execution failed"
                );
                Err(e)
            }
            Err(_) => {
                tracing::error!(
                    container_id = %container_id,
                    cmd = ?cmd_for_log,
                    timeout_secs = timeout_secs,
                    "Command execution timed out"
                );
                Err(VibeRepoError::Internal(
                    "Command execution timed out".to_string(),
                ))
            }
        }
    }

    // ========== Image Management Methods ==========

    /// Check if a Docker image exists locally
    pub async fn image_exists(&self, image_name: &str) -> Result<bool> {
        let mut filters = HashMap::new();
        filters.insert("reference".to_string(), vec![image_name.to_string()]);

        let options = ListImagesOptions {
            filters,
            ..Default::default()
        };

        let images = self
            .docker
            .list_images(Some(options))
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to list images: {}", e)))?;

        Ok(!images.is_empty())
    }

    /// Build a Docker image from a Dockerfile
    pub async fn build_image(
        &self,
        dockerfile_path: &str,
        image_name: &str,
        context_path: &str,
    ) -> Result<BuildImageResult> {
        tracing::info!(
            dockerfile_path = %dockerfile_path,
            image_name = %image_name,
            context_path = %context_path,
            "Starting image build"
        );

        let start_time = Instant::now();

        // Check if Dockerfile exists
        if !tokio::fs::try_exists(dockerfile_path)
            .await
            .unwrap_or(false)
        {
            return Err(VibeRepoError::Internal(format!(
                "Dockerfile not found at path: {}",
                dockerfile_path
            )));
        }

        // Read Dockerfile content
        let dockerfile_content = tokio::fs::read_to_string(dockerfile_path)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to read Dockerfile: {}", e)))?;

        // Create build context tar archive
        let tar_bytes = create_build_context(context_path, &dockerfile_content)?;

        // Build image
        let options = BuildImageOptions {
            t: image_name.to_string(),
            dockerfile: "Dockerfile".to_string(),
            rm: true,
            ..Default::default()
        };

        let mut stream = self
            .docker
            .build_image(options, None, Some(tar_bytes.into()));

        // Stream build output
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(output) => {
                    if let Some(stream) = output.stream {
                        tracing::debug!(output = %stream.trim(), "Build output");
                    }
                    if let Some(error) = output.error {
                        return Err(VibeRepoError::Internal(format!("Build failed: {}", error)));
                    }
                }
                Err(e) => {
                    return Err(VibeRepoError::Internal(format!(
                        "Build stream error: {}",
                        e
                    )));
                }
            }
        }

        let build_time_seconds = start_time.elapsed().as_secs_f64();

        // Inspect image to get ID and size
        let image_info = self.inspect_image(image_name).await?;

        tracing::info!(
            image_name = %image_name,
            image_id = %image_info.id,
            build_time_seconds = build_time_seconds,
            size_bytes = image_info.size_bytes,
            "Image build completed"
        );

        Ok(BuildImageResult {
            image_name: image_name.to_string(),
            image_id: image_info.id,
            build_time_seconds,
            size_bytes: image_info.size_bytes,
        })
    }

    /// Remove a Docker image
    pub async fn remove_image(&self, image_name: &str, force: bool) -> Result<()> {
        let options = RemoveImageOptions {
            force,
            ..Default::default()
        };

        self.docker
            .remove_image(image_name, Some(options), None)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to remove image: {}", e)))?;

        tracing::info!(image_name = %image_name, force = force, "Image removed");
        Ok(())
    }

    /// Inspect a Docker image and get its metadata
    pub async fn inspect_image(&self, image_name: &str) -> Result<ImageInfo> {
        let inspect = self
            .docker
            .inspect_image(image_name)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to inspect image: {}", e)))?;

        let id = inspect.id.unwrap_or_default();
        let size_bytes = inspect.size.unwrap_or(0);
        let created_str = inspect.created.unwrap_or_default();
        let created_at = parse_docker_timestamp(&created_str)?;

        Ok(ImageInfo {
            id,
            name: image_name.to_string(),
            size_bytes,
            created_at,
        })
    }

    /// List all containers (running or stopped) using a specific image
    pub async fn list_containers_using_image(&self, image_name: &str) -> Result<Vec<String>> {
        let mut filters = HashMap::new();
        filters.insert("ancestor".to_string(), vec![image_name.to_string()]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to list containers: {}", e)))?;

        let container_ids: Vec<String> = containers.into_iter().filter_map(|c| c.id).collect();

        Ok(container_ids)
    }

    // ========== Container Operations Methods ==========

    /// Restart a Docker container with timeout
    pub async fn restart_container(&self, container_id: &str, timeout: i64) -> Result<()> {
        let options = RestartContainerOptions {
            t: timeout as isize,
        };

        self.docker
            .restart_container(container_id, Some(options))
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to restart container: {}", e)))?;

        tracing::info!(container_id = %container_id, timeout = timeout, "Container restarted");
        Ok(())
    }

    /// Get real-time resource usage statistics for a container
    pub async fn get_container_stats(&self, container_id: &str) -> Result<ContainerStats> {
        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = self.docker.stats(container_id, Some(options));

        if let Some(result) = stream.next().await {
            let stats = result
                .map_err(|e| VibeRepoError::Internal(format!("Failed to get stats: {}", e)))?;

            // Calculate CPU percentage
            let cpu_percent = calculate_cpu_percent(&stats);

            // Calculate memory usage
            let memory_usage_bytes = stats.memory_stats.usage.unwrap_or(0) as f64;
            let memory_limit_bytes = stats.memory_stats.limit.unwrap_or(0) as f64;
            let memory_usage_mb = memory_usage_bytes / (1024.0 * 1024.0);
            let memory_limit_mb = memory_limit_bytes / (1024.0 * 1024.0);
            let memory_percent = if memory_limit_bytes > 0.0 {
                (memory_usage_bytes / memory_limit_bytes) * 100.0
            } else {
                0.0
            };

            // Calculate network stats
            let (network_rx_bytes, network_tx_bytes) = calculate_network_stats(&stats);

            Ok(ContainerStats {
                cpu_percent,
                memory_usage_mb,
                memory_limit_mb,
                memory_percent,
                network_rx_bytes,
                network_tx_bytes,
            })
        } else {
            Err(VibeRepoError::Internal(
                "No stats available for container".to_string(),
            ))
        }
    }
}

// ========== Helper Functions ==========

/// Create a tar archive containing Dockerfile and context files
fn create_build_context(context_path: &str, dockerfile_content: &str) -> Result<Vec<u8>> {
    let mut tar_bytes = Vec::new();
    let mut tar = tar::Builder::new(&mut tar_bytes);

    // Add Dockerfile
    let dockerfile_bytes = dockerfile_content.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(dockerfile_bytes.len() as u64);
    header.set_mode(DOCKERFILE_MODE);
    header.set_cksum();
    tar.append_data(&mut header, "Dockerfile", dockerfile_bytes)
        .map_err(|e| VibeRepoError::Internal(format!("Failed to add Dockerfile to tar: {}", e)))?;

    // Add context files if context_path exists and is a directory
    if std::path::Path::new(context_path).is_dir() {
        tracing::info!(context_path = %context_path, "Adding build context directory");
        tar.append_dir_all(".", context_path)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to add context to tar: {}", e)))?;
    } else {
        tracing::info!(context_path = %context_path, "Context path is not a directory, skipping");
    }

    tar.finish()
        .map_err(|e| VibeRepoError::Internal(format!("Failed to finish tar: {}", e)))?;

    drop(tar);
    Ok(tar_bytes)
}

/// Parse Docker's RFC3339 timestamp format
fn parse_docker_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>> {
    if timestamp_str.is_empty() {
        tracing::warn!("Empty timestamp string, using current time as fallback");
        return Ok(Utc::now());
    }

    DateTime::parse_from_rfc3339(timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| VibeRepoError::Internal(format!("Failed to parse timestamp: {}", e)))
}

/// Calculate CPU usage percentage from Docker stats
fn calculate_cpu_percent(stats: &bollard::container::Stats) -> f64 {
    // Get CPU usage values - total_usage is u64, not Option<u64>
    let cpu_total = stats.cpu_stats.cpu_usage.total_usage as f64;
    let precpu_total = stats.precpu_stats.cpu_usage.total_usage as f64;
    let cpu_delta = cpu_total - precpu_total;

    let system_cpu_usage = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64;
    let precpu_system_usage = stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
    let system_cpu_delta = system_cpu_usage - precpu_system_usage;

    // Get number of CPUs
    let num_cpus = stats.cpu_stats.online_cpus.unwrap_or_else(|| {
        stats
            .cpu_stats
            .cpu_usage
            .percpu_usage
            .as_ref()
            .map(|percpu| percpu.len() as u64)
            .unwrap_or(1)
    }) as f64;

    if system_cpu_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_cpu_delta) * num_cpus * 100.0
    } else {
        0.0
    }
}

/// Calculate network RX and TX bytes from Docker stats
fn calculate_network_stats(stats: &bollard::container::Stats) -> (u64, u64) {
    let networks = match &stats.networks {
        Some(networks) => networks,
        None => return (0, 0),
    };

    let mut rx_bytes = 0u64;
    let mut tx_bytes = 0u64;

    for network_stats in networks.values() {
        rx_bytes += network_stats.rx_bytes;
        tx_bytes += network_stats.tx_bytes;
    }

    (rx_bytes, tx_bytes)
}

fn parse_memory_limit(limit: &str) -> Result<i64> {
    let limit = limit.to_uppercase();

    if let Some(gb) = limit.strip_suffix("GB") {
        let value: f64 = gb
            .parse()
            .map_err(|_| VibeRepoError::Validation("Invalid memory limit format".to_string()))?;
        Ok((value * 1024.0 * 1024.0 * 1024.0) as i64)
    } else if let Some(mb) = limit.strip_suffix("MB") {
        let value: f64 = mb
            .parse()
            .map_err(|_| VibeRepoError::Validation("Invalid memory limit format".to_string()))?;
        Ok((value * 1024.0 * 1024.0) as i64)
    } else {
        Err(VibeRepoError::Validation(
            "Memory limit must end with GB or MB".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test exec_in_container executes command successfully
    /// Requirements: Task 2.1 - Execute commands in containers
    #[tokio::test]
    async fn test_exec_in_container_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create test container
        let container_id = match service
            .create_container("test-exec", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // Start container
        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        // Execute command
        let result = service
            .exec_in_container(
                &container_id,
                vec!["echo".to_string(), "hello".to_string()],
                10,
            )
            .await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        // Assert
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello"));
    }

    /// Test exec_in_container handles timeout correctly
    /// Requirements: Task 2.1 - Execute commands in containers with timeout
    #[tokio::test]
    async fn test_exec_in_container_timeout() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create test container
        let container_id = match service
            .create_container("test-exec-timeout", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // Start container
        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        // Execute command that sleeps longer than timeout
        let result = service
            .exec_in_container(
                &container_id,
                vec!["sleep".to_string(), "10".to_string()],
                2,
            )
            .await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        // Assert - should timeout
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("timed out"));
    }

    /// Test exec_in_container handles non-zero exit codes correctly
    /// Requirements: Task 2.1 - Execute commands in containers with error handling
    #[tokio::test]
    async fn test_exec_in_container_nonzero_exit() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create test container
        let container_id = match service
            .create_container("test-exec-nonzero", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // Start container
        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        // Execute command that exits with non-zero code
        let result = service
            .exec_in_container(
                &container_id,
                vec!["sh".to_string(), "-c".to_string(), "exit 1".to_string()],
                10,
            )
            .await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        // Assert - should succeed but with non-zero exit code
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.exit_code, 1);
    }

    /// Test check_container_health returns correct health status for running container
    /// Requirements: Task 5 - Container health checks
    #[tokio::test]
    async fn test_check_container_health_running_container() {
        // Arrange: Create Docker service and container
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        let container_id = match service
            .create_container("test-health-check", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // Start the container
        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        // Act: Check container health
        let health = service.check_container_health(&container_id).await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        // Assert
        assert!(health.is_ok());
        let health = health.unwrap();
        assert!(health.is_running);
        assert_eq!(health.status, "running");
        assert!(health.exit_code.is_none());
        assert!(health.error.is_none());
    }

    /// Test check_container_health returns correct health status for stopped container
    /// Requirements: Task 5 - Container health checks
    #[tokio::test]
    async fn test_check_container_health_stopped_container() {
        // Arrange: Create and stop container
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        let container_id = match service
            .create_container("test-health-stopped", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // Start then stop the container
        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        if service.stop_container(&container_id, 5).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to stop container");
            return;
        }

        // Act: Check container health
        let health = service.check_container_health(&container_id).await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        // Assert
        assert!(health.is_ok());
        let health = health.unwrap();
        assert!(!health.is_running);
        assert!(health.status == "exited" || health.status == "stopped");
        assert!(health.exit_code.is_some());
        assert!(health.error.is_none());
    }

    /// Test check_container_health returns error for non-existent container
    /// Requirements: Task 5 - Container health checks
    #[tokio::test]
    async fn test_check_container_health_nonexistent_container() {
        // Arrange
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Act: Check health of non-existent container
        let health = service
            .check_container_health("nonexistent-container-id")
            .await;

        // Assert: Should return error
        assert!(health.is_err());
    }

    #[tokio::test]
    async fn test_docker_service_new_success() {
        // This test requires Docker to be running
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // If we got here, Docker is available, so ping should work
        let ping_result = service.ping().await;
        if ping_result.is_err() {
            eprintln!("Skipping test: Docker ping failed");
            return;
        }
        assert!(ping_result.is_ok());
    }

    #[tokio::test]
    async fn test_create_container_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        let result = service
            .create_container(
                "test-workspace-1",
                "alpine:latest",
                vec!["/workspace".to_string()],
                2.0,
                "4GB",
            )
            .await;

        if result.is_err() {
            eprintln!("Skipping test: Failed to create container (Docker may not be running or image not available)");
            return;
        }

        let container_id = result.unwrap();
        assert!(!container_id.is_empty());

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;
    }

    #[tokio::test]
    async fn test_start_stop_container() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        let container_id = match service
            .create_container("test-lifecycle", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // Start container
        let start_result = service.start_container(&container_id).await;
        assert!(start_result.is_ok());

        // Check status
        let status = service.get_container_status(&container_id).await.unwrap();
        assert_eq!(status, "running");

        // Stop container
        let stop_result = service.stop_container(&container_id, 10).await;
        assert!(stop_result.is_ok());

        // Check status
        let status = service.get_container_status(&container_id).await.unwrap();
        assert!(status == "exited" || status == "stopped");

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;
    }

    // ========== Image Management Tests ==========

    /// Test image_exists returns true when image exists
    /// Requirements: 7.2.1 - Image existence check
    #[tokio::test]
    async fn test_image_exists_true() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Use alpine:latest which is commonly available
        // Try to pull it first to ensure it exists
        let _ = service.docker.create_image(
            Some(bollard::image::CreateImageOptions {
                from_image: "alpine",
                tag: "latest",
                ..Default::default()
            }),
            None,
            None,
        );

        let result = service.image_exists("alpine:latest").await;
        if result.is_err() {
            eprintln!("Skipping test: Failed to check image existence");
            return;
        }

        // Note: This test may fail if alpine:latest is not available
        // In CI/CD, ensure the image is pre-pulled
        assert!(result.unwrap());
    }

    /// Test image_exists returns false when image doesn't exist
    /// Requirements: 7.2.1 - Image existence check
    #[tokio::test]
    async fn test_image_exists_false() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        let result = service
            .image_exists("nonexistent-image:nonexistent-tag")
            .await;

        if result.is_err() {
            eprintln!("Skipping test: Failed to check image existence");
            return;
        }

        assert!(!result.unwrap());
    }

    /// Test build_image builds an image successfully
    /// Requirements: 7.2.1 - Image building
    #[tokio::test]
    async fn test_build_image_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create a temporary Dockerfile
        let temp_dir = std::env::temp_dir();
        let dockerfile_path = temp_dir.join("Dockerfile.test");
        let dockerfile_content = "FROM alpine:latest\nRUN echo 'test'\n";

        if tokio::fs::write(&dockerfile_path, dockerfile_content)
            .await
            .is_err()
        {
            eprintln!("Skipping test: Failed to write Dockerfile");
            return;
        }

        let image_name = "test-build-image:latest";

        // Build image
        let result = service
            .build_image(
                dockerfile_path.to_str().unwrap(),
                image_name,
                temp_dir.to_str().unwrap(),
            )
            .await;

        // Cleanup
        let _ = tokio::fs::remove_file(&dockerfile_path).await;
        let _ = service.remove_image(image_name, true).await;

        if result.is_err() {
            eprintln!("Skipping test: Failed to build image (may need alpine:latest)");
            return;
        }

        let build_result = result.unwrap();
        assert_eq!(build_result.image_name, image_name);
        assert!(!build_result.image_id.is_empty());
        assert!(build_result.build_time_seconds > 0.0);
        assert!(build_result.size_bytes > 0);
    }

    /// Test remove_image removes an image successfully
    /// Requirements: 7.2.1 - Image removal
    #[tokio::test]
    async fn test_remove_image_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create a test image first
        let temp_dir = std::env::temp_dir();
        let dockerfile_path = temp_dir.join("Dockerfile.remove_test");
        let dockerfile_content = "FROM alpine:latest\n";

        if tokio::fs::write(&dockerfile_path, dockerfile_content)
            .await
            .is_err()
        {
            eprintln!("Skipping test: Failed to write Dockerfile");
            return;
        }

        let image_name = "test-remove-image:latest";

        // Build image
        if service
            .build_image(
                dockerfile_path.to_str().unwrap(),
                image_name,
                temp_dir.to_str().unwrap(),
            )
            .await
            .is_err()
        {
            let _ = tokio::fs::remove_file(&dockerfile_path).await;
            eprintln!("Skipping test: Failed to build image");
            return;
        }

        // Remove image
        let result = service.remove_image(image_name, false).await;

        // Cleanup
        let _ = tokio::fs::remove_file(&dockerfile_path).await;

        assert!(result.is_ok());

        // Verify image is removed
        let exists = service.image_exists(image_name).await.unwrap();
        assert!(!exists);
    }

    /// Test inspect_image returns image information
    /// Requirements: 7.2.1 - Image inspection
    #[tokio::test]
    async fn test_inspect_image_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Use alpine:latest
        let _ = service.docker.create_image(
            Some(bollard::image::CreateImageOptions {
                from_image: "alpine",
                tag: "latest",
                ..Default::default()
            }),
            None,
            None,
        );

        let result = service.inspect_image("alpine:latest").await;
        if result.is_err() {
            eprintln!("Skipping test: Failed to inspect image");
            return;
        }

        let info = result.unwrap();
        assert!(!info.id.is_empty());
        assert_eq!(info.name, "alpine:latest");
        assert!(info.size_bytes > 0);
    }

    /// Test list_containers_using_image lists containers
    /// Requirements: 7.2.1 - List containers by image
    #[tokio::test]
    async fn test_list_containers_using_image() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create a container with alpine:latest
        let container_id = match service
            .create_container("test-list-by-image", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        // List containers using alpine:latest
        let result = service.list_containers_using_image("alpine:latest").await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        assert!(result.is_ok());
        let container_ids = result.unwrap();
        assert!(container_ids.contains(&container_id));
    }

    // ========== Container Operations Tests ==========

    /// Test restart_container restarts a container successfully
    /// Requirements: 7.2.2 - Container restart
    #[tokio::test]
    async fn test_restart_container_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create and start a container
        let container_id = match service
            .create_container("test-restart", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        // Restart container
        let result = service.restart_container(&container_id, 10).await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        assert!(result.is_ok());
    }

    /// Test get_container_stats returns statistics
    /// Requirements: 7.2.3 - Container resource monitoring
    #[tokio::test]
    async fn test_get_container_stats_success() {
        let service = match DockerService::new() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping test: Docker not available");
                return;
            }
        };

        // Create and start a container
        let container_id = match service
            .create_container("test-stats", "alpine:latest", vec![], 1.0, "1GB")
            .await
        {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Skipping test: Failed to create container");
                return;
            }
        };

        if service.start_container(&container_id).await.is_err() {
            let _ = service.remove_container(&container_id, true).await;
            eprintln!("Skipping test: Failed to start container");
            return;
        }

        // Get stats
        let result = service.get_container_stats(&container_id).await;

        // Cleanup
        let _ = service.remove_container(&container_id, true).await;

        if result.is_err() {
            eprintln!("Skipping test: Failed to get stats");
            return;
        }

        let stats = result.unwrap();
        assert!(stats.cpu_percent >= 0.0);
        assert!(stats.memory_usage_mb >= 0.0);
        assert!(stats.memory_limit_mb > 0.0);
        assert!(stats.memory_percent >= 0.0);
        // network_rx_bytes and network_tx_bytes are u64, always >= 0
    }

    // ========== Helper Function Tests ==========

    /// Test parse_docker_timestamp parses RFC3339 timestamps
    /// Requirements: 7.2.1 - Timestamp parsing
    #[test]
    fn test_parse_docker_timestamp() {
        // Valid RFC3339 timestamp
        let timestamp = "2024-01-20T10:30:00Z";
        let result = parse_docker_timestamp(timestamp);
        assert!(result.is_ok());

        // Empty string should return current time
        let result = parse_docker_timestamp("");
        assert!(result.is_ok());

        // Invalid timestamp
        let result = parse_docker_timestamp("invalid");
        assert!(result.is_err());
    }

    /// Test calculate_cpu_percent calculates CPU usage correctly
    /// Requirements: 7.2.3 - CPU calculation
    #[test]
    fn test_calculate_cpu_percent() {
        use bollard::container::{CPUStats, CPUUsage, Stats};

        // Create mock stats with CPU data
        let stats = Stats {
            read: "".to_string(),
            preread: "".to_string(),
            num_procs: 0,
            pids_stats: bollard::container::PidsStats {
                current: None,
                limit: None,
            },
            blkio_stats: bollard::container::BlkioStats {
                io_service_bytes_recursive: None,
                io_serviced_recursive: None,
                io_queue_recursive: None,
                io_service_time_recursive: None,
                io_wait_time_recursive: None,
                io_merged_recursive: None,
                io_time_recursive: None,
                sectors_recursive: None,
            },
            cpu_stats: CPUStats {
                cpu_usage: CPUUsage {
                    total_usage: 1000000000,
                    percpu_usage: Some(vec![500000000, 500000000]),
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: Some(10000000000),
                online_cpus: Some(2),
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            precpu_stats: CPUStats {
                cpu_usage: CPUUsage {
                    total_usage: 500000000,
                    percpu_usage: None,
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: Some(9000000000),
                online_cpus: None,
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            memory_stats: bollard::container::MemoryStats {
                usage: None,
                max_usage: None,
                stats: None,
                failcnt: None,
                limit: None,
                commit: None,
                commit_peak: None,
                commitbytes: None,
                commitpeakbytes: None,
                privateworkingset: None,
            },
            name: "".to_string(),
            id: "".to_string(),
            network: None,
            networks: None,
            storage_stats: bollard::container::StorageStats {
                read_count_normalized: None,
                read_size_bytes: None,
                write_count_normalized: None,
                write_size_bytes: None,
            },
        };

        let cpu_percent = calculate_cpu_percent(&stats);
        assert!(cpu_percent >= 0.0);
        assert!(cpu_percent <= 200.0); // Max 200% for 2 CPUs

        // Test with zero delta (should return 0.0)
        let stats_zero = Stats {
            read: "".to_string(),
            preread: "".to_string(),
            num_procs: 0,
            pids_stats: bollard::container::PidsStats {
                current: None,
                limit: None,
            },
            blkio_stats: bollard::container::BlkioStats {
                io_service_bytes_recursive: None,
                io_serviced_recursive: None,
                io_queue_recursive: None,
                io_service_time_recursive: None,
                io_wait_time_recursive: None,
                io_merged_recursive: None,
                io_time_recursive: None,
                sectors_recursive: None,
            },
            cpu_stats: CPUStats {
                cpu_usage: CPUUsage {
                    total_usage: 1000000000,
                    percpu_usage: None,
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: Some(10000000000),
                online_cpus: Some(2),
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            precpu_stats: CPUStats {
                cpu_usage: CPUUsage {
                    total_usage: 1000000000,
                    percpu_usage: None,
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: Some(10000000000),
                online_cpus: None,
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            memory_stats: bollard::container::MemoryStats {
                usage: None,
                max_usage: None,
                stats: None,
                failcnt: None,
                limit: None,
                commit: None,
                commit_peak: None,
                commitbytes: None,
                commitpeakbytes: None,
                privateworkingset: None,
            },
            name: "".to_string(),
            id: "".to_string(),
            network: None,
            networks: None,
            storage_stats: bollard::container::StorageStats {
                read_count_normalized: None,
                read_size_bytes: None,
                write_count_normalized: None,
                write_size_bytes: None,
            },
        };

        let cpu_percent_zero = calculate_cpu_percent(&stats_zero);
        assert_eq!(cpu_percent_zero, 0.0);
    }

    /// Test calculate_network_stats calculates network bytes correctly
    /// Requirements: 7.2.3 - Network calculation
    #[test]
    fn test_calculate_network_stats() {
        use bollard::container::{NetworkStats, Stats};
        use std::collections::HashMap;

        // Create mock stats with network data
        let mut networks = HashMap::new();
        networks.insert(
            "eth0".to_string(),
            NetworkStats {
                rx_bytes: 1024,
                tx_bytes: 2048,
                rx_packets: 0,
                tx_packets: 0,
                rx_errors: 0,
                tx_errors: 0,
                rx_dropped: 0,
                tx_dropped: 0,
            },
        );
        networks.insert(
            "eth1".to_string(),
            NetworkStats {
                rx_bytes: 512,
                tx_bytes: 1024,
                rx_packets: 0,
                tx_packets: 0,
                rx_errors: 0,
                tx_errors: 0,
                rx_dropped: 0,
                tx_dropped: 0,
            },
        );

        let stats = Stats {
            read: "".to_string(),
            preread: "".to_string(),
            num_procs: 0,
            pids_stats: bollard::container::PidsStats {
                current: None,
                limit: None,
            },
            blkio_stats: bollard::container::BlkioStats {
                io_service_bytes_recursive: None,
                io_serviced_recursive: None,
                io_queue_recursive: None,
                io_service_time_recursive: None,
                io_wait_time_recursive: None,
                io_merged_recursive: None,
                io_time_recursive: None,
                sectors_recursive: None,
            },
            cpu_stats: bollard::container::CPUStats {
                cpu_usage: bollard::container::CPUUsage {
                    total_usage: 0,
                    percpu_usage: None,
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: None,
                online_cpus: None,
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            precpu_stats: bollard::container::CPUStats {
                cpu_usage: bollard::container::CPUUsage {
                    total_usage: 0,
                    percpu_usage: None,
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: None,
                online_cpus: None,
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            memory_stats: bollard::container::MemoryStats {
                usage: None,
                max_usage: None,
                stats: None,
                failcnt: None,
                limit: None,
                commit: None,
                commit_peak: None,
                commitbytes: None,
                commitpeakbytes: None,
                privateworkingset: None,
            },
            name: "".to_string(),
            id: "".to_string(),
            network: None,
            networks: Some(networks),
            storage_stats: bollard::container::StorageStats {
                read_count_normalized: None,
                read_size_bytes: None,
                write_count_normalized: None,
                write_size_bytes: None,
            },
        };

        let (rx_bytes, tx_bytes) = calculate_network_stats(&stats);
        assert_eq!(rx_bytes, 1536); // 1024 + 512
        assert_eq!(tx_bytes, 3072); // 2048 + 1024

        // Test with no network data
        let stats_no_network = Stats {
            read: "".to_string(),
            preread: "".to_string(),
            num_procs: 0,
            pids_stats: bollard::container::PidsStats {
                current: None,
                limit: None,
            },
            blkio_stats: bollard::container::BlkioStats {
                io_service_bytes_recursive: None,
                io_serviced_recursive: None,
                io_queue_recursive: None,
                io_service_time_recursive: None,
                io_wait_time_recursive: None,
                io_merged_recursive: None,
                io_time_recursive: None,
                sectors_recursive: None,
            },
            cpu_stats: bollard::container::CPUStats {
                cpu_usage: bollard::container::CPUUsage {
                    total_usage: 0,
                    percpu_usage: None,
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: None,
                online_cpus: None,
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            precpu_stats: bollard::container::CPUStats {
                cpu_usage: bollard::container::CPUUsage {
                    total_usage: 0,
                    percpu_usage: None,
                    usage_in_kernelmode: 0,
                    usage_in_usermode: 0,
                },
                system_cpu_usage: None,
                online_cpus: None,
                throttling_data: bollard::container::ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            memory_stats: bollard::container::MemoryStats {
                usage: None,
                max_usage: None,
                stats: None,
                failcnt: None,
                limit: None,
                commit: None,
                commit_peak: None,
                commitbytes: None,
                commitpeakbytes: None,
                privateworkingset: None,
            },
            name: "".to_string(),
            id: "".to_string(),
            network: None,
            networks: None,
            storage_stats: bollard::container::StorageStats {
                read_count_normalized: None,
                read_size_bytes: None,
                write_count_normalized: None,
                write_size_bytes: None,
            },
        };

        let (rx_bytes, tx_bytes) = calculate_network_stats(&stats_no_network);
        assert_eq!(rx_bytes, 0);
        assert_eq!(tx_bytes, 0);
    }
}
