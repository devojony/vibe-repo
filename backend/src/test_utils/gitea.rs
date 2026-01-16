//! Gitea test utilities
//!
//! Provides helpers for testing with the Gitea test instance.
//! Configuration is loaded from environment variables.

use std::time::Duration;

/// Gitea test configuration
///
/// Loaded from environment variables:
/// - `GITEA_TEST_URL`: Base URL of the test Gitea instance (default: https://gitea.devo.top:66)
/// - `GITEA_TEST_TOKEN`: Access token for the test Gitea instance (required for external tests)
#[derive(Debug, Clone)]
pub struct GiteaTestConfig {
    /// Base URL of the test Gitea instance
    pub base_url: String,
    /// Access token for authentication
    pub access_token: Option<String>,
}

impl GiteaTestConfig {
    /// Load configuration from environment variables
    ///
    /// Returns None if the required environment variables are not set.
    pub fn from_env() -> Option<Self> {
        let base_url =
            std::env::var("GITEA_TEST_URL").unwrap_or_else(|_| "https://gitea.devo.top:66".into());

        let access_token = std::env::var("GITEA_TEST_TOKEN").ok();

        Some(Self {
            base_url,
            access_token,
        })
    }

    /// Check if the configuration has valid credentials
    pub fn has_credentials(&self) -> bool {
        self.access_token
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /// Get the access token, panicking if not set
    ///
    /// Use this in tests that require authentication.
    pub fn token(&self) -> &str {
        self.access_token
            .as_ref()
            .expect("GITEA_TEST_TOKEN environment variable not set")
    }
}

/// Check if the Gitea test instance is available
///
/// Returns true if the instance responds to a health check within the timeout.
pub async fn is_gitea_available(config: &GiteaTestConfig, timeout: Duration) -> bool {
    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Try to reach the Gitea API
    let url = format!("{}/api/v1/version", config.base_url);
    client.get(&url).send().await.is_ok()
}

/// Wait for repositories to be synced with polling
///
/// Returns true if repositories are found within the timeout, false otherwise.
pub async fn wait_for_repositories(
    app: axum::Router,
    provider_id: i32,
    timeout: Duration,
    poll_interval: Duration,
) -> bool {
    use axum::body::Body;
    use axum::http::Request;
    use std::time::Instant;
    use tower::ServiceExt;

    let start = Instant::now();

    while start.elapsed() < timeout {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/repositories?provider_id={}", provider_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Use axum's body handling
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await;
        if let Ok(bytes) = body_bytes {
            if let Ok(repos) = serde_json::from_slice::<Vec<serde_json::Value>>(&bytes) {
                if !repos.is_empty() {
                    return true;
                }
            }
        }

        tokio::time::sleep(poll_interval).await;
    }

    false
}

/// Skip test if Gitea is not available
///
/// Use this macro at the start of tests that require the Gitea test instance.
#[macro_export]
macro_rules! skip_if_gitea_unavailable {
    ($config:expr) => {
        if !$crate::test_utils::gitea::is_gitea_available(
            $config,
            std::time::Duration::from_secs(5),
        )
        .await
        {
            eprintln!(
                "Skipping test: Gitea instance not available at {}",
                $config.base_url
            );
            return;
        }
    };
}

/// Skip test if Gitea credentials are not set
///
/// Use this macro at the start of tests that require authentication.
#[macro_export]
macro_rules! skip_if_no_gitea_credentials {
    ($config:expr) => {
        if !$config.has_credentials() {
            eprintln!("Skipping test: GITEA_TEST_TOKEN environment variable not set");
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // Tests for GiteaTestConfig
    // ============================================

    #[test]
    fn test_gitea_config_from_env_returns_some() {
        // Arrange & Act
        let config = GiteaTestConfig::from_env();

        // Assert
        assert!(config.is_some());
    }

    #[test]
    fn test_gitea_config_has_default_base_url() {
        // Arrange & Act
        let config = GiteaTestConfig::from_env().unwrap();

        // Assert - should have default URL if env var not set
        assert!(!config.base_url.is_empty());
        assert!(config.base_url.starts_with("http"));
    }

    #[test]
    fn test_gitea_config_has_credentials_returns_false_when_no_token() {
        // Arrange
        let config = GiteaTestConfig {
            base_url: "https://example.com".to_string(),
            access_token: None,
        };

        // Act & Assert
        assert!(!config.has_credentials());
    }

    #[test]
    fn test_gitea_config_has_credentials_returns_false_when_empty_token() {
        // Arrange
        let config = GiteaTestConfig {
            base_url: "https://example.com".to_string(),
            access_token: Some("".to_string()),
        };

        // Act & Assert
        assert!(!config.has_credentials());
    }

    #[test]
    fn test_gitea_config_has_credentials_returns_true_when_token_set() {
        // Arrange
        let config = GiteaTestConfig {
            base_url: "https://example.com".to_string(),
            access_token: Some("valid_token".to_string()),
        };

        // Act & Assert
        assert!(config.has_credentials());
    }

    #[test]
    fn test_gitea_config_token_returns_token_when_set() {
        // Arrange
        let config = GiteaTestConfig {
            base_url: "https://example.com".to_string(),
            access_token: Some("my_token".to_string()),
        };

        // Act & Assert
        assert_eq!(config.token(), "my_token");
    }

    #[test]
    #[should_panic(expected = "GITEA_TEST_TOKEN environment variable not set")]
    fn test_gitea_config_token_panics_when_not_set() {
        // Arrange
        let config = GiteaTestConfig {
            base_url: "https://example.com".to_string(),
            access_token: None,
        };

        // Act - should panic
        let _ = config.token();
    }
}
