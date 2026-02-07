//! Test utilities module
//!
//! Provides helpers for testing, including test database creation,
//! test application state setup, and Gitea test instance utilities.
//!
//! This module is only compiled when running tests or when the
//! `test-utils` feature is enabled.

pub mod db;
pub mod gitea;
pub mod state;

pub use db::{create_test_database, create_test_repository, TestDatabase};
pub use gitea::{is_gitea_available, GiteaTestConfig};
pub use state::{create_test_app, create_test_state};
