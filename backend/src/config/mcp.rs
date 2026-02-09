//! MCP (Model Context Protocol) server configuration module
//!
//! This module provides support for loading MCP server configurations from JSON files.
//! Configuration can be specified at two levels:
//! 1. Repository level: `{workspace_dir}/.vibe-repo/mcp-servers.json` (priority)
//! 2. Global level: `./data/vibe-repo/config/mcp-servers.json` (fallback)
//!
//! # Configuration Format
//!
//! ```json
//! {
//!   "version": "1.0",
//!   "servers": [
//!     {
//!       "name": "github",
//!       "command": "npx",
//!       "args": ["-y", "@modelcontextprotocol/server-github"],
//!       "env": [
//!         {
//!           "name": "GITHUB_TOKEN",
//!           "value": "${GITHUB_TOKEN}"
//!         }
//!       ],
//!       "disabled": false
//!     }
//!   ],
//!   "metadata": {
//!     "description": "MCP servers configuration",
//!     "updated_at": "2026-02-09T10:00:00Z"
//!   }
//! }
//! ```

use crate::error::{Result, VibeRepoError};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// MCP server configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServersConfig {
    /// Configuration version
    pub version: String,
    /// List of MCP servers
    pub servers: Vec<McpServerConfig>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: Option<McpConfigMetadata>,
}

impl Default for McpServersConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            servers: Vec::new(),
            metadata: None,
        }
    }
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name (must be unique)
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default)]
    pub env: Vec<McpEnvVar>,
    /// Whether this server is disabled
    #[serde(default)]
    pub disabled: bool,
}

impl McpServerConfig {
    /// Convert to ACP SDK McpServer type
    pub fn to_acp_server(&self) -> agent_client_protocol::McpServer {
        use agent_client_protocol::{EnvVariable, McpServer, McpServerStdio};
        use std::path::PathBuf;

        let stdio = McpServerStdio::new(self.name.clone(), PathBuf::from(&self.command))
            .args(self.args.clone())
            .env(
                self.env
                    .iter()
                    .map(|e| EnvVariable::new(e.name.clone(), e.value.clone()))
                    .collect(),
            );

        McpServer::Stdio(stdio)
    }
}

/// Environment variable for MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEnvVar {
    /// Variable name
    pub name: String,
    /// Variable value (supports ${VAR} placeholders)
    pub value: String,
}

/// Metadata for MCP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigMetadata {
    /// Configuration description
    #[serde(default)]
    pub description: Option<String>,
    /// Last update timestamp
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// MCP configuration loader
pub struct McpConfigLoader {
    /// Global configuration directory
    global_config_dir: PathBuf,
}

impl McpConfigLoader {
    /// Create a new MCP configuration loader
    ///
    /// # Arguments
    ///
    /// * `global_config_dir` - Path to global configuration directory
    pub fn new(global_config_dir: PathBuf) -> Self {
        Self { global_config_dir }
    }

    /// Load MCP configuration for a workspace
    ///
    /// Priority order:
    /// 1. Repository level: `{workspace_dir}/.vibe-repo/mcp-servers.json`
    /// 2. Global level: `{global_config_dir}/mcp-servers.json`
    /// 3. Default: Empty configuration
    ///
    /// # Arguments
    ///
    /// * `workspace_dir` - Path to workspace directory
    ///
    /// # Returns
    ///
    /// Returns a validated and processed MCP configuration
    pub fn load_for_workspace(&self, workspace_dir: &Path) -> Result<McpServersConfig> {
        // Try repository-level configuration first
        let repo_config_path = workspace_dir.join(".vibe-repo").join("mcp-servers.json");
        if repo_config_path.exists() {
            info!(
                "Loading repository-level MCP configuration from: {}",
                repo_config_path.display()
            );
            match self.load_from_file(&repo_config_path) {
                Ok(config) => {
                    let processed = self.validate_and_process(config)?;
                    info!(
                        "Loaded {} MCP server(s) from repository configuration",
                        processed.servers.len()
                    );
                    return Ok(processed);
                }
                Err(e) => {
                    warn!("Failed to load repository-level MCP configuration: {}", e);
                }
            }
        }

        // Try global configuration
        let global_config_path = self.global_config_dir.join("mcp-servers.json");
        if global_config_path.exists() {
            info!(
                "Loading global MCP configuration from: {}",
                global_config_path.display()
            );
            match self.load_from_file(&global_config_path) {
                Ok(config) => {
                    let processed = self.validate_and_process(config)?;
                    info!(
                        "Loaded {} MCP server(s) from global configuration",
                        processed.servers.len()
                    );
                    return Ok(processed);
                }
                Err(e) => {
                    warn!("Failed to load global MCP configuration: {}", e);
                }
            }
        }

        // Return default empty configuration
        debug!("No MCP configuration found, using default empty configuration");
        Ok(McpServersConfig::default())
    }

    /// Load configuration from a JSON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to configuration file
    fn load_from_file(&self, path: &Path) -> Result<McpServersConfig> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            VibeRepoError::Config(format!(
                "Failed to read MCP configuration from {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: McpServersConfig = serde_json::from_str(&content).map_err(|e| {
            VibeRepoError::Config(format!(
                "Failed to parse MCP configuration from {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }

    /// Validate and process configuration
    ///
    /// This method:
    /// - Checks for duplicate server names
    /// - Filters out disabled servers
    /// - Substitutes environment variable placeholders
    ///
    /// # Arguments
    ///
    /// * `config` - Raw configuration to validate
    fn validate_and_process(&self, mut config: McpServersConfig) -> Result<McpServersConfig> {
        // Check for duplicate names
        let mut seen_names = HashSet::new();
        for server in &config.servers {
            if !seen_names.insert(server.name.clone()) {
                return Err(VibeRepoError::Config(format!(
                    "Duplicate MCP server name: {}",
                    server.name
                )));
            }
        }

        // Filter out disabled servers
        let original_count = config.servers.len();
        config.servers.retain(|s| !s.disabled);
        let disabled_count = original_count - config.servers.len();
        if disabled_count > 0 {
            debug!("Filtered out {} disabled MCP server(s)", disabled_count);
        }

        // Substitute environment variables
        for server in &mut config.servers {
            for env_var in &mut server.env {
                env_var.value = self.substitute_env_vars(&env_var.value)?;
            }
        }

        Ok(config)
    }

    /// Substitute environment variable placeholders in a string
    ///
    /// Replaces `${VAR_NAME}` with the value of the environment variable.
    ///
    /// # Arguments
    ///
    /// * `value` - String containing placeholders
    ///
    /// # Returns
    ///
    /// Returns the string with all placeholders replaced
    fn substitute_env_vars(&self, value: &str) -> Result<String> {
        let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
        let mut result = value.to_string();
        let mut missing_vars = Vec::new();

        for cap in re.captures_iter(value) {
            let var_name = &cap[1];
            match std::env::var(var_name) {
                Ok(var_value) => {
                    let placeholder = format!("${{{}}}", var_name);
                    result = result.replace(&placeholder, &var_value);
                }
                Err(_) => {
                    missing_vars.push(var_name.to_string());
                }
            }
        }

        if !missing_vars.is_empty() {
            return Err(VibeRepoError::Config(format!(
                "Missing environment variable(s) for MCP configuration: {}",
                missing_vars.join(", ")
            )));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_substitute_env_vars() {
        // Set test environment variable
        env::set_var("TEST_TOKEN", "secret123");

        let loader = McpConfigLoader::new(PathBuf::from("."));
        let result = loader.substitute_env_vars("token=${TEST_TOKEN}").unwrap();
        assert_eq!(result, "token=secret123");

        // Clean up
        env::remove_var("TEST_TOKEN");
    }

    #[test]
    fn test_substitute_env_vars_multiple() {
        // Set test environment variables
        env::set_var("TEST_VAR1", "value1");
        env::set_var("TEST_VAR2", "value2");

        let loader = McpConfigLoader::new(PathBuf::from("."));
        let result = loader
            .substitute_env_vars("${TEST_VAR1}:${TEST_VAR2}")
            .unwrap();
        assert_eq!(result, "value1:value2");

        // Clean up
        env::remove_var("TEST_VAR1");
        env::remove_var("TEST_VAR2");
    }

    #[test]
    fn test_substitute_env_vars_missing() {
        let loader = McpConfigLoader::new(PathBuf::from("."));
        let result = loader.substitute_env_vars("token=${MISSING_VAR}");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing environment variable"));
    }

    #[test]
    fn test_substitute_env_vars_no_placeholders() {
        let loader = McpConfigLoader::new(PathBuf::from("."));
        let result = loader.substitute_env_vars("plain text").unwrap();
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_validate_duplicate_names() {
        let loader = McpConfigLoader::new(PathBuf::from("."));
        let config = McpServersConfig {
            version: "1.0".to_string(),
            servers: vec![
                McpServerConfig {
                    name: "server1".to_string(),
                    command: "cmd1".to_string(),
                    args: vec![],
                    env: vec![],
                    disabled: false,
                },
                McpServerConfig {
                    name: "server1".to_string(),
                    command: "cmd2".to_string(),
                    args: vec![],
                    env: vec![],
                    disabled: false,
                },
            ],
            metadata: None,
        };

        let result = loader.validate_and_process(config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate"));
    }

    #[test]
    fn test_filter_disabled_servers() {
        let loader = McpConfigLoader::new(PathBuf::from("."));
        let config = McpServersConfig {
            version: "1.0".to_string(),
            servers: vec![
                McpServerConfig {
                    name: "enabled".to_string(),
                    command: "cmd1".to_string(),
                    args: vec![],
                    env: vec![],
                    disabled: false,
                },
                McpServerConfig {
                    name: "disabled".to_string(),
                    command: "cmd2".to_string(),
                    args: vec![],
                    env: vec![],
                    disabled: true,
                },
            ],
            metadata: None,
        };

        let result = loader.validate_and_process(config).unwrap();
        assert_eq!(result.servers.len(), 1);
        assert_eq!(result.servers[0].name, "enabled");
    }

    #[test]
    fn test_load_priority() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");
        let global_config_dir = temp_dir.path().join("global");

        std::fs::create_dir_all(&workspace_dir).unwrap();
        std::fs::create_dir_all(&global_config_dir).unwrap();

        // Create global config
        let global_config = McpServersConfig {
            version: "1.0".to_string(),
            servers: vec![McpServerConfig {
                name: "global".to_string(),
                command: "global-cmd".to_string(),
                args: vec![],
                env: vec![],
                disabled: false,
            }],
            metadata: None,
        };
        std::fs::write(
            global_config_dir.join("mcp-servers.json"),
            serde_json::to_string(&global_config).unwrap(),
        )
        .unwrap();

        // Create repository config
        let repo_config_dir = workspace_dir.join(".vibe-repo");
        std::fs::create_dir_all(&repo_config_dir).unwrap();
        let repo_config = McpServersConfig {
            version: "1.0".to_string(),
            servers: vec![McpServerConfig {
                name: "repo".to_string(),
                command: "repo-cmd".to_string(),
                args: vec![],
                env: vec![],
                disabled: false,
            }],
            metadata: None,
        };
        std::fs::write(
            repo_config_dir.join("mcp-servers.json"),
            serde_json::to_string(&repo_config).unwrap(),
        )
        .unwrap();

        // Test: Repository config should take priority
        let loader = McpConfigLoader::new(global_config_dir.clone());
        let config = loader.load_for_workspace(&workspace_dir).unwrap();
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "repo");

        // Test: Global config should be used when repository config doesn't exist
        std::fs::remove_file(repo_config_dir.join("mcp-servers.json")).unwrap();
        let config = loader.load_for_workspace(&workspace_dir).unwrap();
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "global");
    }

    #[test]
    fn test_parse_config_file() {
        let json = r#"{
            "version": "1.0",
            "servers": [
                {
                    "name": "github",
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-github"],
                    "env": [
                        {
                            "name": "GITHUB_TOKEN",
                            "value": "test-token"
                        }
                    ],
                    "disabled": false
                }
            ],
            "metadata": {
                "description": "Test configuration",
                "updated_at": "2026-02-09T10:00:00Z"
            }
        }"#;

        let config: McpServersConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "github");
        assert_eq!(config.servers[0].command, "npx");
        assert_eq!(config.servers[0].args.len(), 2);
        assert_eq!(config.servers[0].env.len(), 1);
        assert_eq!(config.servers[0].env[0].name, "GITHUB_TOKEN");
        assert!(config.metadata.is_some());
    }

    #[test]
    fn test_to_acp_server() {
        use agent_client_protocol::McpServer;

        let server = McpServerConfig {
            name: "test".to_string(),
            command: "test-cmd".to_string(),
            args: vec!["arg1".to_string(), "arg2".to_string()],
            env: vec![McpEnvVar {
                name: "KEY".to_string(),
                value: "value".to_string(),
            }],
            disabled: false,
        };

        let acp_server = server.to_acp_server();
        match acp_server {
            McpServer::Stdio(stdio) => {
                assert_eq!(stdio.name, "test");
                assert_eq!(stdio.command.to_str().unwrap(), "test-cmd");
                assert_eq!(stdio.args, vec!["arg1", "arg2"]);
                assert_eq!(stdio.env.len(), 1);
                assert_eq!(stdio.env[0].name, "KEY");
                assert_eq!(stdio.env[0].value, "value");
            }
            _ => panic!("Expected McpServer::Stdio variant"),
        }
    }

    #[test]
    fn test_default_config() {
        let config = McpServersConfig::default();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.servers.len(), 0);
        assert!(config.metadata.is_none());
    }
}
