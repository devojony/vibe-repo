//! Test state utilities
//!
//! Provides helpers for creating test application state.

use axum::Router;
use std::sync::Arc;

use crate::api::create_router;
use crate::config::AppConfig;
use crate::error::Result;
use crate::services::RepositoryService;
use crate::state::AppState;

use super::db::create_test_database;

/// Create test application state with a temporary database
///
/// Returns an Arc-wrapped AppState suitable for use in tests.
pub async fn create_test_state() -> Result<Arc<AppState>> {
    let db = create_test_database().await?;
    let config = AppConfig::default();
    let config_arc = Arc::new(config.clone());
    let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));
    Ok(Arc::new(AppState::new(db, config, repository_service)))
}

/// Create a test router with test state
///
/// Returns a fully configured router suitable for integration testing.
pub async fn create_test_app() -> Result<Router> {
    let state = create_test_state().await?;
    Ok(create_router(state))
}
