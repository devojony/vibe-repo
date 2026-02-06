//! Configuration management module
//!
//! Loads configuration from environment variables with sensible defaults.

use serde::{Deserialize, Serialize};

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind host
    pub host: String,
    /// Server bind port
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("SERVER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000),
        }
    }
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Base domain for webhook URLs (e.g., "https://vibe-repo.example.com")
    pub domain: String,
    /// Secret key for signing webhook payloads
    pub secret_key: String,
    /// Bot username for mention detection in webhook events
    pub bot_username: String,
    /// Retry configuration
    pub retry: WebhookRetryConfig,
}

/// Issue polling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuePollingConfig {
    /// Whether issue polling is enabled
    pub enabled: bool,
    /// Polling interval in seconds
    pub interval_seconds: u64,
    /// Required labels for issues to be processed (comma-separated)
    pub required_labels: Option<Vec<String>>,
    /// Bot username to filter out bot-created issues
    pub bot_username: Option<String>,
    /// Maximum age of issues to process (in days)
    pub max_issue_age_days: Option<i64>,
    /// Maximum number of concurrent repository polls
    #[serde(default = "default_max_concurrent_polls")]
    pub max_concurrent_polls: usize,
    /// Maximum number of retry attempts for rate-limited requests
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_max_concurrent_polls() -> usize {
    10
}

fn default_max_retries() -> u32 {
    3
}

/// Webhook retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay in seconds
    pub initial_delay_secs: u64,
    /// Maximum retry delay in seconds
    pub max_delay_secs: u64,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Retry count threshold to enable polling fallback
    #[serde(default = "default_polling_fallback_threshold")]
    pub polling_fallback_threshold: u32,
}

fn default_polling_fallback_threshold() -> u32 {
    5
}

impl Default for WebhookRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: std::env::var("WEBHOOK_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            initial_delay_secs: std::env::var("WEBHOOK_INITIAL_DELAY_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60), // 1 minute
            max_delay_secs: std::env::var("WEBHOOK_MAX_DELAY_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600), // 1 hour
            backoff_multiplier: std::env::var("WEBHOOK_BACKOFF_MULTIPLIER")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2.0),
            polling_fallback_threshold: std::env::var("WEBHOOK_RETRY_POLLING_FALLBACK_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        }
    }
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            domain: std::env::var("WEBHOOK_DOMAIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            secret_key: std::env::var("WEBHOOK_SECRET_KEY").unwrap_or_default(),
            bot_username: std::env::var("WEBHOOK_BOT_USERNAME")
                .unwrap_or_else(|_| "vibe-repo-bot".to_string()),
            retry: WebhookRetryConfig::default(),
        }
    }
}

impl Default for IssuePollingConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("ISSUE_POLLING_ENABLED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
            interval_seconds: std::env::var("ISSUE_POLLING_INTERVAL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300), // 5 minutes
            required_labels: std::env::var("ISSUE_POLLING_REQUIRED_LABELS")
                .ok()
                .map(|s| {
                    s.split(',')
                        .map(|label| label.trim().to_string())
                        .filter(|label| !label.is_empty())
                        .collect()
                })
                .or_else(|| Some(vec!["vibe-auto".to_string()])),
            bot_username: std::env::var("ISSUE_POLLING_BOT_USERNAME")
                .ok()
                .or_else(|| Some("vibe-repo-bot".to_string())),
            max_issue_age_days: std::env::var("ISSUE_POLLING_MAX_ISSUE_AGE_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .or(Some(30)),
            max_concurrent_polls: std::env::var("ISSUE_POLLING_MAX_CONCURRENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            max_retries: std::env::var("ISSUE_POLLING_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
        }
    }
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Base directory for all workspaces
    pub base_dir: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            base_dir: std::env::var("WORKSPACE_BASE_DIR")
                .unwrap_or_else(|_| "./data/vibe-repo/workspaces".to_string()),
        }
    }
}

/// Git Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitProviderConfig {
    /// GitHub personal access token
    pub github_token: Option<String>,
    /// GitHub base URL (for GitHub Enterprise)
    pub github_base_url: Option<String>,
    /// Webhook secret for signature verification
    pub webhook_secret: Option<String>,
}

impl Default for GitProviderConfig {
    fn default() -> Self {
        Self {
            github_token: std::env::var("GITHUB_TOKEN").ok(),
            github_base_url: std::env::var("GITHUB_BASE_URL").ok(),
            webhook_secret: std::env::var("WEBHOOK_SECRET").ok(),
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Default command to run in agent container
    pub default_command: String,
    /// Default timeout in seconds
    pub default_timeout: u64,
    /// Default Docker image for agent containers
    pub default_docker_image: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            default_command: std::env::var("DEFAULT_AGENT_COMMAND")
                .unwrap_or_else(|_| "bash".to_string()),
            default_timeout: std::env::var("DEFAULT_AGENT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600), // 10 minutes
            default_docker_image: std::env::var("DEFAULT_DOCKER_IMAGE")
                .unwrap_or_else(|_| "ubuntu:22.04".to_string()),
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    /// Server configuration
    pub server: ServerConfig,
    /// Webhook configuration
    pub webhook: WebhookConfig,
    /// Issue polling configuration
    pub issue_polling: IssuePollingConfig,
    /// Workspace configuration
    pub workspace: WorkspaceConfig,
    /// Git provider configuration
    pub git_provider: GitProviderConfig,
    /// Agent configuration
    pub agent: AgentConfig,
}

impl AppConfig {
    /// Load configuration from environment variables with defaults
    pub fn from_env() -> Result<Self, ConfigError> {
        let config = Self::default();
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate DATABASE_URL is not empty
        if self.database.url.is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "DATABASE_URL".to_string(),
                message: "DATABASE_URL cannot be empty".to_string(),
            });
        }

        // Validate SERVER_PORT is in valid range
        if self.server.port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "SERVER_PORT".to_string(),
                message: "SERVER_PORT must be between 1 and 65535".to_string(),
            });
        }

        // Validate DATABASE_MAX_CONNECTIONS is positive
        if self.database.max_connections == 0 {
            return Err(ConfigError::InvalidValue {
                field: "DATABASE_MAX_CONNECTIONS".to_string(),
                message: "DATABASE_MAX_CONNECTIONS must be greater than 0".to_string(),
            });
        }

        // Validate WORKSPACE_BASE_DIR is not empty
        if self.workspace.base_dir.is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "WORKSPACE_BASE_DIR".to_string(),
                message: "WORKSPACE_BASE_DIR cannot be empty".to_string(),
            });
        }

        // Validate webhook secret key
        const WEAK_SECRETS: &[&str] = &[
            "change-this-in-production",
            "default-webhook-secret-change-in-production",
            "secret",
            "password",
            "test",
            "webhook-secret",
        ];

        if WEAK_SECRETS.contains(&self.webhook.secret_key.as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "WEBHOOK_SECRET_KEY".to_string(),
                message: format!(
                    "Weak or default webhook secret detected: '{}'. \
                     Please set WEBHOOK_SECRET_KEY to a strong random value. \
                     Generate one with: openssl rand -hex 32",
                    self.webhook.secret_key
                ),
            });
        }

        // Warn if webhook secret is empty or too short
        if self.webhook.secret_key.is_empty() {
            tracing::warn!(
                "SECURITY WARNING: WEBHOOK_SECRET_KEY is not set. \
                 Webhook signature verification will be disabled. \
                 This is not recommended for production. \
                 Generate a secure key with: openssl rand -hex 32"
            );
        } else if self.webhook.secret_key.len() < 32 {
            tracing::warn!(
                "SECURITY WARNING: WEBHOOK_SECRET_KEY is too short ({} characters). \
                 Recommended minimum is 32 characters. \
                 Generate a secure key with: openssl rand -hex 32",
                self.webhook.secret_key.len()
            );
        }

        // Validate issue polling configuration
        if self.issue_polling.enabled {
            // interval_seconds should be at least 60 (1 minute)
            if self.issue_polling.interval_seconds < 60 {
                return Err(ConfigError::InvalidValue {
                    field: "ISSUE_POLLING_INTERVAL_SECONDS".to_string(),
                    message: "ISSUE_POLLING_INTERVAL_SECONDS must be at least 60 seconds"
                        .to_string(),
                });
            }

            // max_issue_age_days should be positive if set
            if let Some(max_age) = self.issue_polling.max_issue_age_days {
                if max_age <= 0 {
                    return Err(ConfigError::InvalidValue {
                        field: "ISSUE_POLLING_MAX_ISSUE_AGE_DAYS".to_string(),
                        message: "ISSUE_POLLING_MAX_ISSUE_AGE_DAYS must be greater than 0"
                            .to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

/// Configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    #[error("Missing required configuration: {0}")]
    MissingRequired(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // Task 2.1: Tests for configuration defaults
    // Requirements: 2.1, 2.2, 2.3
    // ============================================

    #[test]
    fn test_database_url_default_value() {
        // Clear environment variable to test default
        std::env::remove_var("DATABASE_URL");

        let config = DatabaseConfig::default();

        assert_eq!(
            config.url, "sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc",
            "DATABASE_URL should default to SQLite development database"
        );
    }

    #[test]
    fn test_server_host_default_value() {
        // Clear environment variable to test default
        std::env::remove_var("SERVER_HOST");

        let config = ServerConfig::default();

        assert_eq!(
            config.host, "0.0.0.0",
            "SERVER_HOST should default to 0.0.0.0"
        );
    }

    #[test]
    fn test_server_port_default_value() {
        // Clear environment variable to test default
        std::env::remove_var("SERVER_PORT");

        let config = ServerConfig::default();

        assert_eq!(config.port, 3000, "SERVER_PORT should default to 3000");
    }

    #[test]
    fn test_database_max_connections_default_value() {
        // Clear environment variable to test default
        std::env::remove_var("DATABASE_MAX_CONNECTIONS");

        let config = DatabaseConfig::default();

        assert_eq!(
            config.max_connections, 10,
            "DATABASE_MAX_CONNECTIONS should default to 10"
        );
    }

    #[test]
    fn test_app_config_default_contains_all_defaults() {
        // Clear all environment variables
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("DATABASE_MAX_CONNECTIONS");
        std::env::remove_var("SERVER_HOST");
        std::env::remove_var("SERVER_PORT");
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("DEFAULT_AGENT_COMMAND");
        std::env::remove_var("DEFAULT_AGENT_TIMEOUT");

        let config = AppConfig::default();

        // Verify all defaults are set correctly
        assert_eq!(
            config.database.url,
            "sqlite:./data/vibe-repo/db/vibe-repo.db?mode=rwc"
        );
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.agent.default_command, "bash");
        assert_eq!(config.agent.default_timeout, 600);
        assert_eq!(config.agent.default_docker_image, "ubuntu:22.04");
    }

    // ============================================
    // Webhook Configuration Tests
    // ============================================

    #[test]
    fn test_webhook_config_bot_username_default() {
        // Clear environment variable to test default
        std::env::remove_var("WEBHOOK_BOT_USERNAME");

        let config = WebhookConfig::default();

        assert_eq!(
            config.bot_username, "vibe-repo-bot",
            "WEBHOOK_BOT_USERNAME should default to 'vibe-repo-bot'"
        );
    }

    // ============================================
    // Task 2.3: Property test for configuration defaults validity
    // Property 1: Configuration defaults are valid
    // Validates: Requirements 2.1, 2.2, 2.3
    // ============================================
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Feature: backend-init, Property 1: Configuration defaults are valid
            /// For any AppConfig created with default(), all fields SHALL have valid values
            #[test]
            fn prop_config_defaults_are_valid(_seed in any::<u64>()) {
                // Clear environment variables to ensure we test defaults
                std::env::remove_var("DATABASE_URL");
                std::env::remove_var("DATABASE_MAX_CONNECTIONS");
                std::env::remove_var("SERVER_HOST");
                std::env::remove_var("SERVER_PORT");

                let config = AppConfig::default();

                // DATABASE_URL must be a non-empty string
                prop_assert!(
                    !config.database.url.is_empty(),
                    "DATABASE_URL must be non-empty"
                );

                // SERVER_HOST must be a non-empty string (valid IP or hostname)
                prop_assert!(
                    !config.server.host.is_empty(),
                    "SERVER_HOST must be non-empty"
                );

                // SERVER_PORT must be in range 1-65535 (u16 max is 65535, so only check >= 1)
                prop_assert!(
                    config.server.port >= 1,
                    "SERVER_PORT must be between 1 and 65535, got {}",
                    config.server.port
                );

                // DATABASE_MAX_CONNECTIONS must be greater than 0
                prop_assert!(
                    config.database.max_connections > 0,
                    "DATABASE_MAX_CONNECTIONS must be greater than 0, got {}",
                    config.database.max_connections
                );

                // Validation should pass for default config
                prop_assert!(
                    config.validate().is_ok(),
                    "Default configuration should pass validation"
                );
            }

            // ============================================
            // Task 2.6: Property test for configuration validation correctness
            // Property 2: Configuration validation correctness
            // Validates: Requirements 2.4, 2.5
            // ============================================

            /// Feature: backend-init, Property 2: Configuration validation correctness
            /// For any valid configuration, validation SHALL accept it
            #[test]
            fn prop_valid_config_is_accepted(
                url in "[a-z]+://[a-z0-9./]+".prop_filter("non-empty URL", |s| !s.is_empty()),
                host in "[a-z0-9.]+".prop_filter("non-empty host", |s| !s.is_empty()),
                port in 1u16..=65535u16,
                max_connections in 1u32..=1000u32,
            ) {
                let config = AppConfig {
                    database: DatabaseConfig {
                        url,
                        max_connections,
                    },
                    server: ServerConfig {

                        host,
                        port,
                    },
                    webhook: WebhookConfig::default(),
                    issue_polling: IssuePollingConfig::default(),
                    workspace: WorkspaceConfig::default(),
                    git_provider: GitProviderConfig::default(),
                    agent: AgentConfig::default(),
                };

                let result = config.validate();
                prop_assert!(
                    result.is_ok(),
                    "Valid configuration should be accepted: {:?}",
                    config
                );
            }

            /// Feature: backend-init, Property 2: Configuration validation correctness
            /// For any configuration with empty DATABASE_URL, validation SHALL reject it
            #[test]
            fn prop_empty_database_url_is_rejected(
                host in "[a-z0-9.]+".prop_filter("non-empty host", |s| !s.is_empty()),
                port in 1u16..=65535u16,
                max_connections in 1u32..=1000u32,
            ) {
                let config = AppConfig {
                    database: DatabaseConfig {
                        url: "".to_string(),
                        max_connections,
                    },
                    server: ServerConfig {

                        host,
                        port,
                    },
                    webhook: WebhookConfig::default(),
                    issue_polling: IssuePollingConfig::default(),
                    workspace: WorkspaceConfig::default(),
                    git_provider: GitProviderConfig::default(),
                    agent: AgentConfig::default(),
                };

                let result = config.validate();
                prop_assert!(
                    result.is_err(),
                    "Empty DATABASE_URL should be rejected"
                );

                // Error should be descriptive
                if let Err(ConfigError::InvalidValue { field, message }) = result {
                    prop_assert_eq!(field, "DATABASE_URL");
                    prop_assert!(!message.is_empty(), "Error message should be descriptive");
                }
            }

            /// Feature: backend-init, Property 2: Configuration validation correctness
            /// For any configuration with zero SERVER_PORT, validation SHALL reject it
            #[test]
            fn prop_zero_server_port_is_rejected(
                url in "[a-z]+://[a-z0-9./]+".prop_filter("non-empty URL", |s| !s.is_empty()),
                host in "[a-z0-9.]+".prop_filter("non-empty host", |s| !s.is_empty()),
                max_connections in 1u32..=1000u32,
            ) {
                let config = AppConfig {
                    database: DatabaseConfig {
                        url,
                        max_connections,
                    },
                    server: ServerConfig {

                        host,
                        port: 0,
                    },
                    webhook: WebhookConfig::default(),
                    issue_polling: IssuePollingConfig::default(),
                    workspace: WorkspaceConfig::default(),
                    git_provider: GitProviderConfig::default(),
                    agent: AgentConfig::default(),
                };

                let result = config.validate();
                prop_assert!(
                    result.is_err(),
                    "Zero SERVER_PORT should be rejected"
                );

                // Error should be descriptive
                if let Err(ConfigError::InvalidValue { field, message }) = result {
                    prop_assert_eq!(field, "SERVER_PORT");
                    prop_assert!(!message.is_empty(), "Error message should be descriptive");
                }
            }

            /// Feature: backend-init, Property 2: Configuration validation correctness
            /// For any configuration with zero DATABASE_MAX_CONNECTIONS, validation SHALL reject it
            #[test]
            fn prop_zero_max_connections_is_rejected(
                url in "[a-z]+://[a-z0-9./]+".prop_filter("non-empty URL", |s| !s.is_empty()),
                host in "[a-z0-9.]+".prop_filter("non-empty host", |s| !s.is_empty()),
                port in 1u16..=65535u16,
            ) {
                let config = AppConfig {
                    database: DatabaseConfig {
                        url,
                        max_connections: 0,
                    },
                    server: ServerConfig {

                        host,
                        port,
                    },
                    webhook: WebhookConfig::default(),
                    issue_polling: IssuePollingConfig::default(),
                    workspace: WorkspaceConfig::default(),
                    git_provider: GitProviderConfig::default(),
                    agent: AgentConfig::default(),
                };

                let result = config.validate();
                prop_assert!(
                    result.is_err(),
                    "Zero DATABASE_MAX_CONNECTIONS should be rejected"
                );

                // Error should be descriptive
                if let Err(ConfigError::InvalidValue { field, message }) = result {
                    prop_assert_eq!(field, "DATABASE_MAX_CONNECTIONS");
                    prop_assert!(!message.is_empty(), "Error message should be descriptive");
                }
            }
        }
    }

    // ============================================
    // Task 2.4: Tests for configuration validation
    // Requirements: 2.4, 2.5
    // ============================================

    #[test]
    fn test_valid_configuration_is_accepted() {
        let config = AppConfig {
            database: DatabaseConfig {
                url: "sqlite:./test.db".to_string(),
                max_connections: 5,
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig::default(),
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        assert!(
            config.validate().is_ok(),
            "Valid configuration should be accepted"
        );
    }

    #[test]
    fn test_empty_database_url_returns_error() {
        let config = AppConfig {
            database: DatabaseConfig {
                url: "".to_string(),
                max_connections: 10,
            },
            server: ServerConfig::default(),
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig::default(),
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        let result = config.validate();
        assert!(result.is_err(), "Empty DATABASE_URL should return error");

        let err = result.unwrap_err();
        match err {
            ConfigError::InvalidValue { field, message } => {
                assert_eq!(field, "DATABASE_URL");
                assert!(
                    message.contains("empty"),
                    "Error message should mention empty"
                );
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_zero_server_port_returns_error() {
        let config = AppConfig {
            database: DatabaseConfig::default(),
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 0,
            },
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig::default(),
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        let result = config.validate();
        assert!(result.is_err(), "Zero SERVER_PORT should return error");

        let err = result.unwrap_err();
        match err {
            ConfigError::InvalidValue { field, message } => {
                assert_eq!(field, "SERVER_PORT");
                assert!(
                    message.contains("1") && message.contains("65535"),
                    "Error message should mention valid range"
                );
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_zero_max_connections_returns_error() {
        let config = AppConfig {
            database: DatabaseConfig {
                url: "sqlite:./test.db".to_string(),
                max_connections: 0,
            },
            server: ServerConfig::default(),
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig::default(),
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        let result = config.validate();
        assert!(
            result.is_err(),
            "Zero DATABASE_MAX_CONNECTIONS should return error"
        );

        let err = result.unwrap_err();
        match err {
            ConfigError::InvalidValue { field, message } => {
                assert_eq!(field, "DATABASE_MAX_CONNECTIONS");
                assert!(
                    message.contains("greater than 0"),
                    "Error message should mention greater than 0"
                );
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_from_env_validates_configuration() {
        // Clear environment variables to use defaults
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("DATABASE_MAX_CONNECTIONS");
        std::env::remove_var("SERVER_HOST");
        std::env::remove_var("SERVER_PORT");

        let result = AppConfig::from_env();
        assert!(result.is_ok(), "from_env() with defaults should succeed");
    }

    // ============================================
    // Issue Polling Configuration Tests
    // ============================================

    #[test]
    fn test_issue_polling_default_values() {
        // Clear environment variables to test defaults
        std::env::remove_var("ISSUE_POLLING_ENABLED");
        std::env::remove_var("ISSUE_POLLING_INTERVAL_SECONDS");
        std::env::remove_var("ISSUE_POLLING_REQUIRED_LABELS");
        std::env::remove_var("ISSUE_POLLING_BOT_USERNAME");
        std::env::remove_var("ISSUE_POLLING_MAX_ISSUE_AGE_DAYS");
        std::env::remove_var("ISSUE_POLLING_MAX_CONCURRENT");
        std::env::remove_var("ISSUE_POLLING_MAX_RETRIES");

        let config = IssuePollingConfig::default();

        assert!(!config.enabled, "Polling should be disabled by default");
        assert_eq!(
            config.interval_seconds, 300,
            "Default interval should be 300 seconds (5 minutes)"
        );
        assert_eq!(
            config.required_labels,
            Some(vec!["vibe-auto".to_string()]),
            "Default required labels should be ['vibe-auto']"
        );
        assert_eq!(
            config.bot_username,
            Some("vibe-repo-bot".to_string()),
            "Default bot username should be 'vibe-repo-bot'"
        );
        assert_eq!(
            config.max_issue_age_days,
            Some(30),
            "Default max issue age should be 30 days"
        );
        assert_eq!(
            config.max_concurrent_polls, 10,
            "Default max concurrent polls should be 10"
        );
        assert_eq!(config.max_retries, 3, "Default max retries should be 3");
    }

    #[test]
    fn test_issue_polling_loads_from_env() {
        // Set environment variables
        std::env::set_var("ISSUE_POLLING_ENABLED", "true");
        std::env::set_var("ISSUE_POLLING_INTERVAL_SECONDS", "600");
        std::env::set_var("ISSUE_POLLING_REQUIRED_LABELS", "bug,feature");
        std::env::set_var("ISSUE_POLLING_BOT_USERNAME", "my-bot");
        std::env::set_var("ISSUE_POLLING_MAX_ISSUE_AGE_DAYS", "60");
        std::env::set_var("ISSUE_POLLING_MAX_CONCURRENT", "20");
        std::env::set_var("ISSUE_POLLING_MAX_RETRIES", "5");

        let config = IssuePollingConfig::default();

        assert!(config.enabled, "Polling should be enabled");
        assert_eq!(
            config.interval_seconds, 600,
            "Interval should be 600 seconds"
        );
        assert_eq!(
            config.required_labels,
            Some(vec!["bug".to_string(), "feature".to_string()]),
            "Required labels should be parsed from comma-separated string"
        );
        assert_eq!(
            config.bot_username,
            Some("my-bot".to_string()),
            "Bot username should be loaded from env"
        );
        assert_eq!(
            config.max_issue_age_days,
            Some(60),
            "Max issue age should be 60 days"
        );
        assert_eq!(
            config.max_concurrent_polls, 20,
            "Max concurrent polls should be 20"
        );
        assert_eq!(config.max_retries, 5, "Max retries should be 5");

        // Clean up
        std::env::remove_var("ISSUE_POLLING_ENABLED");
        std::env::remove_var("ISSUE_POLLING_INTERVAL_SECONDS");
        std::env::remove_var("ISSUE_POLLING_REQUIRED_LABELS");
        std::env::remove_var("ISSUE_POLLING_BOT_USERNAME");
        std::env::remove_var("ISSUE_POLLING_MAX_ISSUE_AGE_DAYS");
        std::env::remove_var("ISSUE_POLLING_MAX_CONCURRENT");
        std::env::remove_var("ISSUE_POLLING_MAX_RETRIES");
    }

    #[test]
    fn test_issue_polling_validation_rejects_low_interval() {
        let config = AppConfig {
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig {
                enabled: true,
                interval_seconds: 30, // Less than 60
                required_labels: None,
                bot_username: None,
                max_issue_age_days: Some(30),
                max_concurrent_polls: 10,
                max_retries: 3,
            },
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        let result = config.validate();
        assert!(
            result.is_err(),
            "Validation should reject interval_seconds < 60"
        );

        let err = result.unwrap_err();
        match err {
            ConfigError::InvalidValue { field, message } => {
                assert_eq!(field, "ISSUE_POLLING_INTERVAL_SECONDS");
                assert!(
                    message.contains("60"),
                    "Error message should mention minimum of 60 seconds"
                );
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_issue_polling_validation_rejects_negative_max_age() {
        let config = AppConfig {
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig {
                enabled: true,
                interval_seconds: 300,
                required_labels: None,
                bot_username: None,
                max_issue_age_days: Some(-1), // Negative value
                max_concurrent_polls: 10,
                max_retries: 3,
            },
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        let result = config.validate();
        assert!(
            result.is_err(),
            "Validation should reject negative max_issue_age_days"
        );

        let err = result.unwrap_err();
        match err {
            ConfigError::InvalidValue { field, message } => {
                assert_eq!(field, "ISSUE_POLLING_MAX_ISSUE_AGE_DAYS");
                assert!(
                    message.contains("greater than 0"),
                    "Error message should mention greater than 0"
                );
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_issue_polling_validation_accepts_valid_config() {
        let config = AppConfig {
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig {
                enabled: true,
                interval_seconds: 300,
                required_labels: Some(vec!["vibe-auto".to_string()]),
                bot_username: Some("bot".to_string()),
                max_issue_age_days: Some(30),
                max_concurrent_polls: 10,
                max_retries: 3,
            },
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        let result = config.validate();
        assert!(
            result.is_ok(),
            "Validation should accept valid issue polling config"
        );
    }

    #[test]
    fn test_issue_polling_validation_skipped_when_disabled() {
        let config = AppConfig {
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig {
                enabled: false,       // Disabled
                interval_seconds: 30, // Invalid, but should be ignored
                required_labels: None,
                bot_username: None,
                max_issue_age_days: Some(-1), // Invalid, but should be ignored
                max_concurrent_polls: 10,
                max_retries: 3,
            },
            workspace: WorkspaceConfig::default(),
            git_provider: GitProviderConfig::default(),
            agent: AgentConfig::default(),
        };

        let result = config.validate();
        assert!(
            result.is_ok(),
            "Validation should skip issue polling checks when disabled"
        );
    }
}
