//! Log Cleanup Service
//!
//! Background service that cleans up old init script log files.
//!
//! This service runs daily to delete log files older than 30 days and
//! remove empty workspace directories.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

use crate::entities::prelude::*;
use crate::entities::init_script;
use crate::error::Result;
use crate::services::BackgroundService;
use crate::state::AppState;

/// Log cleanup service
///
/// Periodically scans for old log files and removes them from the filesystem.
pub struct LogCleanupService {
    db: DatabaseConnection,
    log_base_dir: String,
    retention_days: i64,
}

impl LogCleanupService {
    /// Create a new log cleanup service
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `log_base_dir` - Base directory for log files (default: /data/gitautodev/init-script-logs)
    /// * `retention_days` - Number of days to retain logs (default: 30)
    pub fn new(db: DatabaseConnection, log_base_dir: Option<String>, retention_days: Option<i64>) -> Self {
        Self {
            db,
            log_base_dir: log_base_dir.unwrap_or_else(|| "/data/gitautodev/init-script-logs".to_string()),
            retention_days: retention_days.unwrap_or(30),
        }
    }

    /// Clean up old log files
    ///
    /// Deletes log files older than retention_days and removes empty workspace directories.
    async fn cleanup_old_logs(&self) -> Result<()> {
        tracing::info!(
            retention_days = self.retention_days,
            "Starting init script log cleanup"
        );

        let cutoff_date = Utc::now() - Duration::days(self.retention_days);

        // Find init scripts with old log files
        let old_scripts = InitScript::find()
            .filter(init_script::Column::ExecutedAt.lt(cutoff_date))
            .filter(init_script::Column::OutputFilePath.is_not_null())
            .all(&self.db)
            .await?;

        let mut deleted_files = 0;
        let mut deleted_db_entries = 0;
        let mut errors = 0;

        for script in old_scripts {
            if let Some(file_path) = &script.output_file_path {
                // Delete the log file
                match fs::remove_file(file_path).await {
                    Ok(_) => {
                        tracing::debug!(
                            script_id = script.id,
                            workspace_id = script.workspace_id,
                            file_path = %file_path,
                            "Deleted old log file"
                        );
                        deleted_files += 1;

                        // Update database to clear file path
                        let mut script_active: init_script::ActiveModel = script.into();
                        script_active.output_file_path = sea_orm::Set(None);

                        if let Err(e) = script_active.update(&self.db).await {
                            tracing::error!(
                                error = %e,
                                "Failed to update database after deleting log file"
                            );
                            errors += 1;
                        } else {
                            deleted_db_entries += 1;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        // File already deleted, just update database
                        tracing::debug!(
                            script_id = script.id,
                            file_path = %file_path,
                            "Log file already deleted, updating database"
                        );

                        let mut script_active: init_script::ActiveModel = script.into();
                        script_active.output_file_path = sea_orm::Set(None);

                        if let Err(e) = script_active.update(&self.db).await {
                            tracing::error!(
                                error = %e,
                                "Failed to update database for missing log file"
                            );
                            errors += 1;
                        } else {
                            deleted_db_entries += 1;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            script_id = script.id,
                            file_path = %file_path,
                            error = %e,
                            "Failed to delete log file"
                        );
                        errors += 1;
                    }
                }
            }
        }

        // Clean up empty workspace directories
        let empty_dirs = self.cleanup_empty_directories().await?;

        tracing::info!(
            deleted_files = deleted_files,
            deleted_db_entries = deleted_db_entries,
            empty_dirs_removed = empty_dirs,
            errors = errors,
            "Init script log cleanup complete"
        );

        Ok(())
    }

    /// Clean up empty workspace directories
    ///
    /// Scans the log base directory and removes any empty workspace directories.
    async fn cleanup_empty_directories(&self) -> Result<usize> {
        let base_path = Path::new(&self.log_base_dir);

        if !base_path.exists() {
            tracing::debug!(
                base_dir = %self.log_base_dir,
                "Log base directory does not exist, skipping empty directory cleanup"
            );
            return Ok(0);
        }

        let mut removed_count = 0;
        let mut entries = fs::read_dir(base_path).await.map_err(|e| {
            crate::error::VibeRepoError::Internal(format!("Failed to read log directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            crate::error::VibeRepoError::Internal(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            if path.is_dir() {
                // Check if directory is empty
                match fs::read_dir(&path).await {
                    Ok(mut dir_entries) => {
                        if dir_entries.next_entry().await.map_err(|e| {
                            crate::error::VibeRepoError::Internal(format!("Failed to check directory: {}", e))
                        })?.is_none() {
                            // Directory is empty, remove it
                            if let Err(e) = fs::remove_dir(&path).await {
                                tracing::warn!(
                                    path = %path.display(),
                                    error = %e,
                                    "Failed to remove empty directory"
                                );
                            } else {
                                tracing::debug!(
                                    path = %path.display(),
                                    "Removed empty workspace directory"
                                );
                                removed_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to read workspace directory"
                        );
                    }
                }
            }
        }

        Ok(removed_count)
    }
}

#[async_trait]
impl BackgroundService for LogCleanupService {
    fn name(&self) -> &'static str {
        "log_cleanup_service"
    }

    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        tracing::info!(
            retention_days = self.retention_days,
            log_base_dir = %self.log_base_dir,
            "Starting log cleanup service"
        );

        let db = self.db.clone();
        let log_base_dir = self.log_base_dir.clone();
        let retention_days = self.retention_days;

        tokio::spawn(async move {
            let service = LogCleanupService::new(db, Some(log_base_dir), Some(retention_days));

            // Run daily at 2 AM (approximately)
            // First run after 2 hours, then every 24 hours
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400)); // 24 hours

            // Skip the first immediate tick
            interval.tick().await;

            loop {
                interval.tick().await;

                if let Err(e) = service.cleanup_old_logs().await {
                    tracing::error!(error = %e, "Error in log cleanup service");
                }
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("Log cleanup service stopped");
        Ok(())
    }

    async fn health_check(&self) -> bool {
        // Check database connection
        self.db.ping().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestDatabase;

    #[tokio::test]
    async fn test_cleanup_old_logs() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("test-log-cleanup");
        let _ = fs::create_dir_all(&temp_dir).await;

        let service = LogCleanupService::new(
            db.clone(),
            Some(temp_dir.to_string_lossy().to_string()),
            Some(30),
        );

        // Test that service can be created and cleanup runs without error
        let result = service.cleanup_old_logs().await;
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_cleanup_empty_directories() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create a temporary directory structure
        let temp_dir = std::env::temp_dir().join("test-empty-dirs");
        let empty_dir = temp_dir.join("workspace-1");
        let _ = fs::create_dir_all(&empty_dir).await;

        let service = LogCleanupService::new(
            db.clone(),
            Some(temp_dir.to_string_lossy().to_string()),
            Some(30),
        );

        // Run cleanup
        let result = service.cleanup_empty_directories().await;
        assert!(result.is_ok());

        let removed = result.unwrap();
        assert_eq!(removed, 1);

        // Verify directory was removed
        assert!(!empty_dir.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir).await;
    }
}
