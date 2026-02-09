//! Timeout Watchdog Service
//!
//! Monitors task execution times and enforces timeouts by killing stuck agent processes.
//! This service is necessary because the ACP client uses blocking I/O that is not
//! cancellation-aware, so we must enforce timeouts at the process level.

use crate::entities::{
    prelude::*,
    task::{self, TaskStatus},
};
use crate::error::{Result, VibeRepoError};
use crate::services::service_manager::BackgroundService;
use crate::state::AppState;
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Information about an active task execution
#[derive(Debug, Clone)]
pub struct TaskExecution {
    pub task_id: i32,
    pub started_at: Instant,
    pub timeout_seconds: u64,
    pub container_id: String,
    pub agent_pid: Option<u32>,
}

/// Timeout watchdog service that monitors and enforces task timeouts
#[derive(Clone)]
pub struct TimeoutWatchdog {
    db: DatabaseConnection,
    active_tasks: Arc<Mutex<HashMap<i32, TaskExecution>>>,
    check_interval_seconds: u64,
    running: Arc<Mutex<bool>>,
}

impl TimeoutWatchdog {
    /// Create a new timeout watchdog service
    pub fn new(db: DatabaseConnection, check_interval_seconds: u64) -> Self {
        Self {
            db,
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            check_interval_seconds,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Register a task execution for timeout monitoring
    pub async fn register_task(&self, execution: TaskExecution) {
        let mut tasks = self.active_tasks.lock().await;
        info!(
            "Watchdog: Registering task {} with timeout {}s",
            execution.task_id, execution.timeout_seconds
        );
        tasks.insert(execution.task_id, execution);
    }

    /// Unregister a task execution (called when task completes normally)
    pub async fn unregister_task(&self, task_id: i32) {
        let mut tasks = self.active_tasks.lock().await;
        if tasks.remove(&task_id).is_some() {
            info!("Watchdog: Unregistered task {}", task_id);
        }
    }

    /// Main monitoring loop
    async fn monitor_loop(&self) {
        info!("Watchdog: Starting monitoring loop");
        
        loop {
            // Check if we should stop
            {
                let running = self.running.lock().await;
                if !*running {
                    info!("Watchdog: Stopping monitoring loop");
                    break;
                }
            }

            // Sleep for check interval
            tokio::time::sleep(tokio::time::Duration::from_secs(self.check_interval_seconds)).await;

            // Check all active tasks
            let tasks_to_check = {
                let tasks = self.active_tasks.lock().await;
                tasks.clone()
            };

            for (task_id, execution) in tasks_to_check.iter() {
                let elapsed = execution.started_at.elapsed().as_secs();
                
                if elapsed > execution.timeout_seconds {
                    warn!(
                        "Watchdog: Task {} exceeded timeout ({}s > {}s), killing agent",
                        task_id, elapsed, execution.timeout_seconds
                    );
                    
                    // Kill the agent process
                    if let Err(e) = self.kill_agent_process(&execution.container_id, execution.agent_pid).await {
                        error!("Watchdog: Failed to kill agent for task {}: {}", task_id, e);
                    }
                    
                    // Mark task as failed
                    if let Err(e) = self.fail_task_with_timeout(*task_id, elapsed).await {
                        error!("Watchdog: Failed to mark task {} as failed: {}", task_id, e);
                    }
                    
                    // Unregister the task
                    self.unregister_task(*task_id).await;
                } else {
                    debug!(
                        "Watchdog: Task {} running for {}s (timeout: {}s)",
                        task_id, elapsed, execution.timeout_seconds
                    );
                }
            }
        }
    }

    /// Kill the agent process in the container
    async fn kill_agent_process(&self, container_id: &str, agent_pid: Option<u32>) -> Result<()> {
        info!("Watchdog: Killing agent in container {}", container_id);
        
        // Try to kill specific PID if available
        if let Some(pid) = agent_pid {
            let kill_cmd = format!("kill -9 {}", pid);
            let output = tokio::process::Command::new("docker")
                .args(["exec", container_id, "sh", "-c", &kill_cmd])
                .output()
                .await
                .map_err(|e| VibeRepoError::Internal(format!("Failed to execute docker kill: {}", e)))?;
            
            if !output.status.success() {
                warn!(
                    "Watchdog: Failed to kill PID {} (may have already exited): {}",
                    pid,
                    String::from_utf8_lossy(&output.stderr)
                );
            } else {
                info!("Watchdog: Successfully killed agent PID {}", pid);
                return Ok(());
            }
        }
        
        // Fallback: kill all opencode processes
        let killall_cmd = "pkill -9 opencode || true";
        let output = tokio::process::Command::new("docker")
            .args(["exec", container_id, "sh", "-c", killall_cmd])
            .output()
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to execute docker killall: {}", e)))?;
        
        if output.status.success() {
            info!("Watchdog: Successfully killed all opencode processes");
            Ok(())
        } else {
            Err(VibeRepoError::Internal(format!(
                "Failed to kill agent processes: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Mark a task as failed due to timeout
    async fn fail_task_with_timeout(&self, task_id: i32, elapsed_seconds: u64) -> Result<()> {
        info!("Watchdog: Marking task {} as failed (timeout after {}s)", task_id, elapsed_seconds);
        
        let task = Task::find_by_id(task_id)
            .one(&self.db)
            .await
            .map_err(|e| VibeRepoError::Database(e))?
            .ok_or_else(|| VibeRepoError::NotFound(format!("Task {} not found", task_id)))?;

        let mut task_active: task::ActiveModel = task.into();
        task_active.task_status = Set(TaskStatus::Failed);
        task_active.last_log = Set(Some(format!(
            "Task execution timed out after {} seconds",
            elapsed_seconds
        )));
        task_active.updated_at = Set(chrono::Utc::now());

        task_active
            .update(&self.db)
            .await
            .map_err(|e| VibeRepoError::Database(e))?;

        info!("Watchdog: Task {} marked as failed", task_id);
        Ok(())
    }

    /// Get count of active tasks being monitored
    pub async fn active_task_count(&self) -> usize {
        let tasks = self.active_tasks.lock().await;
        tasks.len()
    }
}

#[async_trait]
impl BackgroundService for TimeoutWatchdog {
    fn name(&self) -> &'static str {
        "TimeoutWatchdog"
    }

    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        info!("Starting TimeoutWatchdog service");
        
        // Set running flag
        {
            let mut running = self.running.lock().await;
            *running = true;
        }
        
        // Spawn monitoring loop
        let watchdog = self.clone();
        tokio::spawn(async move {
            watchdog.monitor_loop().await;
        });
        
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!("Stopping TimeoutWatchdog service");
        
        // Clear running flag
        {
            let mut running = self.running.lock().await;
            *running = false;
        }
        
        // Clear all active tasks
        {
            let mut tasks = self.active_tasks.lock().await;
            tasks.clear();
        }
        
        Ok(())
    }

    async fn health_check(&self) -> bool {
        let running = self.running.lock().await;
        *running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_unregister_task() {
        let db = crate::test_utils::create_test_database().await.unwrap();
        let watchdog = TimeoutWatchdog::new(db, 10);

        let execution = TaskExecution {
            task_id: 1,
            started_at: Instant::now(),
            timeout_seconds: 60,
            container_id: "test-container".to_string(),
            agent_pid: Some(12345),
        };

        // Register task
        watchdog.register_task(execution).await;
        assert_eq!(watchdog.active_task_count().await, 1);

        // Unregister task
        watchdog.unregister_task(1).await;
        assert_eq!(watchdog.active_task_count().await, 0);
    }
}
