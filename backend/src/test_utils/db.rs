//! Test database utilities
//!
//! Provides helpers for creating isolated test databases and test entities.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database, DatabaseConnection};
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::NamedTempFile;

use crate::entities::repository;
use crate::error::{Result, VibeRepoError};
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
            .map_err(|e: std::io::Error| VibeRepoError::Internal(e.to_string()))?;

        let url = format!("sqlite:{}?mode=rwc", temp_file.path().display());
        let connection = Database::connect(&url)
            .await
            .map_err(VibeRepoError::Database)?;

        // Run migrations
        Migrator::up(&connection, None)
            .await
            .map_err(VibeRepoError::Database)?;

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
            .map_err(VibeRepoError::Database)?;

        // Run migrations
        Migrator::up(&connection, None)
            .await
            .map_err(VibeRepoError::Database)?;

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
        .map_err(VibeRepoError::Database)?;

    // Run migrations
    Migrator::up(&connection, None)
        .await
        .map_err(VibeRepoError::Database)?;

    Ok(connection)
}

/// Create a test repository with provider configuration
///
/// Creates a repository with the specified provider configuration.
/// Generates a random webhook_secret for security testing.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `name` - Repository name (e.g., "test-repo")
/// * `full_name` - Full repository name (e.g., "owner/test-repo")
/// * `provider_type` - Provider type (e.g., "github", "gitea", "gitlab")
/// * `provider_base_url` - Provider base URL (e.g., "https://api.github.com")
/// * `access_token` - Access token for the provider
///
/// # Returns
///
/// Returns the created repository model with a randomly generated webhook_secret.
pub async fn create_test_repository(
    db: &DatabaseConnection,
    name: &str,
    full_name: &str,
    provider_type: &str,
    provider_base_url: &str,
    access_token: &str,
) -> Result<repository::Model> {
    use rand::Rng;

    // Generate random webhook_secret (32 bytes = 64 hex characters)
    let mut rng = rand::rng();
    let secret_bytes: [u8; 32] = rng.random();
    let webhook_secret = hex::encode(secret_bytes);

    let repo = repository::ActiveModel {
        name: Set(name.to_string()),
        full_name: Set(full_name.to_string()),
        provider_type: Set(provider_type.to_string()),
        provider_base_url: Set(provider_base_url.to_string()),
        access_token: Set(access_token.to_string()),
        webhook_secret: Set(Some(webhook_secret)),
        clone_url: Set(format!("{}/{}.git", provider_base_url, full_name)),
        default_branch: Set("main".to_string()),
        branches: Set(sea_orm::JsonValue::Array(vec![sea_orm::JsonValue::String(
            "main".to_string(),
        )])),
        validation_status: Set(repository::ValidationStatus::Valid),
        status: Set(repository::RepositoryStatus::Idle),
        has_workspace: Set(false),
        has_required_branches: Set(true),
        has_required_labels: Set(true),
        can_manage_prs: Set(true),
        can_manage_issues: Set(true),
        validation_message: Set(None),
        webhook_status: Set(repository::WebhookStatus::Active),
        agent_command: Set(Some("opencode".to_string())),
        agent_timeout: Set(600),
        agent_env_vars: Set(None),
        docker_image: Set("ubuntu:22.04".to_string()),
        deleted_at: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    repo.insert(db).await.map_err(VibeRepoError::Database)
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
        use crate::entities::prelude::Repository;
        use sea_orm::EntityTrait;

        let count1 = Repository::find().all(&db1).await.unwrap().len();
        let count2 = Repository::find().all(&db2).await.unwrap().len();

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
        use sea_orm::EntityTrait;

        // Insert into db1
        let _repo = create_test_repository(
            &db1.connection,
            "test-repo",
            "owner/test-repo",
            "github",
            "https://api.github.com",
            "test-token",
        )
        .await
        .unwrap();

        // Verify db1 has the repository
        use crate::entities::prelude::Repository;
        let count1 = Repository::find().all(&db1.connection).await.unwrap().len();
        assert_eq!(count1, 1);

        // Verify db2 is empty (isolated)
        let count2 = Repository::find().all(&db2.connection).await.unwrap().len();
        assert_eq!(count2, 0);
    }

    #[tokio::test]
    async fn test_create_test_repository_generates_webhook_secret() {
        // Arrange
        let db = create_test_database().await.unwrap();

        // Act
        let repo = create_test_repository(
            &db,
            "test-repo",
            "owner/test-repo",
            "github",
            "https://api.github.com",
            "test-token",
        )
        .await
        .unwrap();

        // Assert
        assert!(repo.webhook_secret.is_some());
        let secret = repo.webhook_secret.unwrap();
        assert_eq!(secret.len(), 64); // 32 bytes = 64 hex characters
        assert!(secret.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_create_test_repository_sets_correct_fields() {
        // Arrange
        let db = create_test_database().await.unwrap();

        // Act
        let repo = create_test_repository(
            &db,
            "my-repo",
            "myorg/my-repo",
            "gitlab",
            "https://gitlab.com",
            "secret-token",
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(repo.name, "my-repo");
        assert_eq!(repo.full_name, "myorg/my-repo");
        assert_eq!(repo.provider_type, "gitlab");
        assert_eq!(repo.provider_base_url, "https://gitlab.com");
        assert_eq!(repo.access_token, "secret-token");
        assert_eq!(repo.clone_url, "https://gitlab.com/myorg/my-repo.git");
        assert_eq!(repo.default_branch, "main");
        assert_eq!(repo.validation_status, repository::ValidationStatus::Valid);
        assert_eq!(repo.status, repository::RepositoryStatus::Idle);
        assert!(!repo.has_workspace);
    }

    #[tokio::test]
    async fn test_create_test_repository_generates_unique_secrets() {
        // Arrange
        let db = create_test_database().await.unwrap();

        // Act - create two repositories
        let repo1 = create_test_repository(
            &db,
            "repo1",
            "owner/repo1",
            "github",
            "https://api.github.com",
            "token1",
        )
        .await
        .unwrap();

        let repo2 = create_test_repository(
            &db,
            "repo2",
            "owner/repo2",
            "github",
            "https://api.github.com",
            "token2",
        )
        .await
        .unwrap();

        // Assert - webhook secrets should be different
        assert_ne!(repo1.webhook_secret, repo2.webhook_secret);
    }
}
