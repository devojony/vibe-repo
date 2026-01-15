//! Test database utilities
//!
//! Provides helpers for creating isolated test databases.

use sea_orm::{Database, DatabaseConnection};
use tempfile::NamedTempFile;

use crate::error::{GitAutoDevError, Result};
use crate::migration::Migrator;
use sea_orm_migration::MigratorTrait;

/// Test database wrapper that manages a temporary SQLite database
pub struct TestDatabase {
    /// The database connection
    pub connection: DatabaseConnection,
    /// Temporary file (kept alive to prevent deletion)
    #[allow(dead_code)]
    temp_file: NamedTempFile,
}

impl TestDatabase {
    /// Create a new test database with migrations applied
    pub async fn new() -> Result<Self> {
        let temp_file = NamedTempFile::new()
            .map_err(|e: std::io::Error| GitAutoDevError::Internal(e.to_string()))?;

        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());
        let connection = Database::connect(&url)
            .await
            .map_err(GitAutoDevError::Database)?;

        // Run migrations
        Migrator::up(&connection, None)
            .await
            .map_err(GitAutoDevError::Database)?;

        Ok(Self {
            connection,
            temp_file,
        })
    }
}

/// Create a temporary SQLite database for testing
///
/// Returns a database connection with all migrations applied.
/// The database is automatically cleaned up when the connection is dropped.
pub async fn create_test_database() -> Result<DatabaseConnection> {
    let temp_file = NamedTempFile::new()
        .map_err(|e: std::io::Error| GitAutoDevError::Internal(e.to_string()))?;

    let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());
    let connection = Database::connect(&url)
        .await
        .map_err(GitAutoDevError::Database)?;

    // Run migrations
    Migrator::up(&connection, None)
        .await
        .map_err(GitAutoDevError::Database)?;

    // Leak the temp_file to keep it alive for the duration of the test
    // Note: This is intentional - the file will be cleaned up when the process exits
    std::mem::forget(temp_file);

    Ok(connection)
}
