//! Provider token validation
//!
//! Logic for validating provider tokens against Git provider APIs.

use crate::error::{VibeRepoError, Result};
use crate::git_provider::{GitClientFactory, GitProvider, GitUser};

/// Validate a provider token
///
/// Makes a test request to the provider API to verify the token is valid.
pub async fn validate_token(
    base_url: &str,
    access_token: &str,
    provider_type: &str,
) -> Result<(bool, String, Option<GitUser>)> {
    // Create GitProvider client
    let git_client = GitClientFactory::create(provider_type, base_url, access_token)
        .map_err(|e| VibeRepoError::Validation(format!("Failed to create git client: {}", e)))?;

    // Validate token using GitProvider
    match git_client.validate_token().await {
        Ok((valid, user_opt)) => {
            if valid {
                let message = if let Some(ref user) = user_opt {
                    format!("Token is valid for user: {}", user.username)
                } else {
                    "Token is valid".to_string()
                };
                Ok((true, message, user_opt))
            } else {
                Ok((
                    false,
                    "Invalid token or insufficient permissions".to_string(),
                    None,
                ))
            }
        }
        Err(e) => {
            // Check if it's an unauthorized error
            if e.to_string().contains("Unauthorized") {
                Ok((
                    false,
                    "Invalid token or insufficient permissions".to_string(),
                    None,
                ))
            } else {
                Err(VibeRepoError::Internal(format!(
                    "Failed to validate token: {}",
                    e
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test validation with unsupported provider type
    #[test]
    fn test_validate_token_unsupported_type() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(validate_token(
            "https://example.com",
            "token",
            "unsupported",
        ));

        assert!(result.is_err());
        match result {
            Err(VibeRepoError::Validation(msg)) => {
                assert!(msg.contains("Failed to create git client"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    // NOTE: Integration tests with real Gitea instance are in the integration test suite
    // These tests require network access and a running Gitea instance
}
