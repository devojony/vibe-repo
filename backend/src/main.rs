//! GitAutoDev Backend Application
//!
//! Main entry point for the GitAutoDev automated programming assistant.

use anyhow::Result;
use std::sync::Arc;

use gitautodev::{
    api::create_router,
    config::AppConfig,
    db::database::DatabasePool,
    logging,
    services::{RepositoryService, ServiceManager, WebhookCleanupService, WebhookRetryService},
    state::AppState,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists (silently ignore if not found)
    // This allows environment variables to be loaded from .env file in development
    dotenvy::dotenv().ok();

    // Initialize logging
    // Use JSON format in production (when LOG_FORMAT=json), human-readable in development
    let json_format = std::env::var("LOG_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or(false);

    logging::init_tracing(json_format);

    tracing::info!("Starting GitAutoDev system...");

    // Load configuration
    let config = AppConfig::from_env().map_err(|e| anyhow::anyhow!("{}", e))?;
    tracing::info!("Configuration loaded: {:?}", config);

    // Initialize database connection
    let db_pool = DatabasePool::new(&config.database).await?;
    tracing::info!("Database connection established");

    // Run migrations
    db_pool.run_migrations().await?;
    tracing::info!("Database migrations completed");

    // Create repository service (shared across handlers and background tasks)
    let repository_service = Arc::new(RepositoryService::new(db_pool.connection().clone()));
    tracing::info!("Repository service created");

    // Create application state
    let state = AppState::new(
        db_pool.connection().clone(),
        config.clone(),
        repository_service.clone(),
    );
    let state = Arc::new(state);

    // Initialize service manager and register services
    let mut service_manager = ServiceManager::new();

    // Register RepositoryService for background periodic sync
    let background_service = RepositoryService::new(db_pool.connection().clone());
    service_manager.register(background_service);

    // Register WebhookRetryService for failed webhook retry
    let config_arc = Arc::new(config.clone());
    let webhook_retry_service = WebhookRetryService::new(db_pool.connection().clone(), config_arc.clone());
    service_manager.register(webhook_retry_service);

    // Register WebhookCleanupService for orphaned webhook cleanup
    let webhook_cleanup_service = WebhookCleanupService::new(db_pool.connection().clone(), config_arc.clone());
    service_manager.register(webhook_cleanup_service);

    service_manager.start_all(state.clone()).await?;
    tracing::info!("Background services started");

    // Create router
    let app = create_router(state);

    // Start web server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting web server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Graceful shutdown handling
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        tracing::info!("Shutdown signal received");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    // Stop services on shutdown
    service_manager.stop_all().await?;
    tracing::info!("GitAutoDev system shutdown complete");

    Ok(())
}
