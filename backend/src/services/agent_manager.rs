//! Agent Manager Service
//!
//! Manages the lifecycle of ACP-compatible agent subprocesses, including:
//! - Spawning agents (OpenCode, Claude Code)
//! - Health monitoring
//! - Graceful shutdown and force kill
//! - Concurrent task limit enforcement
//! - Resource usage tracking

use crate::services::acp::{AcpAgentConfig, AcpClient, AcpError, AcpResult};
use agent_client_protocol::SessionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Agent type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// OpenCode agent (Bun runtime)
    OpenCode,
    /// Claude Code agent (Node.js adapter)
    ClaudeCode,
}

impl AgentType {
    /// Get the command to spawn this agent type
    pub fn command(&self) -> &str {
        match self {
            AgentType::OpenCode => "opencode",
            AgentType::ClaudeCode => "bun",
        }
    }

    /// Get the arguments for spawning this agent type
    pub fn args(&self) -> Vec<String> {
        match self {
            AgentType::OpenCode => vec!["acp".to_string()],
            AgentType::ClaudeCode => vec!["claude-code-acp".to_string()],
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::OpenCode => write!(f, "opencode"),
            AgentType::ClaudeCode => write!(f, "claude-code"),
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "opencode" => Ok(AgentType::OpenCode),
            "claude-code" | "claudecode" => Ok(AgentType::ClaudeCode),
            _ => Err(format!("Unknown agent type: {}", s)),
        }
    }
}

/// Configuration for spawning an agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Type of agent to spawn
    pub agent_type: AgentType,
    /// API key for agent authentication (optional)
    pub api_key: Option<String>,
    /// Model to use for agent
    pub model: Option<String>,
    /// Timeout in seconds for agent operations
    pub timeout: u64,
    /// Working directory for the agent
    pub working_dir: PathBuf,
    /// Container ID to run agent in (if None, runs on host)
    pub container_id: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_type: AgentType::OpenCode,
            api_key: None,
            model: None,
            timeout: 600, // 10 minutes
            working_dir: PathBuf::from("."),
            container_id: None,
        }
    }
}

/// Resource usage statistics for an agent
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Running time in seconds
    pub running_time_secs: u64,
}

/// Status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is starting up
    Starting,
    /// Agent is running normally
    Running,
    /// Agent is stopping
    Stopping,
    /// Agent has stopped
    Stopped,
    /// Agent has crashed
    Crashed,
}

/// Handle to a running agent
pub struct AgentHandle {
    /// Unique identifier for this agent
    pub id: String,
    /// Agent configuration
    pub config: AgentConfig,
    /// ACP client for communication
    client: Arc<Mutex<AcpClient>>,
    /// Current session ID
    session_id: Arc<Mutex<Option<SessionId>>>,
    /// Agent status
    status: Arc<RwLock<AgentStatus>>,
    /// Start time
    start_time: Instant,
    /// Resource usage
    resource_usage: Arc<RwLock<ResourceUsage>>,
}

impl AgentHandle {
    /// Create a new agent handle
    fn new(id: String, config: AgentConfig, client: AcpClient) -> Self {
        Self {
            id,
            config,
            client: Arc::new(Mutex::new(client)),
            session_id: Arc::new(Mutex::new(None)),
            status: Arc::new(RwLock::new(AgentStatus::Starting)),
            start_time: Instant::now(),
            resource_usage: Arc::new(RwLock::new(ResourceUsage::default())),
        }
    }

    /// Get the current status
    pub async fn status(&self) -> AgentStatus {
        *self.status.read().await
    }

    /// Set the status
    async fn set_status(&self, status: AgentStatus) {
        *self.status.write().await = status;
    }

    /// Get the current session ID
    pub async fn session_id(&self) -> Option<SessionId> {
        self.session_id.lock().await.clone()
    }
    
    /// Get the event store from the ACP client
    pub async fn event_store(&self) -> Arc<Mutex<crate::services::acp::EventStore>> {
        let client = self.client.lock().await;
        client.event_store()
    }

    /// Get resource usage
    pub async fn resource_usage(&self) -> ResourceUsage {
        self.resource_usage.read().await.clone()
    }

    /// Update resource usage
    async fn update_resource_usage(&self, usage: ResourceUsage) {
        *self.resource_usage.write().await = usage;
    }

    /// Initialize the agent and create a session
    pub async fn initialize(&self) -> AcpResult<SessionId> {
        info!("Initializing agent {}", self.id);
        
        let mut client = self.client.lock().await;
        client.initialize().await?;
        
        let session_id = client.new_session().await?;
        *self.session_id.lock().await = Some(session_id.clone());
        
        self.set_status(AgentStatus::Running).await;
        info!("Agent {} initialized with session {:?}", self.id, session_id);
        
        Ok(session_id)
    }

    /// Send a prompt to the agent
    pub async fn prompt(&self, prompt: String) -> AcpResult<()> {
        let session_id = self.session_id.lock().await
            .clone()
            .ok_or_else(|| AcpError::Other("No active session".to_string()))?;
        
        let client = self.client.lock().await;
        client.prompt(&session_id, prompt).await
    }

    /// Cancel the current operation
    pub async fn cancel(&self) -> AcpResult<()> {
        let session_id = self.session_id.lock().await
            .clone()
            .ok_or_else(|| AcpError::Other("No active session".to_string()))?;
        
        let client = self.client.lock().await;
        client.cancel(&session_id).await
    }

    /// Check if the agent is healthy
    pub async fn is_healthy(&self) -> bool {
        let status = self.status().await;
        matches!(status, AgentStatus::Running)
    }

    /// Shutdown the agent
    /// 
    /// Note: ACP protocol does not define a graceful shutdown.
    /// This method cancels active sessions and kills the process.
    pub async fn shutdown(&self, timeout: Duration) -> AcpResult<()> {
        info!("Shutting down agent {}", self.id);
        self.set_status(AgentStatus::Stopping).await;
        
        // Try to cancel any active session first
        if let Some(_session_id) = self.session_id.lock().await.as_ref() {
            info!("Cancelling active session for agent {}", self.id);
            let _ = self.cancel().await;
            
            // Give agent a moment to process cancellation
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Shutdown the client (kills the process)
        let mut client = self.client.lock().await;
        let result = tokio::time::timeout(timeout, client.shutdown()).await;
        
        // Always mark as stopped, even if shutdown had errors
        self.set_status(AgentStatus::Stopped).await;
        
        match result {
            Ok(Ok(())) => {
                info!("Agent {} shut down successfully", self.id);
                Ok(())
            }
            Ok(Err(e)) => {
                warn!("Error during agent {} shutdown: {}", self.id, e);
                // Still return Ok since process is dead
                Ok(())
            }
            Err(_) => {
                warn!("Agent {} shutdown timed out, but process should be dead", self.id);
                // Still return Ok since we force killed the process
                Ok(())
            }
        }
    }

    /// Force kill the agent immediately without cancellation
    pub async fn force_kill(&self) -> AcpResult<()> {
        warn!("Force killing agent {}", self.id);
        self.set_status(AgentStatus::Stopped).await;
        
        let mut client = self.client.lock().await;
        let _ = client.shutdown().await;
        
        info!("Agent {} force killed", self.id);
        Ok(())
    }

    /// Get running time in seconds
    pub fn running_time_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Agent Manager Service
pub struct AgentManager {
    /// Active agents by task ID
    agents: Arc<RwLock<HashMap<String, Arc<AgentHandle>>>>,
    /// Maximum concurrent agents
    max_concurrent: usize,
    /// Default agent configuration
    default_config: AgentConfig,
}

impl AgentManager {
    /// Create a new agent manager
    pub fn new(max_concurrent: usize, default_config: AgentConfig) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent,
            default_config,
        }
    }

    /// Get the number of active agents
    pub async fn active_count(&self) -> usize {
        self.agents.read().await.len()
    }

    /// Check if we can spawn a new agent
    pub async fn can_spawn(&self) -> bool {
        self.active_count().await < self.max_concurrent
    }

    /// Spawn a new agent
    pub async fn spawn_agent(&self, task_id: String, config: AgentConfig) -> AcpResult<Arc<AgentHandle>> {
        // Check concurrent limit
        if !self.can_spawn().await {
            return Err(AcpError::Other(format!(
                "Maximum concurrent agents ({}) reached",
                self.max_concurrent
            )));
        }

        info!("Spawning {} agent for task {}", config.agent_type, task_id);

        // Create ACP client configuration
        let mut acp_config = AcpAgentConfig {
            command: config.agent_type.command().to_string(),
            args: config.agent_type.args(),
            env: Vec::new(),
            working_dir: config.working_dir.clone(),
            timeout: config.timeout,
            container_id: config.container_id.clone(),
            mcp_servers: Vec::new(),
        };

        // Load MCP configuration for the workspace
        let global_config_dir = PathBuf::from("./data/vibe-repo/config");
        if let Err(e) = acp_config.load_mcp_servers(&config.working_dir, &global_config_dir) {
            warn!("Failed to load MCP configuration: {}", e);
            // Continue without MCP servers - this is not a fatal error
        }

        // Add API key to environment if provided
        if let Some(api_key) = &config.api_key {
            let env_var = match config.agent_type {
                AgentType::OpenCode => "ANTHROPIC_API_KEY",
                AgentType::ClaudeCode => "ANTHROPIC_API_KEY",
            };
            acp_config.env.push((env_var.to_string(), api_key.clone()));
        }

        // Add model to environment if provided
        if let Some(model) = &config.model {
            acp_config.env.push(("AGENT_MODEL".to_string(), model.clone()));
        }

        // Create ACP client
        let client = AcpClient::new(acp_config);

        // Create agent handle
        let handle = Arc::new(AgentHandle::new(task_id.clone(), config, client));

        // Store in active agents
        self.agents.write().await.insert(task_id.clone(), handle.clone());

        info!("Agent spawned for task {}", task_id);
        Ok(handle)
    }

    /// Spawn an OpenCode agent
    pub async fn spawn_opencode(&self, task_id: String, working_dir: PathBuf, api_key: Option<String>) -> AcpResult<Arc<AgentHandle>> {
        let config = AgentConfig {
            agent_type: AgentType::OpenCode,
            api_key,
            model: None,
            timeout: self.default_config.timeout,
            working_dir,
            container_id: None,
        };
        self.spawn_agent(task_id, config).await
    }

    /// Spawn a Claude Code agent
    pub async fn spawn_claude_code(&self, task_id: String, working_dir: PathBuf, api_key: Option<String>) -> AcpResult<Arc<AgentHandle>> {
        let config = AgentConfig {
            agent_type: AgentType::ClaudeCode,
            api_key,
            model: None,
            timeout: self.default_config.timeout,
            working_dir,
            container_id: None,
        };
        self.spawn_agent(task_id, config).await
    }

    /// Get an agent by task ID
    pub async fn get_agent(&self, task_id: &str) -> Option<Arc<AgentHandle>> {
        self.agents.read().await.get(task_id).cloned()
    }

    /// Remove an agent from tracking
    pub async fn remove_agent(&self, task_id: &str) -> Option<Arc<AgentHandle>> {
        self.agents.write().await.remove(task_id)
    }

    /// Monitor agent health
    pub async fn monitor_health(&self) {
        let agents = self.agents.read().await;
        for (task_id, handle) in agents.iter() {
            let status = handle.status().await;
            let running_time = handle.running_time_secs();
            
            debug!(
                "Agent {} status: {:?}, running time: {}s",
                task_id, status, running_time
            );

            // Update resource usage (simplified - in production, would query actual process stats)
            let usage = ResourceUsage {
                cpu_percent: 0.0, // TODO: Implement actual CPU monitoring
                memory_bytes: 0,  // TODO: Implement actual memory monitoring
                running_time_secs: running_time,
            };
            handle.update_resource_usage(usage).await;

            // Check for unhealthy agents
            if !handle.is_healthy().await {
                warn!("Agent {} is unhealthy: {:?}", task_id, status);
            }
        }
    }

    /// Gracefully shutdown an agent
    pub async fn shutdown_agent(&self, task_id: &str, timeout: Duration) -> AcpResult<()> {
        if let Some(handle) = self.get_agent(task_id).await {
            let result = handle.shutdown(timeout).await;
            self.remove_agent(task_id).await;
            result
        } else {
            Err(AcpError::Other(format!("Agent not found: {}", task_id)))
        }
    }

    /// Force kill an agent
    pub async fn force_kill_agent(&self, task_id: &str) -> AcpResult<()> {
        if let Some(handle) = self.get_agent(task_id).await {
            let result = handle.force_kill().await;
            self.remove_agent(task_id).await;
            result
        } else {
            Err(AcpError::Other(format!("Agent not found: {}", task_id)))
        }
    }

    /// Shutdown all agents
    pub async fn shutdown_all(&self, timeout: Duration) {
        info!("Shutting down all agents");
        let agents: Vec<_> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };

        for task_id in agents {
            if let Err(e) = self.shutdown_agent(&task_id, timeout).await {
                error!("Error shutting down agent {}: {}", task_id, e);
            }
        }
    }

    /// Get all active agent IDs
    pub async fn active_agents(&self) -> Vec<String> {
        self.agents.read().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::OpenCode.to_string(), "opencode");
        assert_eq!(AgentType::ClaudeCode.to_string(), "claude-code");
    }

    #[test]
    fn test_agent_type_from_str() {
        assert_eq!("opencode".parse::<AgentType>().unwrap(), AgentType::OpenCode);
        assert_eq!("OpenCode".parse::<AgentType>().unwrap(), AgentType::OpenCode);
        assert_eq!("claude-code".parse::<AgentType>().unwrap(), AgentType::ClaudeCode);
        assert_eq!("claudecode".parse::<AgentType>().unwrap(), AgentType::ClaudeCode);
        assert!("invalid".parse::<AgentType>().is_err());
    }

    #[test]
    fn test_agent_type_command() {
        assert_eq!(AgentType::OpenCode.command(), "opencode");
        assert_eq!(AgentType::ClaudeCode.command(), "bun");
    }

    #[test]
    fn test_agent_type_args() {
        assert_eq!(AgentType::OpenCode.args(), vec!["acp"]);
        assert_eq!(AgentType::ClaudeCode.args(), vec!["claude-code-acp"]);
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.agent_type, AgentType::OpenCode);
        assert_eq!(config.timeout, 600);
        assert!(config.api_key.is_none());
        assert!(config.model.is_none());
    }

    #[tokio::test]
    async fn test_agent_manager_creation() {
        let config = AgentConfig::default();
        let manager = AgentManager::new(5, config);
        
        assert_eq!(manager.active_count().await, 0);
        assert!(manager.can_spawn().await);
    }

    #[tokio::test]
    async fn test_agent_manager_concurrent_limit() {
        let config = AgentConfig::default();
        let manager = AgentManager::new(2, config);
        
        assert!(manager.can_spawn().await);
        
        // Simulate adding agents
        let agents = manager.agents.clone();
        let mut agents_guard = agents.write().await;
        
        // Add dummy agents
        for i in 0..2 {
            let task_id = format!("task-{}", i);
            let client = AcpClient::new(AcpAgentConfig::default());
            let handle = Arc::new(AgentHandle::new(
                task_id.clone(),
                AgentConfig::default(),
                client,
            ));
            agents_guard.insert(task_id, handle);
        }
        drop(agents_guard);
        
        assert_eq!(manager.active_count().await, 2);
        assert!(!manager.can_spawn().await);
    }

    #[tokio::test]
    async fn test_agent_manager_get_agent() {
        let config = AgentConfig::default();
        let manager = AgentManager::new(5, config);
        
        // Add a dummy agent
        let task_id = "test-task".to_string();
        let client = AcpClient::new(AcpAgentConfig::default());
        let handle = Arc::new(AgentHandle::new(
            task_id.clone(),
            AgentConfig::default(),
            client,
        ));
        manager.agents.write().await.insert(task_id.clone(), handle);
        
        // Get the agent
        let agent = manager.get_agent(&task_id).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().id, task_id);
        
        // Try to get non-existent agent
        let agent = manager.get_agent("non-existent").await;
        assert!(agent.is_none());
    }

    #[tokio::test]
    async fn test_agent_manager_remove_agent() {
        let config = AgentConfig::default();
        let manager = AgentManager::new(5, config);
        
        // Add a dummy agent
        let task_id = "test-task".to_string();
        let client = AcpClient::new(AcpAgentConfig::default());
        let handle = Arc::new(AgentHandle::new(
            task_id.clone(),
            AgentConfig::default(),
            client,
        ));
        manager.agents.write().await.insert(task_id.clone(), handle);
        
        assert_eq!(manager.active_count().await, 1);
        
        // Remove the agent
        let removed = manager.remove_agent(&task_id).await;
        assert!(removed.is_some());
        assert_eq!(manager.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_agent_manager_active_agents() {
        let config = AgentConfig::default();
        let manager = AgentManager::new(5, config);
        
        // Add multiple dummy agents
        for i in 0..3 {
            let task_id = format!("task-{}", i);
            let client = AcpClient::new(AcpAgentConfig::default());
            let handle = Arc::new(AgentHandle::new(
                task_id.clone(),
                AgentConfig::default(),
                client,
            ));
            manager.agents.write().await.insert(task_id, handle);
        }
        
        let active = manager.active_agents().await;
        assert_eq!(active.len(), 3);
        assert!(active.contains(&"task-0".to_string()));
        assert!(active.contains(&"task-1".to_string()));
        assert!(active.contains(&"task-2".to_string()));
    }

    #[tokio::test]
    async fn test_agent_handle_status() {
        let client = AcpClient::new(AcpAgentConfig::default());
        let handle = AgentHandle::new(
            "test-task".to_string(),
            AgentConfig::default(),
            client,
        );
        
        assert_eq!(handle.status().await, AgentStatus::Starting);
        
        handle.set_status(AgentStatus::Running).await;
        assert_eq!(handle.status().await, AgentStatus::Running);
        assert!(handle.is_healthy().await);
        
        handle.set_status(AgentStatus::Crashed).await;
        assert_eq!(handle.status().await, AgentStatus::Crashed);
        assert!(!handle.is_healthy().await);
    }

    #[tokio::test]
    async fn test_agent_handle_resource_usage() {
        let client = AcpClient::new(AcpAgentConfig::default());
        let handle = AgentHandle::new(
            "test-task".to_string(),
            AgentConfig::default(),
            client,
        );
        
        let usage = handle.resource_usage().await;
        assert_eq!(usage.cpu_percent, 0.0);
        assert_eq!(usage.memory_bytes, 0);
        
        let new_usage = ResourceUsage {
            cpu_percent: 25.5,
            memory_bytes: 1024 * 1024 * 100, // 100 MB
            running_time_secs: 60,
        };
        handle.update_resource_usage(new_usage.clone()).await;
        
        let usage = handle.resource_usage().await;
        assert_eq!(usage.cpu_percent, 25.5);
        assert_eq!(usage.memory_bytes, 1024 * 1024 * 100);
        assert_eq!(usage.running_time_secs, 60);
    }

    #[tokio::test]
    async fn test_agent_handle_running_time() {
        let client = AcpClient::new(AcpAgentConfig::default());
        let handle = AgentHandle::new(
            "test-task".to_string(),
            AgentConfig::default(),
            client,
        );
        
        // Should be close to 0 initially
        let running_time = handle.running_time_secs();
        assert!(running_time < 2);
        
        // Wait a bit
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Should be at least 1 second
        let running_time = handle.running_time_secs();
        assert!(running_time >= 1);
    }

    #[test]
    fn test_resource_usage_default() {
        let usage = ResourceUsage::default();
        assert_eq!(usage.cpu_percent, 0.0);
        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.running_time_secs, 0);
    }
}
