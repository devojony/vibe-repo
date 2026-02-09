//! ACP client wrapper for VibeRepo
//!
//! This module provides a high-level wrapper around the official `agent-client-protocol` SDK,
//! tailored for VibeRepo's task execution needs.

use agent_client_protocol as acp;
use agent_client_protocol::Agent as _;
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use futures::TryStreamExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{debug, error, info, warn};

use super::error::{AcpError, AcpResult};
use super::permissions::{PermissionPolicy, PermissionRequest, ToolKind};

/// Configuration for spawning an ACP agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Command to execute (e.g., "opencode", "claude")
    pub command: String,
    /// Arguments to pass to the command
    pub args: Vec<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Working directory for the agent
    pub working_dir: PathBuf,
    /// Timeout for operations (in seconds)
    pub timeout: u64,
    /// Container ID to run agent in (if None, runs on host)
    pub container_id: Option<String>,
    /// MCP servers to configure for the agent
    pub mcp_servers: Vec<crate::config::mcp::McpServerConfig>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "opencode".to_string(),
            args: vec!["acp".to_string()],
            env: Vec::new(),
            working_dir: PathBuf::from("."),
            timeout: 600, // 10 minutes
            container_id: None,
            mcp_servers: Vec::new(),
        }
    }
}

impl AgentConfig {
    /// Load MCP configuration for a workspace
    ///
    /// # Arguments
    ///
    /// * `workspace_dir` - Path to workspace directory
    /// * `global_config_dir` - Path to global configuration directory
    pub fn load_mcp_servers(
        &mut self,
        workspace_dir: &std::path::Path,
        global_config_dir: &std::path::Path,
    ) -> crate::error::Result<()> {
        use crate::config::mcp::McpConfigLoader;

        let loader = McpConfigLoader::new(global_config_dir.to_path_buf());
        let config = loader.load_for_workspace(workspace_dir)?;

        self.mcp_servers = config.servers;

        Ok(())
    }
}

use super::events::{parse_session_update, EventStore};

/// A client implementation that handles requests from the agent
struct VibeRepoClient {
    /// Event store for tracking agent events
    event_store: Arc<Mutex<EventStore>>,
    /// Permission policy for evaluating agent requests
    permission_policy: Arc<Mutex<PermissionPolicy>>,
}

impl VibeRepoClient {
    fn new(event_store: Arc<Mutex<EventStore>>, permission_policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self {
            event_store,
            permission_policy,
        }
    }
}

#[async_trait(?Send)]
impl acp::Client for VibeRepoClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        debug!("Permission request for tool call: {:?}", args.tool_call);
        
        // Extract tool information from the tool call fields
        let tool_name = args.tool_call.fields.title.as_deref().unwrap_or("unknown");
        
        // Convert tool call to our internal permission request format
        let tool_kind = match tool_name.to_lowercase().as_str() {
            s if s.contains("read") => ToolKind::Read,
            s if s.contains("write") || s.contains("edit") => ToolKind::Write,
            s if s.contains("bash") || s.contains("execute") || s.contains("terminal") => ToolKind::Execute,
            s if s.contains("delete") || s.contains("remove") => ToolKind::Delete,
            s if s.contains("search") || s.contains("grep") || s.contains("glob") || s.contains("find") => ToolKind::Search,
            _ => {
                warn!("Unknown tool type: {}, defaulting to Execute", tool_name);
                ToolKind::Execute
            }
        };

        // Try to extract path, command, and args from raw_input if available
        // This is a best-effort approach since the tool call format may vary
        let path = None; // TODO: Parse from raw_input if needed
        let command = None; // TODO: Parse from raw_input if needed
        let cmd_args = None; // TODO: Parse from raw_input if needed

        let request = PermissionRequest {
            tool_kind,
            path,
            command,
            args: cmd_args,
        };

        // Evaluate permission
        let policy = self.permission_policy.lock().await;
        let decision = policy.evaluate(&request);
        drop(policy); // Release lock

        // Log the decision
        info!(
            "Permission {} for tool '{}': {}",
            if decision.is_allowed() { "ALLOWED" } else { "DENIED" },
            tool_name,
            decision.reason()
        );

        // Find the appropriate option to select based on our decision
        // ACP protocol requires us to select one of the provided options
        let selected_option = if decision.is_allowed() {
            // Look for "allow" option
            args.options.iter()
                .find(|opt| matches!(opt.kind, acp::PermissionOptionKind::AllowOnce | acp::PermissionOptionKind::AllowAlways))
                .or_else(|| args.options.first())
        } else {
            // Look for "reject" option
            args.options.iter()
                .find(|opt| matches!(opt.kind, acp::PermissionOptionKind::RejectOnce | acp::PermissionOptionKind::RejectAlways))
                .or_else(|| args.options.first())
        };

        // If no option found, return an error
        let option_id = selected_option
            .map(|opt| opt.option_id.clone())
            .ok_or_else(|| {
                error!("No permission options provided in request");
                acp::Error::method_not_found()
            })?;

        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(option_id))
        ))
    }

    async fn write_text_file(
        &self,
        _args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        // Not implemented - agents should use their own file operations
        Err(acp::Error::method_not_found())
    }

    async fn read_text_file(
        &self,
        _args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        // Not implemented - agents should use their own file operations
        Err(acp::Error::method_not_found())
    }

    async fn create_terminal(
        &self,
        _args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        // Not implemented - agents should use their own terminal
        Err(acp::Error::method_not_found())
    }

    async fn terminal_output(
        &self,
        _args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn release_terminal(
        &self,
        _args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn wait_for_terminal_exit(
        &self,
        _args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn kill_terminal_command(
        &self,
        _args: acp::KillTerminalCommandRequest,
    ) -> acp::Result<acp::KillTerminalCommandResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(
        &self,
        args: acp::SessionNotification,
    ) -> acp::Result<(), acp::Error> {
        // Parse and store events
        let events = parse_session_update(&args.update);
        
        if !events.is_empty() {
            let mut store = self.event_store.lock().await;
            for event in &events {
                debug!("Storing event: {:?}", event.event_type());
                store.add_event(event.clone());
            }
        }
        
        // Log session updates for debugging
        match &args.update {
            acp::SessionUpdate::UserMessageChunk(chunk) => {
                let text = match &chunk.content {
                    acp::ContentBlock::Text(text_content) => &text_content.text,
                    acp::ContentBlock::Image(_) => "<image>",
                    acp::ContentBlock::Audio(_) => "<audio>",
                    acp::ContentBlock::ResourceLink(resource_link) => &resource_link.uri,
                    acp::ContentBlock::Resource(_) => "<resource>",
                    _ => "<unknown>",
                };
                debug!("User message: {}", text);
            }
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                let text = match &chunk.content {
                    acp::ContentBlock::Text(text_content) => &text_content.text,
                    acp::ContentBlock::Image(_) => "<image>",
                    acp::ContentBlock::Audio(_) => "<audio>",
                    acp::ContentBlock::ResourceLink(resource_link) => &resource_link.uri,
                    acp::ContentBlock::Resource(_) => "<resource>",
                    _ => "<unknown>",
                };
                debug!("Agent message: {}", text);
            }
            acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                let text = match &chunk.content {
                    acp::ContentBlock::Text(text_content) => &text_content.text,
                    acp::ContentBlock::Image(_) => "<image>",
                    acp::ContentBlock::Audio(_) => "<audio>",
                    acp::ContentBlock::ResourceLink(resource_link) => &resource_link.uri,
                    acp::ContentBlock::Resource(_) => "<resource>",
                    _ => "<unknown>",
                };
                debug!("Agent thought: {}", text);
            }
            acp::SessionUpdate::Plan(plan) => {
                info!("Agent plan: {} entries", plan.entries.len());
            }
            acp::SessionUpdate::ToolCall(tool_call) => {
                info!("Tool call: {}", tool_call.title);
            }
            acp::SessionUpdate::ToolCallUpdate(update) => {
                debug!("Tool call update: {:?}", update);
            }
            acp::SessionUpdate::AvailableCommandsUpdate(update) => {
                info!("Available commands updated: {} commands", update.available_commands.len());
            }
            acp::SessionUpdate::CurrentModeUpdate(update) => {
                info!("Current mode changed to: {:?}", update.current_mode_id);
            }
            acp::SessionUpdate::ConfigOptionUpdate(update) => {
                debug!("Config option updated: {:?}", update);
            }
            _ => {
                debug!("Other session update: {:?}", args.update);
            }
        }
        Ok(())
    }

    async fn ext_method(&self, _args: acp::ExtRequest) -> acp::Result<acp::ExtResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(&self, _args: acp::ExtNotification) -> acp::Result<()> {
        Err(acp::Error::method_not_found())
    }
}

/// High-level ACP client for VibeRepo
pub struct AcpClient {
    /// Configuration for the agent
    config: AgentConfig,
    /// The agent subprocess
    child: Arc<Mutex<Option<Child>>>,
    /// The ACP connection
    connection: Option<acp::ClientSideConnection>,
    /// Current session ID
    session_id: Arc<Mutex<Option<acp::SessionId>>>,
    /// Event store for tracking agent events
    event_store: Arc<Mutex<EventStore>>,
    /// Permission policy for evaluating agent requests
    permission_policy: Arc<Mutex<PermissionPolicy>>,
}

impl AcpClient {
    /// Create a new ACP client with the given configuration
    pub fn new(config: AgentConfig) -> Self {
        // Create permission policy based on working directory
        let permission_policy = PermissionPolicy::new(config.working_dir.clone());
        
        Self {
            config,
            child: Arc::new(Mutex::new(None)),
            connection: None,
            session_id: Arc::new(Mutex::new(None)),
            event_store: Arc::new(Mutex::new(EventStore::new())),
            permission_policy: Arc::new(Mutex::new(permission_policy)),
        }
    }
    
    /// Get the event store
    pub fn event_store(&self) -> Arc<Mutex<EventStore>> {
        self.event_store.clone()
    }

    /// Spawn the agent subprocess and initialize the connection
    /// 
    /// This must be called within a LocalSet context
    pub async fn initialize(&mut self) -> AcpResult<()> {
        info!("Spawning agent: {} {:?}", self.config.command, self.config.args);

        // Check if we should run in a container
        if let Some(container_id) = self.config.container_id.clone() {
            // Run agent in container using docker exec
            self.initialize_in_container(&container_id).await
        } else {
            // Run agent on host
            self.initialize_on_host().await
        }
    }

    /// Initialize agent on host machine
    async fn initialize_on_host(&mut self) -> AcpResult<()> {
        // Spawn the agent subprocess
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .current_dir(&self.config.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;

        // Get stdin/stdout for communication
        let stdin = child.stdin.take().ok_or_else(|| {
            AcpError::Other("Failed to get stdin from child process".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AcpError::Other("Failed to get stdout from child process".to_string())
        })?;

        // Store the child process
        *self.child.lock().await = Some(child);

        // Create the ACP connection
        self.create_connection(stdin, stdout).await
    }

    /// Initialize agent in Docker container
    async fn initialize_in_container(&mut self, container_id: &str) -> AcpResult<()> {
        use bollard::exec::{CreateExecOptions, StartExecResults};
        use bollard::Docker;

        info!("Spawning agent in container: {}", container_id);

        // Connect to Docker
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| AcpError::Other(format!("Failed to connect to Docker: {}", e)))?;

        // Build command for docker exec
        let mut exec_cmd = vec![self.config.command.clone()];
        exec_cmd.extend(self.config.args.clone());

        // Build environment variables
        let env_vars: Vec<String> = self.config.env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Create exec instance without TTY mode for JSON-RPC protocol
        // TTY mode can interfere with JSON-RPC because of line buffering
        let exec_config = CreateExecOptions {
            cmd: Some(exec_cmd),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false), // Don't use TTY for JSON-RPC
            env: Some(env_vars),
            working_dir: Some(self.config.working_dir.to_string_lossy().to_string()),
            ..Default::default()
        };

        let exec = docker
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| AcpError::Other(format!("Failed to create exec: {}", e)))?;

        // Start exec
        let exec_id = exec.id.clone();
        let start_result = docker
            .start_exec(&exec_id, None)
            .await
            .map_err(|e| AcpError::Other(format!("Failed to start exec: {}", e)))?;

        // Get the streams
        let (output, input) = match start_result {
            StartExecResults::Attached { output, input } => (output, input),
            StartExecResults::Detached => {
                return Err(AcpError::Other("Exec started in detached mode".to_string()));
            }
        };

        // Note: We can't store the Docker exec process in self.child since it's not a tokio::process::Child
        // We'll need to handle cleanup differently for container-based agents

        // Convert the Docker streams to the format expected by ACP
        // Docker exec with TTY returns streams that implement tokio::io traits
        // We need to convert them to futures::io traits using tokio_util::compat
        use tokio_util::compat::TokioAsyncWriteCompatExt;
        
        // Convert the output stream to AsyncRead
        // The stream returns Result<LogOutput, bollard::errors::Error>
        // We need to map it to Result<Bytes, std::io::Error> and then convert to AsyncRead
        use futures::StreamExt;
        let output_mapped = output.map(|result| {
            result
                .map(|log_output| {
                    use bollard::container::LogOutput;
                    match log_output {
                        LogOutput::StdOut { message } | LogOutput::StdErr { message } => message,
                        _ => axum::body::Bytes::new(),
                    }
                })
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        });
        
        // into_async_read() returns a futures::io::AsyncRead, which is what ACP expects
        let output_reader = output_mapped.into_async_read();
        
        // Create the ACP connection using the Docker exec streams
        let client = VibeRepoClient::new(self.event_store.clone(), self.permission_policy.clone());

        let (conn, handle_io) = acp::ClientSideConnection::new(
            client,
            input.compat_write(),
            output_reader,
            |fut: LocalBoxFuture<'static, ()>| {
                tokio::task::spawn_local(fut);
            },
        );

        // Spawn the I/O handler
        tokio::task::spawn_local(handle_io);

        // Send initialize request
        info!("Sending initialize request");
        let implementation = acp::Implementation::new(
            "vibe-repo".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ).title("VibeRepo".to_string());
        
        // Explicitly declare client capabilities
        // We don't support file operations or terminal - agents should use their own
        let capabilities = acp::ClientCapabilities::new()
            .fs(acp::FileSystemCapability::new()
                .read_text_file(false)
                .write_text_file(false))
            .terminal(false);
        
        let init_request = acp::InitializeRequest::new(acp::ProtocolVersion::V1)
            .client_capabilities(capabilities)
            .client_info(implementation);
        
        conn.initialize(init_request).await?;

        info!("Agent initialized successfully in container");
        self.connection = Some(conn);
        Ok(())
    }

    /// Create ACP connection with tokio streams
    async fn create_connection<R, W>(&mut self, stdin: W, stdout: R) -> AcpResult<()>
    where
        R: tokio::io::AsyncRead + Unpin + 'static,
        W: tokio::io::AsyncWrite + Unpin + 'static,
    {
        // Create the ACP connection
        let client = VibeRepoClient::new(self.event_store.clone(), self.permission_policy.clone());

        let (conn, handle_io) = acp::ClientSideConnection::new(
            client,
            stdin.compat_write(),
            stdout.compat(),
            |fut: LocalBoxFuture<'static, ()>| {
                tokio::task::spawn_local(fut);
            },
        );

        // Spawn the I/O handler
        tokio::task::spawn_local(handle_io);

        // Send initialize request
        info!("Sending initialize request");
        let implementation = acp::Implementation::new(
            "vibe-repo".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ).title("VibeRepo".to_string());
        
        // Explicitly declare client capabilities
        // We don't support file operations or terminal - agents should use their own
        let capabilities = acp::ClientCapabilities::new()
            .fs(acp::FileSystemCapability::new()
                .read_text_file(false)
                .write_text_file(false))
            .terminal(false);
        
        let init_request = acp::InitializeRequest::new(acp::ProtocolVersion::V1)
            .client_capabilities(capabilities)
            .client_info(implementation);
        
        conn.initialize(init_request).await?;

        info!("Agent initialized successfully");
        self.connection = Some(conn);
        Ok(())
    }

    /// Create a new session with the agent
    pub async fn new_session(&self) -> AcpResult<acp::SessionId> {
        let conn = self.connection.as_ref()
            .ok_or(AcpError::NotInitialized)?;

        info!("Creating new session in: {:?}", self.config.working_dir);
        
        let mut request = acp::NewSessionRequest::new(self.config.working_dir.clone());
        
        // Add MCP servers if configured
        if !self.config.mcp_servers.is_empty() {
            info!("Configuring {} MCP server(s) for session", self.config.mcp_servers.len());
            let mcp_servers: Vec<acp::McpServer> = self.config.mcp_servers
                .iter()
                .map(|s| s.to_acp_server())
                .collect();
            request = request.mcp_servers(mcp_servers);
        }
        
        let response = conn.new_session(request).await?;

        let session_id = response.session_id.clone();
        *self.session_id.lock().await = Some(session_id.clone());
        
        info!("Session created: {:?}", session_id);
        Ok(session_id)
    }

    /// Send a prompt to the agent and wait for completion
    pub async fn prompt(&self, session_id: &acp::SessionId, prompt: String) -> AcpResult<()> {
        let conn = self.connection.as_ref()
            .ok_or(AcpError::NotInitialized)?;

        info!("Sending prompt to session: {:?}", session_id);
        debug!("Prompt content: {}", prompt);

        // Create timeout
        let timeout = Duration::from_secs(self.config.timeout);
        
        // Create prompt request
        let request = acp::PromptRequest::new(session_id.clone(), vec![prompt.into()]);
        
        // Send prompt with timeout
        let result = tokio::time::timeout(timeout, conn.prompt(request)).await;

        match result {
            Ok(Ok(_)) => {
                info!("Prompt completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Prompt failed: {}", e);
                Err(e.into())
            }
            Err(_) => {
                error!("Prompt timed out after {} seconds", self.config.timeout);
                Err(AcpError::Timeout(self.config.timeout))
            }
        }
    }

    /// Cancel the current operation
    pub async fn cancel(&self, session_id: &acp::SessionId) -> AcpResult<()> {
        let conn = self.connection.as_ref()
            .ok_or(AcpError::NotInitialized)?;

        info!("Sending cancel request for session: {:?}", session_id);
        
        let notification = acp::CancelNotification::new(session_id.clone());
        conn.cancel(notification).await?;

        Ok(())
    }

    /// Get the current session ID
    pub async fn current_session(&self) -> Option<acp::SessionId> {
        self.session_id.lock().await.clone()
    }

    /// Shutdown the agent
    /// 
    /// Implements a two-phase shutdown strategy:
    /// 1. Graceful shutdown: Cancel session + send /exit command
    /// 2. Force shutdown: SIGKILL if graceful shutdown fails
    /// 
    /// The /exit command is an OpenCode-specific slash command that allows
    /// the agent to exit gracefully. While not part of the ACP standard,
    /// it's supported by OpenCode and can be sent via session/prompt.
    pub async fn shutdown(&mut self) -> AcpResult<()> {
        info!("Shutting down agent");

        // Phase 1: Graceful shutdown attempt
        if let Some(session_id) = self.current_session().await {
            info!("Attempting graceful shutdown");
            
            // Step 1: Cancel any active session
            info!("Cancelling active session before shutdown");
            let _ = self.cancel(&session_id).await;
            
            // Give the agent a moment to process the cancellation
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Step 2: Try to send /exit command (OpenCode-specific)
            info!("Sending /exit command for graceful shutdown");
            let exit_result = self.send_exit_command(&session_id).await;
            
            match exit_result {
                Ok(()) => {
                    info!("/exit command sent successfully");
                    // Give the agent time to exit gracefully
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Err(e) => {
                    warn!("/exit command failed: {}, will force kill", e);
                }
            }
            
            // Step 3: Check if process exited gracefully
            if let Some(child) = self.child.lock().await.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        info!("Agent exited gracefully with status: {:?}", status);
                        self.child.lock().await.take(); // Remove the child
                        return Ok(());
                    }
                    Ok(None) => {
                        info!("Agent still running, proceeding to force kill");
                    }
                    Err(e) => {
                        warn!("Error checking agent status: {}", e);
                    }
                }
            }
        }

        // Phase 2: Force shutdown (fallback)
        if let Some(mut child) = self.child.lock().await.take() {
            info!("Force killing agent process");
            
            // Send SIGKILL
            match child.kill().await {
                Ok(()) => {
                    info!("Agent process killed successfully");
                }
                Err(e) => {
                    warn!("Error killing agent process: {}", e);
                }
            }
            
            // Wait for exit with timeout
            match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
                Ok(Ok(status)) => {
                    info!("Agent process exited with status: {:?}", status);
                }
                Ok(Err(e)) => {
                    warn!("Error waiting for agent exit: {}", e);
                }
                Err(_) => {
                    warn!("Agent process did not exit within timeout");
                    // Process should be dead by now, but log if it's not
                }
            }
        } else {
            info!("No child process to kill");
        }

        Ok(())
    }
    
    /// Send /exit command to the agent (OpenCode-specific)
    /// 
    /// This is a slash command that can be sent via session/prompt.
    /// It's not part of the ACP standard but is supported by OpenCode.
    async fn send_exit_command(&self, session_id: &acp::SessionId) -> AcpResult<()> {
        let conn = self.connection.as_ref()
            .ok_or(AcpError::NotInitialized)?;
        
        debug!("Sending /exit command to session: {:?}", session_id);
        
        // Send /exit as a prompt
        let request = acp::PromptRequest::new(session_id.clone(), vec!["/exit".into()]);
        
        // Use a short timeout for the exit command
        let timeout = Duration::from_millis(500);
        let result = tokio::time::timeout(timeout, conn.prompt(request)).await;
        
        match result {
            Ok(Ok(_)) => {
                debug!("/exit command completed");
                Ok(())
            }
            Ok(Err(e)) => {
                debug!("/exit command failed: {}", e);
                Err(e.into())
            }
            Err(_) => {
                debug!("/exit command timed out");
                Err(AcpError::Timeout(1)) // 1 second timeout
            }
        }
    }
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        // Best effort cleanup
        if let Some(mut child) = self.child.try_lock().ok().and_then(|mut guard| guard.take()) {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.command, "opencode");
        assert_eq!(config.args, vec!["acp"]);
        assert_eq!(config.timeout, 600);
    }

    #[test]
    fn test_agent_config_custom() {
        let config = AgentConfig {
            command: "claude".to_string(),
            args: vec!["--mode".to_string(), "acp".to_string()],
            env: vec![("API_KEY".to_string(), "test".to_string())],
            working_dir: PathBuf::from("/tmp"),
            timeout: 300,
            container_id: None,
            mcp_servers: Vec::new(),
        };
        
        assert_eq!(config.command, "claude");
        assert_eq!(config.args.len(), 2);
        assert_eq!(config.env.len(), 1);
        assert_eq!(config.timeout, 300);
    }

    #[tokio::test]
    async fn test_acp_client_creation() {
        let config = AgentConfig::default();
        let client = AcpClient::new(config);
        
        assert!(client.connection.is_none());
        assert!(client.current_session().await.is_none());
    }

    #[tokio::test]
    async fn test_acp_client_not_initialized_error() {
        let config = AgentConfig::default();
        let client = AcpClient::new(config);
        
        let result = client.new_session().await;
        assert!(matches!(result, Err(AcpError::NotInitialized)));
    }
}
