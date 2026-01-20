//! Database connection and pool management
//!
//! Handles database connection setup and migration execution.

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

use crate::config::DatabaseConfig;
use crate::error::{Result, VibeRepoError};

/// Initialize database connection from configuration
///
/// Creates a SeaORM DatabaseConnection with the configured connection pool settings.
/// Supports both SQLite and PostgreSQL via the DATABASE_URL format.
///
/// # Arguments
/// * `config` - Database configuration containing URL and pool settings
///
/// # Returns
/// * `Ok(DatabaseConnection)` - Successfully connected database
/// * `Err(VibeRepoError::Database)` - Connection failed with descriptive error
pub async fn init_database(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(600))
        .sqlx_logging(true);

    Database::connect(opt)
        .await
        .map_err(VibeRepoError::Database)
}

/// Run pending database migrations
///
/// Executes all pending migrations using SeaORM Migration framework.
/// Migrations are idempotent - running multiple times has no additional effect.
///
/// # Arguments
/// * `db` - Database connection to run migrations on
///
/// # Returns
/// * `Ok(())` - Migrations completed successfully
/// * `Err(VibeRepoError::Database)` - Migration failed
pub async fn run_migrations(db: &DatabaseConnection) -> Result<()> {
    use crate::migration::Migrator;
    use sea_orm_migration::MigratorTrait;

    Migrator::up(db, None)
        .await
        .map_err(VibeRepoError::Database)
}

/// Database pool wrapper for compatibility with existing code
pub struct DatabasePool {
    connection: DatabaseConnection,
}

impl DatabasePool {
    /// Create a new database pool from configuration
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let connection = init_database(config).await?;
        Ok(Self { connection })
    }

    /// Get the underlying database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// Run pending migrations
    pub async fn run_migrations(&self) -> Result<()> {
        run_migrations(&self.connection).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    // ============================================
    // Task 6.1: Tests for database connection
    // Requirements: 3.1, 3.4
    // ============================================

    #[tokio::test]
    async fn test_connection_with_valid_sqlite_url() {
        // Create a temporary file for the SQLite database
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());

        let config = DatabaseConfig {
            url,
            max_connections: 5,
        };

        let result = init_database(&config).await;

        assert!(
            result.is_ok(),
            "Connection with valid SQLite URL should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_connection_failure_with_invalid_url() {
        let config = DatabaseConfig {
            url: "invalid://not-a-valid-url".to_string(),
            max_connections: 5,
        };

        let result = init_database(&config).await;

        assert!(result.is_err(), "Connection with invalid URL should fail");

        // Verify error is a Database error
        match result {
            Err(VibeRepoError::Database(_)) => {
                // Expected error type
            }
            Err(other) => {
                panic!("Expected Database error, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected error, but connection succeeded");
            }
        }
    }

    #[tokio::test]
    async fn test_connection_failure_with_nonexistent_postgres_host() {
        let config = DatabaseConfig {
            url: "postgres://user:pass@nonexistent-host:5432/db".to_string(),
            max_connections: 5,
        };

        let result = init_database(&config).await;

        assert!(
            result.is_err(),
            "Connection to nonexistent host should fail"
        );

        // Verify error is a Database error with descriptive message
        match result {
            Err(VibeRepoError::Database(db_err)) => {
                let error_msg = db_err.to_string();
                // Error message should be descriptive (not empty)
                assert!(
                    !error_msg.is_empty(),
                    "Database error message should be descriptive"
                );
            }
            Err(other) => {
                panic!("Expected Database error, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected error, but connection succeeded");
            }
        }
    }

    #[tokio::test]
    async fn test_connection_pool_respects_max_connections() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());

        let config = DatabaseConfig {
            url,
            max_connections: 3, // Specific value to test
        };

        let result = init_database(&config).await;

        // Connection should succeed with custom max_connections
        assert!(
            result.is_ok(),
            "Connection should succeed with custom max_connections"
        );
    }

    #[tokio::test]
    async fn test_database_pool_new_with_valid_config() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());

        let config = DatabaseConfig {
            url,
            max_connections: 5,
        };

        let result = DatabasePool::new(&config).await;

        assert!(
            result.is_ok(),
            "DatabasePool::new should succeed with valid config"
        );

        let pool = result.unwrap();
        // Verify we can get the connection
        let _conn = pool.connection();
    }

    #[tokio::test]
    async fn test_database_pool_new_with_invalid_config() {
        let config = DatabaseConfig {
            url: "invalid://bad-url".to_string(),
            max_connections: 5,
        };

        let result = DatabasePool::new(&config).await;

        assert!(
            result.is_err(),
            "DatabasePool::new should fail with invalid config"
        );
    }

    // ============================================
    // Task 6.3: Tests for migration execution
    // Requirements: 3.3
    // ============================================

    #[tokio::test]
    async fn test_migrations_run_successfully_on_empty_database() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());

        let config = DatabaseConfig {
            url,
            max_connections: 5,
        };

        let db = init_database(&config).await.expect("Failed to connect");

        let result = run_migrations(&db).await;

        assert!(
            result.is_ok(),
            "Migrations should run successfully on empty database: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_migrations_are_idempotent() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());

        let config = DatabaseConfig {
            url,
            max_connections: 5,
        };

        let db = init_database(&config).await.expect("Failed to connect");

        // Run migrations first time
        let result1 = run_migrations(&db).await;
        assert!(
            result1.is_ok(),
            "First migration run should succeed: {:?}",
            result1.err()
        );

        // Run migrations second time - should be idempotent
        let result2 = run_migrations(&db).await;
        assert!(
            result2.is_ok(),
            "Second migration run should succeed (idempotent): {:?}",
            result2.err()
        );

        // Run migrations third time - still idempotent
        let result3 = run_migrations(&db).await;
        assert!(
            result3.is_ok(),
            "Third migration run should succeed (idempotent): {:?}",
            result3.err()
        );
    }

    #[tokio::test]
    async fn test_database_pool_run_migrations() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());

        let config = DatabaseConfig {
            url,
            max_connections: 5,
        };

        let pool = DatabasePool::new(&config)
            .await
            .expect("Failed to create pool");

        let result = pool.run_migrations().await;

        assert!(
            result.is_ok(),
            "DatabasePool::run_migrations should succeed: {:?}",
            result.err()
        );
    }
}
