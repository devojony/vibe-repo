//! Mention detection for webhook comments

/// Detect if a comment mentions a specific username
///
/// Checks for @username patterns in the comment body.
/// Matches: @username followed by whitespace, punctuation, or end of string
/// Does not match: @username followed by alphanumeric characters (e.g., @bot123)
pub fn detect_mention(comment_body: &str, username: &str) -> bool {
    let mention = format!("@{}", username);
    
    // Check if mention exists in the comment
    if !comment_body.contains(&mention) {
        return false;
    }
    
    // Find all occurrences and check if they are valid mentions
    comment_body.match_indices(&mention).any(|(idx, _)| {
        let after_idx = idx + mention.len();
        
        // Check if there's a character after the mention
        if after_idx < comment_body.len() {
            let next_char = comment_body.chars().nth(after_idx).unwrap();
            // Valid if followed by whitespace or punctuation (not alphanumeric)
            next_char.is_whitespace() || !next_char.is_alphanumeric()
        } else {
            // Valid if at end of string
            true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mention_at_start() {
        assert!(detect_mention("@bot please help", "bot"));
    }

    #[test]
    fn test_detect_mention_in_middle() {
        assert!(detect_mention("Hey @bot can you help?", "bot"));
    }

    #[test]
    fn test_detect_mention_at_end() {
        assert!(detect_mention("Please help @bot", "bot"));
    }

    #[test]
    fn test_detect_mention_with_newline() {
        assert!(detect_mention("@bot\nPlease help", "bot"));
    }

    #[test]
    fn test_no_mention() {
        assert!(!detect_mention("Please help", "bot"));
    }

    #[test]
    fn test_partial_match_not_detected() {
        assert!(!detect_mention("@robot please help", "bot"));
    }

    #[test]
    fn test_case_sensitive() {
        assert!(!detect_mention("@Bot please help", "bot"));
    }

    #[test]
    fn test_mention_with_punctuation() {
        assert!(detect_mention("Hey @bot! Can you help?", "bot"));
        assert!(detect_mention("Hey @bot, can you help?", "bot"));
        assert!(detect_mention("Hey @bot. Can you help?", "bot"));
    }

    #[test]
    fn test_alphanumeric_suffix_not_detected() {
        assert!(!detect_mention("@bot123 please help", "bot"));
        assert!(!detect_mention("@botman please help", "bot"));
    }
}
