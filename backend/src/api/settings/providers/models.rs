//! Provider API models
//!
//! Request and response DTOs for the RepoProvider API.

use crate::entities::repo_provider::{Model as RepoProviderModel, ProviderType};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to create a new provider
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProviderRequest {
    /// Display name for the provider
    pub name: String,
    /// Provider type (only 'gitea' supported in v0.1.0)
    pub provider_type: ProviderType,
    /// Base URL for the Git provider instance
    pub base_url: String,
    /// Authentication token for API access
    pub access_token: String,
}

/// Request to update an existing provider
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProviderRequest {
    /// Display name for the provider
    pub name: Option<String>,
    /// Provider type (only 'gitea' supported in v0.1.0)
    pub provider_type: Option<ProviderType>,
    /// Base URL for the Git provider instance
    pub base_url: Option<String>,
    /// Authentication token for API access
    pub access_token: Option<String>,
    /// Whether the provider is locked (cannot be deleted)
    pub locked: Option<bool>,
}

/// Provider response with masked credentials
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProviderResponse {
    /// Provider ID
    pub id: i32,
    /// Display name
    pub name: String,
    /// Provider type
    pub provider_type: ProviderType,
    /// Base URL
    pub base_url: String,
    /// Masked access token (first 8 chars + "***")
    pub access_token: String,
    /// Whether the provider is locked
    pub locked: bool,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last update timestamp (ISO 8601)
    pub updated_at: String,
}

/// Token validation response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationResponse {
    /// Whether the token is valid
    pub valid: bool,
    /// Validation message
    pub message: String,
    /// User information (if valid)
    pub user_info: Option<UserInfo>,
}

/// User information from provider
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    /// Username
    pub username: String,
    /// User ID
    pub id: i64,
    /// Email address
    pub email: Option<String>,
}

impl ProviderResponse {
    /// Convert entity model to response DTO with masked token
    pub fn from_model(model: RepoProviderModel) -> Self {
        Self {
            id: model.id,
            name: model.name,
            provider_type: model.provider_type,
            base_url: model.base_url,
            access_token: mask_token(&model.access_token),
            locked: model.locked,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }
}

/// Mask sensitive token for API responses
///
/// Shows first 8 characters followed by "***", or just "***" if shorter than 8 characters.
///
/// # Examples
///
/// ```
/// use vibe_repo::api::settings::providers::models::mask_token;
///
/// assert_eq!(mask_token("ghp_1234567890abcdef"), "ghp_1234***");
/// assert_eq!(mask_token("short"), "***");
/// assert_eq!(mask_token(""), "***");
/// ```
pub fn mask_token(token: &str) -> String {
    let char_count = token.chars().count();
    if char_count <= 8 {
        "***".to_string()
    } else {
        let prefix: String = token.chars().take(8).collect();
        format!("{}***", prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_token_long() {
        assert_eq!(mask_token("ghp_1234567890abcdef"), "ghp_1234***");
    }

    #[test]
    fn test_mask_token_short() {
        assert_eq!(mask_token("short"), "***");
    }

    #[test]
    fn test_mask_token_empty() {
        assert_eq!(mask_token(""), "***");
    }

    #[test]
    fn test_mask_token_exactly_8() {
        assert_eq!(mask_token("12345678"), "***");
    }

    #[test]
    fn test_mask_token_9_chars() {
        assert_eq!(mask_token("123456789"), "12345678***");
    }

    // Edge case tests for various token lengths
    #[test]
    fn test_mask_token_single_char() {
        assert_eq!(mask_token("a"), "***");
    }

    #[test]
    fn test_mask_token_two_chars() {
        assert_eq!(mask_token("ab"), "***");
    }

    #[test]
    fn test_mask_token_seven_chars() {
        assert_eq!(mask_token("1234567"), "***");
    }

    #[test]
    fn test_mask_token_very_long() {
        let long_token = "a".repeat(100);
        let expected = format!("{}***", &long_token[..8]);
        assert_eq!(mask_token(&long_token), expected);
    }

    // Tests for tokens with special characters
    #[test]
    fn test_mask_token_with_special_chars() {
        assert_eq!(mask_token("abc!@#$%^&*()"), "abc!@#$%***");
    }

    #[test]
    fn test_mask_token_with_unicode() {
        // "token🔑🔐🔒" has 7 characters (5 ASCII + 3 emoji), so should return "***"
        assert_eq!(mask_token("token🔑🔐🔒"), "***");
        // Test with 9 characters including unicode
        assert_eq!(mask_token("token🔑🔐🔒🔓"), "token🔑🔐🔒***");
    }

    #[test]
    fn test_mask_token_with_spaces() {
        assert_eq!(mask_token("tok en 12345"), "tok en 1***");
    }

    #[test]
    fn test_mask_token_with_newlines() {
        assert_eq!(mask_token("token\n\r\t123"), "token\n\r\t***");
    }

    #[test]
    fn test_mask_token_all_special_chars() {
        assert_eq!(mask_token("!@#$%^&*()"), "!@#$%^&****");
    }

    #[test]
    fn test_mask_token_with_quotes() {
        assert_eq!(mask_token("\"token'123\""), "\"token'1***");
    }
}
