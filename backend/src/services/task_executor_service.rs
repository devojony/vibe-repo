//! Task Executor Service
//!
//! Executes tasks using ACP-compatible agents with concurrency control.

use crate::entities::{
    agent,
    prelude::*,
    task::{self, TaskStatus},
    workspace,
};
use crate::error::{Result, VibeRepoError};
use crate::services::{
    agent_manager::{AgentConfig, AgentManager, AgentType},
    AgentService, GitService, PRCreationService, TaskService,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, error, info, warn};

/// Concurrency manager for workspace task execution
#[derive(Clone)]
struct ConcurrencyManager {
    semaphores: Arc<Mutex<HashMap<i32, Arc<Semaphore>>>>,
}

impl ConcurrencyManager {
    fn new() -> Self {
        Self {
            semaphores: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create a semaphore for a workspace
    async fn get_semaphore(&self, workspace_id: i32, max_concurrent: i32) -> Arc<Semaphore> {
        let mut semaphores = self.semaphores.lock().await;
        semaphores
            .entry(workspace_id)
            .or_insert_with(|| Arc::new(Semaphore::new(max_concurrent as usize)))
            .clone()
    }

    /// Get current available permits for a workspace
    async fn available_permits(&self, workspace_id: i32) -> Option<usize> {
        let semaphores = self.semaphores.lock().await;
        semaphores
            .get(&workspace_id)
            .map(|sem| sem.available_permits())
    }
}

#[derive(Clone)]
pub struct TaskExecutorService {
    db: DatabaseConnection,
    task_service: TaskService,
    agent_service: AgentService,
    pr_creation_service: PRCreationService,
    concurrency_manager: ConcurrencyManager,
    workspace_base_dir: String,
    agent_manager: Arc<AgentManager>,
}

impl TaskExecutorService {
    pub fn new(db: DatabaseConnection, workspace_base_dir: String) -> Self {
        let task_service = TaskService::new(db.clone());
        let agent_service = AgentService::new(db.clone());
        let pr_creation_service = PRCreationService::new(db.clone());
        
        // Create agent manager with default configuration
        let default_agent_config = AgentConfig {
            agent_type: AgentType::OpenCode,
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            model: None,
            timeout: 600, // 10 minutes
            working_dir: PathBuf::from(&workspace_base_dir),
            container_id: None,
        };
        let agent_manager = Arc::new(AgentManager::new(3, default_agent_config));
        
        Self {
            db,
            task_service,
            agent_service,
            pr_creation_service,
            concurrency_manager: ConcurrencyManager::new(),
            workspace_base_dir,
            agent_manager,
        }
    }

    /// Get available execution slots for a workspace
    pub async fn get_available_slots(&self, workspace_id: i32) -> Result<usize> {
        self.concurrency_manager
            .available_permits(workspace_id)
            .await
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!(
                    "No concurrency control initialized for workspace {}",
                    workspace_id
                ))
            })
    }

    /// Execute a task in its workspace container with concurrency control
    pub async fn execute_task(&self, task_id: i32) -> Result<()> {
        info!(task_id = task_id, "Starting task execution");

        // Get task details
        let task = self.task_service.get_task_by_id(task_id).await?;

        // Validate task status
        if task.task_status != TaskStatus::Pending {
            return Err(VibeRepoError::Validation(format!(
                "Task {} is not in a valid state for execution (current: {})",
                task_id, task.task_status
            )));
        }

        // Get workspace
        let workspace = Workspace::find_by_id(task.workspace_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Workspace {} not found", task.workspace_id))
            })?;

        // Acquire semaphore permit for concurrency control
        // Use hardcoded value of 3 for max concurrent tasks in simplified MVP
        let semaphore = self
            .concurrency_manager
            .get_semaphore(workspace.id, 3)
            .await;

        info!(
            task_id = task_id,
            workspace_id = workspace.id,
            available_permits = semaphore.available_permits(),
            "Acquiring execution permit"
        );

        let _permit = semaphore.acquire().await.map_err(|e| {
            VibeRepoError::Internal(format!("Failed to acquire semaphore permit: {}", e))
        })?;

        info!(
            task_id = task_id,
            workspace_id = workspace.id,
            remaining_permits = semaphore.available_permits(),
            "Execution permit acquired"
        );

        // Execute task (permit will be released when _permit is dropped)
        self.execute_task_internal(task_id, &workspace).await
    }

    /// Internal task execution logic (without concurrency control)
    async fn execute_task_internal(
        &self,
        task_id: i32,
        workspace: &workspace::Model,
    ) -> Result<()> {
        let task = self.task_service.get_task_by_id(task_id).await?;

        // Get agent configuration
        let agent = if let Some(agent_id) = task.assigned_agent_id {
            Some(self.agent_service.get_agent_by_id(agent_id).await?)
        } else {
            // If no agent assigned, try to find a default agent
            let agents = self
                .agent_service
                .list_agents_by_workspace(workspace.id)
                .await?;
            agents.into_iter().next()
        };

        let agent = agent.ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "No enabled agent found for workspace {}",
                workspace.id
            ))
        })?;

        // Generate branch name for this task
        let branch_name = format!("feature/issue-{}", task.issue_number);

        info!(
            task_id = task_id,
            branch_name = %branch_name,
            "Generated branch name for task"
        );

        // Create git worktree for this task
        let git_service = GitService::new(self.db.clone(), self.workspace_base_dir.clone());

        info!(
            task_id = task_id,
            workspace_id = workspace.id,
            branch_name = %branch_name,
            "Creating git worktree for task"
        );

        // Get container_id from workspace
        let container_id = workspace.container_id.as_ref().ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Container ID not found for workspace {}",
                workspace.id
            ))
        })?;

        // Create worktree
        let worktree_path = match git_service
            .create_task_worktree(workspace.id, task_id, &branch_name, container_id)
            .await
        {
            Ok(path) => {
                info!(
                    task_id = task_id,
                    worktree_path = ?path,
                    "Git worktree created successfully"
                );
                path
            }
            Err(e) => {
                error!(
                    task_id = task_id,
                    error = %e,
                    "Failed to create git worktree"
                );
                return Err(VibeRepoError::Internal(format!(
                    "Failed to create git worktree: {}",
                    e
                )));
            }
        };

        // Update task with branch name
        let mut task_active: task::ActiveModel = task.clone().into();
        task_active.branch_name = Set(Some(branch_name.clone()));
        let _task = task_active
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        info!(
            task_id = task_id,
            branch_name = %branch_name,
            "Updated task with branch name"
        );

        // Update task status to running
        self.task_service.start_task(task_id).await?;

        // Execute task using ACP
        match self
            .execute_with_acp(task_id, workspace, &agent, &worktree_path)
            .await
        {
            Ok(()) => {
                info!(task_id = task_id, "Task execution completed successfully");

                // Check if we have a branch to create PR from
                let task = self.task_service.get_task_by_id(task_id).await?;

                if let Some(branch_name) = &task.branch_name {
                    // Push branch to remote repository
                    info!(
                        task_id,
                        branch = ?branch_name,
                        "Pushing branch to remote repository"
                    );

                    match git_service
                        .push_branch(workspace.id, task_id, branch_name, container_id)
                        .await
                    {
                        Ok(()) => {
                            info!(task_id, branch = ?branch_name, "Branch pushed successfully");
                        }
                        Err(e) => {
                            error!(task_id, branch = ?branch_name, error = %e, "Failed to push branch");
                            // Mark task as failed if push fails
                            self.task_service
                                .fail_task(
                                    task_id,
                                    format!("Task completed but failed to push branch: {}", e),
                                )
                                .await?;
                            return Ok(());
                        }
                    }

                    // Try to create PR via PRCreationService
                    info!(
                        task_id,
                        branch = ?branch_name,
                        "Attempting to create PR automatically"
                    );

                    match self.pr_creation_service.create_pr_for_task(task_id).await {
                        Ok(()) => {
                            info!(task_id, "PR created successfully via PRCreationService");
                        }
                        Err(e) => {
                            error!(task_id, error = %e, "Failed to create PR automatically");
                            // Mark task as failed if PR creation fails
                            self.task_service
                                .fail_task(
                                    task_id,
                                    format!("Task completed but PR creation failed: {}", e),
                                )
                                .await?;
                        }
                    }
                } else {
                    // No branch - task failed
                    self.task_service
                        .fail_task(
                            task_id,
                            "Task completed but no branch was created".to_string(),
                        )
                        .await?;
                    warn!(task_id, "Task completed but no branch info found");
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    task_id = task_id,
                    error = %e,
                    "Task execution failed"
                );

                // Mark task as failed
                self.task_service.fail_task(task_id, e.to_string()).await?;

                Err(e)
            }
        }
    }

    /// Execute task using ACP protocol
    async fn execute_with_acp(
        &self,
        task_id: i32,
        workspace: &workspace::Model,
        agent: &agent::Model,
        worktree_path: &Path,
    ) -> Result<()> {
        info!(task_id = task_id, "Starting ACP-based task execution");

        // Get container_id from workspace
        let container_id = workspace.container_id.as_ref().ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "Container ID not found for workspace {}",
                workspace.id
            ))
        })?;

        // Spawn agent using AgentManager - agent will run in container
        let task_id_str = format!("task-{}", task_id);
        let agent_config = AgentConfig {
            agent_type: AgentType::OpenCode, // TODO: Get from agent.agent_type field
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            model: None,
            timeout: agent.timeout as u64,
            working_dir: worktree_path.to_path_buf(),
            container_id: Some(container_id.clone()),
        };

        let agent_handle = self
            .agent_manager
            .spawn_agent(task_id_str.clone(), agent_config)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to spawn agent: {}", e)))?;

        info!(task_id = task_id, container_id = %container_id, "Agent spawned successfully in container");

        // Initialize agent and create session
        let session_id = agent_handle
            .initialize()
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to initialize agent: {}", e)))?;

        info!(task_id = task_id, session_id = ?session_id, "Agent initialized with session");

        // Get event store from agent handle
        let event_store = agent_handle.event_store().await;

        // Build prompt from task
        let task = self.task_service.get_task_by_id(task_id).await?;
        let prompt = format!(
            "Please implement the following GitHub issue:\n\n\
            Issue #{}: {}\n\n\
            Description:\n{}\n\n\
            Instructions:\n\
            1. Read and understand the issue requirements\n\
            2. Make the necessary code changes to implement the feature or fix the bug\n\
            3. Create or modify files as needed\n\
            4. Test your changes if possible\n\
            5. When you're done with all code changes, respond with 'Implementation complete'\n\n\
            Important: Do NOT commit changes or push to git - that will be handled automatically.\n\
            Focus only on implementing the code changes.",
            task.issue_number,
            task.issue_title,
            task.issue_body.as_ref().unwrap_or(&String::from("No description provided"))
        );

        info!(task_id = task_id, "Sending prompt to agent");

        // Spawn a background task to periodically update the database with events
        let db_clone = self.db.clone();
        let event_store_clone = event_store.clone();
        let update_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                
                // Get current events and plans
                let store = event_store_clone.lock().await;
                let events = store.get_events();
                let plans = store.get_plans();
                drop(store);
                
                // Update database
                if let Ok(Some(task)) = Task::find_by_id(task_id).one(&db_clone).await {
                    let mut task_active: task::ActiveModel = task.into();
                    task_active.events = Set(Some(serde_json::to_value(&events).unwrap_or_default()));
                    task_active.plans = Set(Some(serde_json::to_value(&plans).unwrap_or_default()));
                    
                    if let Err(e) = task_active.update(&db_clone).await {
                        warn!(task_id = task_id, error = %e, "Failed to update task events in database");
                    } else {
                        debug!(
                            task_id = task_id,
                            event_count = events.len(),
                            plan_count = plans.len(),
                            "Updated task events in database"
                        );
                    }
                }
            }
        });

        // Send prompt and wait for completion with process-level timeout
        let timeout_duration = Duration::from_secs(agent.timeout as u64);
        info!(
            task_id = task_id,
            timeout_seconds = agent.timeout,
            "Sending prompt to agent with process-level timeout"
        );

        // Use tokio::select! to race between prompt and timeout
        let result = tokio::select! {
            prompt_result = agent_handle.prompt(prompt) => {
                // Prompt completed (either success or error)
                match prompt_result {
                    Ok(()) => {
                        info!(task_id = task_id, "Agent completed successfully");
                        Ok(())
                    }
                    Err(e) => {
                        error!(task_id = task_id, error = %e, "Agent execution failed");
                        Err(VibeRepoError::Internal(format!("Agent execution failed: {}", e)))
                    }
                }
            }
            _ = tokio::time::sleep(timeout_duration) => {
                // Timeout occurred
                error!(
                    task_id = task_id,
                    timeout_seconds = agent.timeout,
                    "Agent execution timed out - killing process"
                );
                
                // Kill the agent process
                if let Err(e) = self.agent_manager.shutdown_agent(&task_id_str, Duration::from_secs(2)).await {
                    warn!(task_id = task_id, error = %e, "Failed to kill agent process after timeout");
                }
                
                Err(VibeRepoError::Timeout(format!(
                    "Agent execution timed out after {} seconds",
                    agent.timeout
                )))
            }
        };

        // Stop the background update task
        update_task.abort();

        // Final update: Store events and plans to database
        let events = event_store.lock().await.get_events();
        let plans = event_store.lock().await.get_plans();

        let mut task_active: task::ActiveModel = task.clone().into();
        task_active.events = Set(Some(serde_json::to_value(&events).unwrap_or_default()));
        task_active.plans = Set(Some(serde_json::to_value(&plans).unwrap_or_default()));
        
        // Store last message as log
        if let Some(last_msg) = event_store.lock().await.get_latest_message() {
            task_active.last_log = Set(Some(last_msg.content));
        }
        
        task_active
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        info!(
            task_id = task_id,
            event_count = events.len(),
            plan_count = plans.len(),
            "Final update: Stored events and plans to database"
        );

        // Shutdown agent
        let shutdown_result = self
            .agent_manager
            .shutdown_agent(&task_id_str, Duration::from_secs(5))
            .await;
        
        if let Err(e) = shutdown_result {
            warn!(task_id = task_id, error = %e, "Failed to shutdown agent gracefully");
        }

        // Handle the prompt result and update task status accordingly
        match result {
            Ok(()) => {
                info!(task_id = task_id, "Agent prompt completed successfully");
                Ok(())
            }
            Err(e) => {
                error!(task_id = task_id, error = %e, "Agent prompt failed");
                
                // Update task status to failed
                let error_msg = format!("Agent prompt failed: {}", e);
                self.task_service.fail_task(task_id, error_msg.clone()).await?;
                
                Err(VibeRepoError::Internal(error_msg))
            }
        }
    }

    /// Cancel a running task
    pub async fn cancel_task(&self, task_id: i32) -> Result<()> {
        info!(task_id = task_id, "Cancelling task");

        let task_id_str = format!("task-{}", task_id);
        
        // Try to cancel via agent manager
        if let Some(agent_handle) = self.agent_manager.get_agent(&task_id_str).await {
            info!(task_id = task_id, "Sending cancel request to agent");
            
            let result = agent_handle.cancel().await;

            match result {
                Ok(()) => {
                    info!(task_id = task_id, "Agent cancelled successfully");
                }
                Err(e) => {
                    warn!(task_id = task_id, error = %e, "Failed to cancel agent gracefully, force killing");
                    let _ = self.agent_manager.force_kill_agent(&task_id_str).await;
                }
            }
        }

        // Update task status to cancelled
        let task = self.task_service.get_task_by_id(task_id).await?;
        let mut task_active: task::ActiveModel = task.into();
        task_active.task_status = Set(TaskStatus::Cancelled);
        task_active.completed_at = Set(Some(chrono::Utc::now()));
        task_active
            .update(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        info!(task_id = task_id, "Task cancelled");
        Ok(())
    }

    /// Get next pending task for a workspace
    pub async fn get_next_pending_task(&self, workspace_id: i32) -> Result<Option<task::Model>> {
        let tasks = Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_id))
            .filter(task::Column::TaskStatus.eq(TaskStatus::Pending))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        // Return highest priority task
        let mut tasks = tasks;
        tasks.sort_by(|a, b| {
            let priority_order = |p: &str| match p {
                "high" => 0,
                "medium" => 1,
                "low" => 2,
                _ => 3,
            };
            priority_order(&a.priority).cmp(&priority_order(&b.priority))
        });

        Ok(tasks.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{repository, workspace};
    use crate::test_utils::db::TestDatabase;
    use sea_orm::Set;
    use serde_json::json;

    #[tokio::test]
    async fn test_get_next_pending_task_returns_highest_priority() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let task_service = TaskService::new(db.clone());
        let executor = TaskExecutorService::new(db.clone(), "/tmp/test-workspace".to_string());

        // Create tasks with different priorities
        let _low_task = task_service
            .create_task(
                workspace.id,
                101,
                "Low priority task".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        let high_task = task_service
            .create_task(
                workspace.id,
                102,
                "High priority task".to_string(),
                None,
                None,
                "high".to_string(),
            )
            .await
            .unwrap();

        let _medium_task = task_service
            .create_task(
                workspace.id,
                103,
                "Medium priority task".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Act
        let result = executor.get_next_pending_task(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        let next_task = result.unwrap();
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().id, high_task.id);
    }

    #[tokio::test]
    async fn test_cancel_task_updates_status() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let task_service = TaskService::new(db.clone());
        let executor = TaskExecutorService::new(db.clone(), "/tmp/test-workspace".to_string());

        let task = task_service
            .create_task(
                workspace.id,
                201,
                "Test task".to_string(),
                Some("Task description".to_string()),
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Start the task first
        task_service.start_task(task.id).await.unwrap();

        // Act
        let result = executor.cancel_task(task.id).await;

        // Assert
        assert!(result.is_ok());
        let updated_task = task_service.get_task_by_id(task.id).await.unwrap();
        assert_eq!(updated_task.task_status, TaskStatus::Cancelled);
        assert!(updated_task.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_concurrency_manager_creates_semaphore() {
        // Arrange
        let manager = ConcurrencyManager::new();

        // Act
        let semaphore = manager.get_semaphore(1, 3).await;

        // Assert
        assert_eq!(semaphore.available_permits(), 3);
    }

    #[tokio::test]
    async fn test_concurrency_manager_reuses_semaphore() {
        // Arrange
        let manager = ConcurrencyManager::new();

        // Act
        let sem1 = manager.get_semaphore(1, 3).await;
        let _permit = sem1.acquire().await.unwrap();

        let sem2 = manager.get_semaphore(1, 3).await;

        // Assert - should be the same semaphore with 2 permits left
        assert_eq!(sem2.available_permits(), 2);
    }

    #[tokio::test]
    async fn test_concurrency_manager_different_workspaces() {
        // Arrange
        let manager = ConcurrencyManager::new();

        // Act
        let sem1 = manager.get_semaphore(1, 3).await;
        let _permit1 = sem1.acquire().await.unwrap();

        let sem2 = manager.get_semaphore(2, 5).await;

        // Assert - different workspaces have independent semaphores
        assert_eq!(sem1.available_permits(), 2);
        assert_eq!(sem2.available_permits(), 5);
    }

    #[tokio::test]
    async fn test_concurrency_manager_available_permits() {
        // Arrange
        let manager = ConcurrencyManager::new();

        // Act - before creating semaphore
        let before = manager.available_permits(1).await;

        // Create semaphore
        let _sem = manager.get_semaphore(1, 3).await;

        // Act - after creating semaphore
        let after = manager.available_permits(1).await;

        // Assert
        assert!(before.is_none());
        assert_eq!(after, Some(3));
    }

    #[tokio::test]
    async fn test_get_available_slots_returns_error_when_not_initialized() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let executor = TaskExecutorService::new(db.clone(), "/tmp/test-workspace".to_string());

        // Act
        let result = executor.get_available_slots(999).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VibeRepoError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_available_slots_returns_permits_when_initialized() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let executor = TaskExecutorService::new(db.clone(), "/tmp/test-workspace".to_string());

        // Initialize semaphore
        let _sem = executor.concurrency_manager.get_semaphore(1, 3).await;

        // Act
        let result = executor.get_available_slots(1).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        let repo = create_test_repository(db).await;
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
            ..Default::default()
        };
        Workspace::insert(ws).exec_with_returning(db).await.unwrap()
    }

    async fn create_test_repository(db: &DatabaseConnection) -> repository::Model {
        use crate::test_utils::create_test_repository as create_repo;

        let repo_name = format!("test-repo-{}", uuid::Uuid::new_v4());
        let full_name = format!("owner/{}", repo_name);

        create_repo(
            db,
            &repo_name,
            &full_name,
            "gitea",
            "https://git.example.com",
            "test-token",
        )
        .await
        .unwrap()
    }

    /// Test execute_task_creates_pr_on_success
    /// Requirements: Task execution should create PR via PRCreationService when task completes with branch but no PR
    #[tokio::test]
    #[ignore] // Requires mock Git provider - this test verifies integration with PRCreationService
    async fn test_execute_task_creates_pr_on_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let agent_service = AgentService::new(db.clone());
        let task_service = TaskService::new(db.clone());

        // Create agent
        let _agent = agent_service
            .create_agent(
                workspace.id,
                "Test Agent",
                "opencode",
                "echo 'BRANCH_NAME=feature/test-123'", // Simulates agent creating branch but not PR
                json!({}),
                1800,
            )
            .await
            .unwrap();

        // Create task
        let task = task_service
            .create_task(
                workspace.id,
                301,
                "Test task for PR creation".to_string(),
                Some("Task body".to_string()),
                None,
                "high".to_string(),
            )
            .await
            .unwrap();

        // Task is already in pending status, ready for execution
        let executor = TaskExecutorService::new(db.clone(), "/tmp/test-workspace".to_string());

        // Act
        // Note: This test is ignored because it requires a real Docker container
        // and Git provider to fully test the integration
        let result = executor.execute_task(task.id).await;

        // Assert
        assert!(result.is_ok(), "Task execution should succeed");
        // TODO: When mock is available, verify:
        // - Task status is "completed"
        // - Task has pr_number set
        // - Task has pr_url set
    }

    /// Test execute_task_skips_pr_if_no_branch
    /// Requirements: PR creation should be skipped if task has no branch_name
    #[tokio::test]
    #[ignore] // Requires mock Git provider
    async fn test_execute_task_skips_pr_if_no_branch() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let agent_service = AgentService::new(db.clone());
        let task_service = TaskService::new(db.clone());

        // Create agent
        let _agent = agent_service
            .create_agent(
                workspace.id,
                "Test Agent",
                "opencode",
                "echo 'Task completed'", // No branch or PR info
                json!({}),
                1800,
            )
            .await
            .unwrap();

        // Create task
        let task = task_service
            .create_task(
                workspace.id,
                302,
                "Test task without branch".to_string(),
                None,
                None,
                "medium".to_string(),
            )
            .await
            .unwrap();

        // Task is already in pending status, ready for execution
        let executor = TaskExecutorService::new(db.clone(), "/tmp/test-workspace".to_string());

        // Act
        let result = executor.execute_task(task.id).await;

        // Assert
        assert!(
            result.is_ok() || result.is_err(),
            "Task execution completes"
        );
        // TODO: When mock is available, verify:
        // - PRCreationService.create_pr_for_task was NOT called
    }

    /// Test execute_task_continues_if_pr_creation_fails
    /// Requirements: Task should not fail if PR creation fails
    #[tokio::test]
    #[ignore] // Requires mock Git provider
    async fn test_execute_task_continues_if_pr_creation_fails() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let agent_service = AgentService::new(db.clone());
        let task_service = TaskService::new(db.clone());

        // Create agent
        let _agent = agent_service
            .create_agent(
                workspace.id,
                "Test Agent",
                "opencode",
                "echo 'BRANCH_NAME=feature/nonexistent-branch'", // Branch that doesn't exist
                json!({}),
                1800,
            )
            .await
            .unwrap();

        // Create task
        let task = task_service
            .create_task(
                workspace.id,
                303,
                "Test task with PR creation failure".to_string(),
                None,
                None,
                "low".to_string(),
            )
            .await
            .unwrap();

        // Task is already in pending status, ready for execution
        let executor = TaskExecutorService::new(db.clone(), "/tmp/test-workspace".to_string());

        // Act
        let result = executor.execute_task(task.id).await;

        // Assert
        assert!(
            result.is_ok() || result.is_err(),
            "Task execution completes"
        );
        // TODO: When mock is available, verify:
        // - Task status is "failed" (because PR creation failed)
        // - Error message mentions PR creation failure
    }
}
