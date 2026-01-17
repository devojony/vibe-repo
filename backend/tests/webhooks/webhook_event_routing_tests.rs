//! Integration tests for webhook event routing

use vibe_repo::api::webhooks::mention::detect_mention;

#[test]
fn test_mention_detection_basic() {
    assert!(detect_mention("@bot help", "bot"));
    assert!(!detect_mention("help", "bot"));
}

#[test]
fn test_mention_detection_multiple_patterns() {
    assert!(detect_mention("@bot please", "bot"));
    assert!(detect_mention("Hey @bot ", "bot"));
    assert!(detect_mention("@bot\nNew line", "bot"));
}

#[test]
fn test_mention_detection_edge_cases() {
    // Should not match partial usernames
    assert!(!detect_mention("@robot", "bot"));
    
    // Should be case sensitive
    assert!(!detect_mention("@Bot", "bot"));
    
    // Should match at end of string
    assert!(detect_mention("Help me @bot", "bot"));
}
