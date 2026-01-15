//! Service manager
//!
//! Manages the lifecycle of all background services.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::error::Result;
use crate::state::AppState;

/// Trait for background services
#[async_trait]
pub trait BackgroundService: Send + Sync {
    /// Service name for logging and identification
    fn name(&self) -> &'static str;

    /// Start the service (called once at application startup)
    async fn start(&self, state: Arc<AppState>) -> Result<()>;

    /// Stop the service gracefully (called on shutdown)
    async fn stop(&self) -> Result<()>;

    /// Check if the service is healthy
    async fn health_check(&self) -> bool;
}

/// Service manager coordinates all background services
pub struct ServiceManager {
    services: Vec<Box<dyn BackgroundService>>,
    #[allow(dead_code)]
    handles: Vec<JoinHandle<()>>,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            handles: Vec::new(),
        }
    }

    /// Register a background service
    pub fn register<S: BackgroundService + 'static>(&mut self, service: S) {
        self.services.push(Box::new(service));
    }

    /// Start all registered services
    pub async fn start_all(&mut self, state: Arc<AppState>) -> Result<()> {
        for service in &self.services {
            tracing::info!("Starting service: {}", service.name());
            if let Err(e) = service.start(state.clone()).await {
                tracing::error!("Failed to start service {}: {}", service.name(), e);
                // Continue starting other services even if one fails
            }
        }
        Ok(())
    }

    /// Stop all services gracefully
    pub async fn stop_all(&mut self) -> Result<()> {
        for service in &self.services {
            tracing::info!("Stopping service: {}", service.name());
            if let Err(e) = service.stop().await {
                tracing::error!("Failed to stop service {}: {}", service.name(), e);
            }
        }
        Ok(())
    }

    /// Check health of all services
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut status = HashMap::new();
        for service in &self.services {
            let healthy = service.health_check().await;
            status.insert(service.name().to_string(), healthy);
        }
        status
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
