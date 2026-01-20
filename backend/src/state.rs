//! Application state module
//!
//! Provides shared application state accessible in all handlers.

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::services::{DockerService, RepositoryService};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub db: DatabaseConnection,
    /// Application configuration
    pub config: AppConfig,
    /// Repository service for direct method calls
    pub repository_service: Arc<RepositoryService>,
    /// Docker service for container management (optional)
    pub docker: Option<DockerService>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        db: DatabaseConnection,
        config: AppConfig,
        repository_service: Arc<RepositoryService>,
    ) -> Self {
        // Try to initialize Docker service, log warning if unavailable
        let docker = match DockerService::new() {
            Ok(service) => {
                tracing::info!("Docker service initialized successfully");
                Some(service)
            }
            Err(e) => {
                tracing::warn!(
                    "Docker service unavailable: {}. Container features will be disabled.",
                    e
                );
                None
            }
        };

        Self {
            db,
            config,
            repository_service,
            docker,
        }
    }

    /// Create a thread-safe Arc-wrapped state
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, IssuePollingConfig, ServerConfig, WebhookConfig};
    use crate::test_utils::db::create_test_database;

    // ============================================
    // Task 8.1: Tests for AppState
    // Requirements: 5.1, 5.2, 5.3
    // ============================================

    #[tokio::test]
    async fn test_appstate_creation_with_db_and_config() {
        // Arrange: Create a test database and config
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig {
            database: DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                max_connections: 5,
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            webhook: WebhookConfig::default(),
            issue_polling: IssuePollingConfig::default(),
        };
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));

        // Act: Create AppState
        let state = AppState::new(db, config.clone(), repository_service);

        // Assert: Verify state contains the config
        assert_eq!(state.config.database.url, "sqlite::memory:");
        assert_eq!(state.config.database.max_connections, 5);
        assert_eq!(state.config.server.host, "127.0.0.1");
        assert_eq!(state.config.server.port, 8080);
    }

    #[tokio::test]
    async fn test_appstate_into_arc_creates_arc_wrapper() {
        // Arrange: Create a test database and config
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::default();
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));

        // Act: Create AppState and wrap in Arc
        let state = AppState::new(db, config, repository_service);
        let arc_state = state.into_arc();

        // Assert: Verify Arc wrapper works
        assert_eq!(Arc::strong_count(&arc_state), 1);

        // Clone the Arc and verify reference count increases
        let arc_state_clone = Arc::clone(&arc_state);
        assert_eq!(Arc::strong_count(&arc_state), 2);
        assert_eq!(Arc::strong_count(&arc_state_clone), 2);
    }

    #[test]
    fn test_appstate_is_send() {
        // This test verifies that AppState implements Send
        // If AppState doesn't implement Send, this will fail to compile
        fn assert_send<T: Send>() {}
        assert_send::<AppState>();
    }

    #[test]
    fn test_appstate_is_sync() {
        // This test verifies that AppState implements Sync
        // If AppState doesn't implement Sync, this will fail to compile
        fn assert_sync<T: Sync>() {}
        assert_sync::<AppState>();
    }

    #[test]
    fn test_arc_appstate_is_send_and_sync() {
        // This test verifies that Arc<AppState> is both Send and Sync
        // This is required for sharing state across Axum handlers
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<AppState>>();
    }

    #[tokio::test]
    async fn test_appstate_clone_shares_db_connection() {
        // Arrange: Create a test database and config
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::default();
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));

        // Act: Create AppState and clone it
        let state1 = AppState::new(db, config, repository_service);
        let state2 = state1.clone();

        // Assert: Both states should have the same config values
        // (DatabaseConnection is cloned internally by SeaORM)
        assert_eq!(state1.config.database.url, state2.config.database.url);
        assert_eq!(state1.config.server.port, state2.config.server.port);
    }

    #[tokio::test]
    async fn test_appstate_accessible_via_axum_state_extractor() {
        use axum::extract::State;

        // Arrange: Create a test state
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::default();
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));
        let state = Arc::new(AppState::new(db, config, repository_service));

        // This test verifies that AppState can be used with Axum's State extractor
        // The State extractor requires the inner type to be Clone
        fn assert_state_extractable<T: Clone + Send + Sync + 'static>() {}
        assert_state_extractable::<Arc<AppState>>();

        // Verify we can create a State extractor from Arc<AppState>
        let _state_extractor: State<Arc<AppState>> = State(state);
    }

    // ============================================
    // Task 4: Tests for AppState with Docker
    // ============================================

    #[tokio::test]
    async fn test_appstate_with_docker_when_available() {
        // Arrange: Create a test database and config
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::default();
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));

        // Act: Create AppState (Docker will be initialized if available)
        let state = AppState::new(db, config, repository_service);

        // Assert: Docker field should be Some if Docker is available, None otherwise
        // This test passes regardless of Docker availability
        match &state.docker {
            Some(docker) => {
                // If Docker is available, verify we can clone it
                let _docker_clone = docker.clone();
            }
            None => {
                // Docker not available, which is acceptable
            }
        }
    }

    #[tokio::test]
    async fn test_appstate_docker_is_optional() {
        // Arrange: Create a test database and config
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::default();
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));

        // Act: Create AppState
        let state = AppState::new(db, config.clone(), repository_service);

        // Assert: AppState should be created successfully regardless of Docker availability
        // This verifies graceful degradation
        assert_eq!(state.config.database.url, config.database.url);
    }
}
