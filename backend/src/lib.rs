//! VibeRepo Backend Library
//!
//! This library provides the core functionality for the VibeRepo automated
//! programming assistant system.

// Core modules
pub mod config;
pub mod error;
pub mod logging;
pub mod state;

// API layer
pub mod api;

// Data layer
pub mod db;
pub mod entities;
pub mod migration;

// Service layer
pub mod services;

// Git provider abstraction
pub mod git_provider;

// Test utilities (only compiled in test mode or with test-utils feature)
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

// Re-export commonly used types
pub use config::AppConfig;
pub use error::{VibeRepoError, Result};
pub use state::AppState;

// Re-export database types
pub use db::database::{init_database, run_migrations, DatabasePool};
