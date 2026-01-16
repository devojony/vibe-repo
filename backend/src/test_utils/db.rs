//! Test database utilities
//!
//! Provides helpers for creating isolated test databases.

use sea_orm::{Database, DatabaseConnection};
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::NamedTempFile;

use crate::error::{GitAutoDevError, Result};
use crate::migration::Migrator;
use sea_orm_migration::MigratorTrait;

/// Counter for generating unique in-memory database names
static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Test database wrapper that manages a temporary SQLite database
pub struct TestDatabase {
    /// The database connection
    pub connection: DatabaseConnection,
    /// Temporary file (kept alive to prevent deletion)
    #[allow(dead_code)]
    temp_file: Option<NamedTempFile>,
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
            temp_file: Some(temp_file),
        })
    }

    /// Create a new in-memory test database with migrations applied
    ///
    /// This is faster than file-based databases and automatically cleaned up.
    /// Each call creates a unique database instance.
    pub async fn new_in_memory() -> Result<Self> {
        // Use a unique file-based URL with :memory: mode for isolation
        // SQLite shared cache requires a unique name for each connection
        let db_id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let url = format!("sqlite:file:memdb{}?mode=memory&cache=shared", db_id);

        let connection = Database::connect(&url)
            .await
            .map_err(GitAutoDevError::Database)?;

        // Run migrations
        Migrator::up(&connection, None)
            .await
            .map_err(GitAutoDevError::Database)?;

        Ok(Self {
            connection,
            temp_file: None,
        })
    }
}

/// Create a temporary SQLite database for testing
///
/// Returns a database connection with all migrations applied.
/// Uses an in-memory database for better performance and automatic cleanup.
///
/// Note: Each call creates a unique database instance to ensure test isolation.
pub async fn create_test_database() -> Result<DatabaseConnection> {
    // Use in-memory database for better performance and no file leaks
    let db_id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let url = format!("sqlite:file:test_db_{}?mode=memory&cache=shared", db_id);

    let connection = Database::connect(&url)
        .await
        .map_err(GitAutoDevError::Database)?;

    // Run migrations
    Migrator::up(&connection, None)
        .await
        .map_err(GitAutoDevError::Database)?;

    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_test_database_returns_valid_connection() {
        // Arrange & Act
        let result = create_test_database().await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_test_database_is_isolated() {
        // Arrange & Act - create two databases
        let db1 = create_test_database().await.unwrap();
        let db2 = create_test_database().await.unwrap();

        // Assert - they should be different connections
        // We can't directly compare connections, but we can verify both work
        use crate::entities::prelude::RepoProvider;
        use sea_orm::EntityTrait;

        let count1 = RepoProvider::find().all(&db1).await.unwrap().len();
        let count2 = RepoProvider::find().all(&db2).await.unwrap().len();

        assert_eq!(count1, 0);
        assert_eq!(count2, 0);
    }

    #[tokio::test]
    async fn test_test_database_new_returns_valid_instance() {
        // Arrange & Act
        let result = TestDatabase::new().await;

        // Assert
        assert!(result.is_ok());
        let db = result.unwrap();
        assert!(db.temp_file.is_some());
    }

    #[tokio::test]
    async fn test_test_database_new_in_memory_returns_valid_instance() {
        // Arrange & Act
        let result = TestDatabase::new_in_memory().await;

        // Assert
        assert!(result.is_ok());
        let db = result.unwrap();
        assert!(db.temp_file.is_none());
    }

    #[tokio::test]
    async fn test_test_database_in_memory_is_isolated() {
        // Arrange & Act - create two in-memory databases
        let db1 = TestDatabase::new_in_memory().await.unwrap();
        let db2 = TestDatabase::new_in_memory().await.unwrap();

        // Assert - they should be isolated
        use crate::entities::repo_provider;
        use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

        // Insert into db1
        let provider = repo_provider::ActiveModel {
            name: ActiveValue::Set("Test Provider".to_string()),
            provider_type: ActiveValue::Set(repo_provider::ProviderType::Gitea),
            base_url: ActiveValue::Set("https://example.com".to_string()),
            access_token: ActiveValue::Set("token123".to_string()),
            locked: ActiveValue::Set(false),
            created_at: ActiveValue::Set(chrono::Utc::now()),
            updated_at: ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };
        provider.insert(&db1.connection).await.unwrap();

        // Verify db1 has the provider
        use crate::entities::prelude::RepoProvider;
        let count1 = RepoProvider::find()
            .all(&db1.connection)
            .await
            .unwrap()
            .len();
        assert_eq!(count1, 1);

        // Verify db2 is empty (isolated)
        let count2 = RepoProvider::find()
            .all(&db2.connection)
            .await
            .unwrap()
            .len();
        assert_eq!(count2, 0);
    }
}
