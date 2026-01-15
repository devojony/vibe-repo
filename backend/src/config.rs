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
            url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "sqlite:./data/gitautodev/db/gitautodev.db?mode=rwc".to_string()
            }),
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

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    /// Server configuration
    pub server: ServerConfig,
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
            config.url, "sqlite:./data/gitautodev/db/gitautodev.db?mode=rwc",
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

        let config = AppConfig::default();

        // Verify all defaults are set correctly
        assert_eq!(
            config.database.url,
            "sqlite:./data/gitautodev/db/gitautodev.db?mode=rwc"
        );
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
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
}
