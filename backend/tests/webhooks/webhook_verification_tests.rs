//! Integration tests for webhook signature verification

use vibe_repo::api::webhooks::verification::*;

#[test]
fn test_verify_gitea_signature_valid() {
    let body = r#"{"test": "data"}"#;
    let secret = "my-secret";

    // Calculate expected signature
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let signature = format!("{:x}", mac.finalize().into_bytes());

    let result = verify_webhook_signature("gitea", &signature, body, secret);

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_verify_gitea_signature_invalid() {
    let body = r#"{"test": "data"}"#;
    let secret = "my-secret";
    let wrong_signature = "wrong_signature";

    let result = verify_webhook_signature("gitea", wrong_signature, body, secret);

    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_verify_github_signature_with_prefix() {
    let body = r#"{"test": "data"}"#;
    let secret = "my-secret";

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let sig = format!("sha256={:x}", mac.finalize().into_bytes());

    let result = verify_webhook_signature(
        "github", // GitHub uses same method as Gitea
        &sig, body, secret,
    );

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_verify_signature_empty_secret() {
    let body = r#"{"test": "data"}"#;
    let secret = "";
    let signature = "any_signature";

    let result = verify_webhook_signature("gitea", signature, body, secret);

    // Should handle empty secret gracefully
    assert!(result.is_ok());
}

#[test]
fn test_verify_signature_empty_body() {
    let body = "";
    let secret = "my-secret";

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let signature = format!("{:x}", mac.finalize().into_bytes());

    let result = verify_webhook_signature("gitea", &signature, body, secret);

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_verify_signature_case_sensitivity() {
    let body = r#"{"test": "data"}"#;
    let secret = "my-secret";

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let signature = format!("{:x}", mac.finalize().into_bytes());

    // Try with uppercase signature (should fail)
    let uppercase_sig = signature.to_uppercase();
    let result = verify_webhook_signature("gitea", &uppercase_sig, body, secret);

    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_verify_signature_with_special_characters() {
    let body = r#"{"test": "data with special chars: !@#$%^&*()"}"#;
    let secret = "secret-with-special-chars-!@#$";

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let signature = format!("{:x}", mac.finalize().into_bytes());

    let result = verify_webhook_signature("gitea", &signature, body, secret);

    assert!(result.is_ok());
    assert!(result.unwrap());
}
