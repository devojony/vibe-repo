//! End-to-end integration tests with real Gitea instance

#[path = "e2e/helpers.rs"]
pub mod helpers;

#[path = "e2e/gitea_client.rs"]
pub mod gitea_client;

// Tests module will be added in Task 2
// #[cfg(test)]
// #[path = "e2e/tests.rs"]
// mod tests;
