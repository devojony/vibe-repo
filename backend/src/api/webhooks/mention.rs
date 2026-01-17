//! Mention detection for webhook comments

/// Detect if a comment mentions a specific username
///
/// Checks for @username patterns in the comment body.
/// Matches: @username, @username<space>, @username<newline>
pub fn detect_mention(comment_body: &str, username: &str) -> bool {
    let patterns = [
        format!("@{}", username),
        format!("@{} ", username),
        format!("@{}\n", username),
        format!("@{}\r", username),
        format!("@{}\t", username),
    ];
    patterns.iter().any(|p| comment_body.contains(p))
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
}
