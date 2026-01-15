//! Test utilities module
//!
//! Provides helpers for testing, including test database creation
//! and test application state setup.
//!
//! This module is only compiled when running tests or when the
//! `test-utils` feature is enabled.

pub mod db;
pub mod state;

pub use db::{create_test_database, TestDatabase};
pub use state::{create_test_app, create_test_state};
