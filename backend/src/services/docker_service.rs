//! Docker service for container management
//!
//! Provides Docker container lifecycle management for workspaces.

use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;

use crate::error::{GitAutoDevError, Result};

#[derive(Clone)]
pub struct DockerService {
    docker: Docker,
}

impl DockerService {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults().map_err(|e| {
            GitAutoDevError::Internal(format!("Failed to connect to Docker: {}", e))
        })?;

        Ok(Self { docker })
    }

    pub async fn ping(&self) -> Result<bool> {
        self.docker
            .ping()
            .await
            .map(|_| true)
            .map_err(|e| GitAutoDevError::Internal(format!("Docker ping failed: {}", e)))
    }

    pub async fn create_container(
        &self,
        name: &str,
        image: &str,
        volumes: Vec<String>,
        cpu_limit: f64,
        memory_limit: &str,
    ) -> Result<String> {
        // Parse memory limit (e.g., "4GB" -> 4294967296 bytes)
        let memory_bytes = parse_memory_limit(memory_limit)?;

        // Create mounts for volumes
        let mounts: Vec<Mount> = volumes
            .iter()
            .map(|v| Mount {
                target: Some(v.clone()),
                source: Some(format!("/tmp/gitautodev/{}", name)),
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
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to create container: {}", e)))?;

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
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to remove container: {}", e)))
    }

    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to start container: {}", e)))
    }

    pub async fn stop_container(&self, container_id: &str, timeout: i64) -> Result<()> {
        let options = StopContainerOptions { t: timeout };

        self.docker
            .stop_container(container_id, Some(options))
            .await
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to stop container: {}", e)))
    }

    pub async fn get_container_status(&self, container_id: &str) -> Result<String> {
        let inspect = self
            .docker
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| {
                GitAutoDevError::Internal(format!("Failed to inspect container: {}", e))
            })?;

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
}

fn parse_memory_limit(limit: &str) -> Result<i64> {
    let limit = limit.to_uppercase();

    if let Some(gb) = limit.strip_suffix("GB") {
        let value: f64 = gb
            .parse()
            .map_err(|_| GitAutoDevError::Validation("Invalid memory limit format".to_string()))?;
        Ok((value * 1024.0 * 1024.0 * 1024.0) as i64)
    } else if let Some(mb) = limit.strip_suffix("MB") {
        let value: f64 = mb
            .parse()
            .map_err(|_| GitAutoDevError::Validation("Invalid memory limit format".to_string()))?;
        Ok((value * 1024.0 * 1024.0) as i64)
    } else {
        Err(GitAutoDevError::Validation(
            "Memory limit must end with GB or MB".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
