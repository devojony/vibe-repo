//! Docker service for container management
//!
//! Provides Docker container lifecycle management for workspaces.

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
}
