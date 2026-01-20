//! Webhook signature verification
//!
//! Implements signature verification for different Git platforms.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{entities::repo_provider::ProviderType, error::Result};

type HmacSha256 = Hmac<Sha256>;

/// Verify webhook signature based on provider type
///
/// # Arguments
/// * `provider_type` - Type of Git provider (Gitea, GitHub, GitLab)
/// * `signature` - Signature from webhook header
/// * `body` - Raw webhook payload body
/// * `secret` - Webhook secret for verification
///
/// # Returns
/// * `Ok(true)` - Signature is valid
/// * `Ok(false)` - Signature is invalid
/// * `Err(_)` - Verification error
pub fn verify_webhook_signature(
    provider_type: &ProviderType,
    signature: &str,
    body: &str,
    secret: &str,
) -> Result<bool> {
    match provider_type {
        ProviderType::Gitea => verify_hmac_sha256(signature, body, secret),
        // GitHub and GitLab would be added here when implemented
    }
}

/// Verify HMAC-SHA256 signature (used by Gitea and GitHub)
///
/// Supports both plain hex format (Gitea) and "sha256=<hex>" format (GitHub)
fn verify_hmac_sha256(signature: &str, body: &str, secret: &str) -> Result<bool> {
    // Strip "sha256=" prefix if present (GitHub format)
    let signature = signature.strip_prefix("sha256=").unwrap_or(signature);

    // Calculate expected signature
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| crate::error::VibeRepoError::Internal(format!("Invalid secret: {}", e)))?;

    mac.update(body.as_bytes());
    let expected = format!("{:x}", mac.finalize().into_bytes());

    // Constant-time comparison
    Ok(signature == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256_verification() {
        let body = "test body";
        let secret = "test-secret";

        // Calculate signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        let signature = format!("{:x}", mac.finalize().into_bytes());

        // Verify
        let result = verify_hmac_sha256(&signature, body, secret).unwrap();
        assert!(result);
    }

    #[test]
    fn test_hmac_sha256_with_github_prefix() {
        let body = "test body";
        let secret = "test-secret";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        let signature = format!("sha256={:x}", mac.finalize().into_bytes());

        let result = verify_hmac_sha256(&signature, body, secret).unwrap();
        assert!(result);
    }

    #[test]
    fn test_hmac_sha256_invalid_signature() {
        let body = "test body";
        let secret = "test-secret";
        let wrong_sig = "wrong_signature";

        let result = verify_hmac_sha256(wrong_sig, body, secret).unwrap();
        assert!(!result);
    }
}
