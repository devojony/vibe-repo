//! Test helper utilities for E2E testing
//!
//! This module provides utility functions for E2E tests including
//! condition polling and test name generation.

use std::future::Future;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Waits for a condition to become true with timeout
///
/// This function polls a condition function at regular intervals until it returns true
/// or the timeout is reached.
///
/// # Arguments
///
/// * `condition` - An async function that returns a boolean indicating if the condition is met
/// * `timeout_secs` - Maximum time to wait in seconds
/// * `check_interval_ms` - Interval between checks in milliseconds
///
/// # Returns
///
/// Returns Ok(()) if the condition becomes true within the timeout,
/// or Err with a descriptive message if the timeout is reached
///
/// # Example
///
/// ```no_run
/// use vibe_repo_e2e::helpers::wait_for_condition;
///
/// # async fn example() -> Result<(), String> {
/// wait_for_condition(
///     || async { check_some_condition().await },
///     30,
///     500
/// ).await?;
/// # Ok(())
/// # }
/// # async fn check_some_condition() -> bool { true }
/// ```
pub async fn wait_for_condition<F, Fut>(
    condition: F,
    timeout_secs: u64,
    check_interval_ms: u64,
) -> Result<(), String>
where
    F: Fn() -> Fut,
    Fut: Future<Output = bool>,
{
    let timeout_duration = Duration::from_secs(timeout_secs);
    let check_interval = Duration::from_millis(check_interval_ms);

    let result = timeout(timeout_duration, async {
        loop {
            if condition().await {
                return;
            }
            sleep(check_interval).await;
        }
    })
    .await;

    match result {
        Ok(()) => Ok(()),
        Err(_) => Err(format!(
            "Timeout after {} seconds waiting for condition",
            timeout_secs
        )),
    }
}

/// Generates a unique test name with timestamp
///
/// This function creates a unique name for test resources (repositories, branches, etc.)
/// by combining a prefix with a timestamp to avoid conflicts between test runs.
///
/// # Arguments
///
/// * `prefix` - Prefix for the test name (e.g., "test-repo", "test-branch")
///
/// # Returns
///
/// Returns a unique string in the format "{prefix}-{timestamp}"
///
/// # Example
///
/// ```
/// use vibe_repo_e2e::helpers::generate_test_name;
///
/// let repo_name = generate_test_name("test-repo");
/// // Returns something like: "test-repo-1706123456789"
/// ```
pub fn generate_test_name(prefix: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        // This is safe: SystemTime::now() is always after UNIX_EPOCH on all supported platforms.
        // The only way this could fail is if the system clock is set before 1970-01-01,
        // which would indicate a serious system misconfiguration.
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis();

    format!("{}-{}", prefix, timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wait_for_condition_success() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let condition = move || {
            let counter = counter_clone.clone();
            async move {
                let val = counter.fetch_add(1, Ordering::SeqCst);
                val >= 2
            }
        };

        let result = wait_for_condition(condition, 5, 100).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_condition_timeout() {
        let condition = || async { false };

        let result = wait_for_condition(condition, 1, 100).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Timeout"));
    }

    #[tokio::test]
    async fn test_wait_for_condition_immediate_success() {
        let condition = || async { true };

        let result = wait_for_condition(condition, 5, 100).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_test_name() {
        let name1 = generate_test_name("test-repo");
        
        // Sleep briefly to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(2));
        
        let name2 = generate_test_name("test-repo");

        // Names should start with the prefix
        assert!(name1.starts_with("test-repo-"));
        assert!(name2.starts_with("test-repo-"));

        // Names should be unique (different timestamps)
        assert_ne!(name1, name2);
    }

    #[test]
    fn test_generate_test_name_different_prefixes() {
        let repo_name = generate_test_name("repo");
        let branch_name = generate_test_name("branch");

        assert!(repo_name.starts_with("repo-"));
        assert!(branch_name.starts_with("branch-"));
    }
}
