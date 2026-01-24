//! End-to-end integration tests with real Gitea instance
//!
//! This module provides the entry point for E2E tests. The actual test
//! infrastructure is organized in the e2e/ subdirectory.
//!
//! ## File Structure
//!
//! Due to Rust's module system and Cargo's test discovery mechanism, we use
//! the following structure:
//!
//! - `tests/e2e.rs` - Integration test entry point (auto-discovered by Cargo)
//! - `tests/e2e/helpers.rs` - Test helper utilities
//! - `tests/e2e/gitea_client.rs` - Gitea API client for E2E tests
//! - `tests/e2e/tests.rs` - Actual E2E test cases
//!
//! Note: We cannot use `tests/e2e/mod.rs` as the entry point because Cargo
//! only auto-discovers `tests/*.rs` files as integration tests. Having both
//! `tests/e2e.rs` and `tests/e2e/mod.rs` would create a module ambiguity error.

#[path = "e2e/helpers.rs"]
pub mod helpers;

#[path = "e2e/gitea_client.rs"]
pub mod gitea_client;

#[cfg(test)]
#[path = "e2e/tests.rs"]
mod tests;
