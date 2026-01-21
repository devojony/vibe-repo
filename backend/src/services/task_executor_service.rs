//! Task Executor Service
//!
//! Executes tasks in Docker containers using AI agents with concurrency control.

use crate::entities::{agent, prelude::*, task, workspace};
use crate::error::{Result, VibeRepoError};
use crate::services::{AgentService, TaskExecutionHistoryService, TaskService};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore};
use tracing::{error, info, warn};

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
    execution_service: TaskExecutionHistoryService,
    concurrency_manager: ConcurrencyManager,
}

impl TaskExecutorService {
    pub fn new(db: DatabaseConnection) -> Self {
        let task_service = TaskService::new(db.clone());
        let agent_service = AgentService::new(db.clone());
        let execution_service = TaskExecutionHistoryService::new(db.clone());
        Self {
            db,
            task_service,
            agent_service,
            execution_service,
            concurrency_manager: ConcurrencyManager::new(),
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
        if task.task_status != "assigned" && task.task_status != "pending" {
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
        let semaphore = self
            .concurrency_manager
            .get_semaphore(workspace.id, workspace.max_concurrent_tasks)
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
    async fn execute_task_internal(&self, task_id: i32, workspace: &workspace::Model) -> Result<()> {
        let task = self.task_service.get_task_by_id(task_id).await?;

        // Get agent if assigned
        let agent = if let Some(agent_id) = task.assigned_agent_id {
            Some(self.agent_service.get_agent_by_id(agent_id).await?)
        } else {
            // If no agent assigned, try to find a default enabled agent
            let agents = self
                .agent_service
                .list_agents_by_workspace(workspace.id)
                .await?;
            agents.into_iter().find(|a| a.enabled)
        };

        let agent = agent.ok_or_else(|| {
            VibeRepoError::NotFound(format!(
                "No enabled agent found for workspace {}",
                workspace.id
            ))
        })?;

        // Build command
        let command = self.build_execution_command(&agent, &task)?;

        // Create execution record
        let execution = self
            .execution_service
            .create_execution(task_id, Some(agent.id), command.clone())
            .await?;

        info!(
            task_id = task_id,
            execution_id = execution.id,
            "Created execution record"
        );

        // Update task status to running
        self.task_service.start_task(task_id).await?;

        // Execute task in container
        match self
            .execute_in_container(&workspace, &agent, &task, &command)
            .await
        {
            Ok(result) => {
                info!(
                    task_id = task_id,
                    execution_id = execution.id,
                    "Task execution completed successfully"
                );

                // Update execution record
                self.execution_service
                    .complete_execution(
                        execution.id,
                        result.exit_code,
                        result.stdout,
                        result.stderr,
                        result.pr_info.as_ref().map(|p| p.pr_number),
                        result.pr_info.as_ref().map(|p| p.pr_url.clone()),
                        result.pr_info.as_ref().map(|p| p.branch_name.clone()),
                    )
                    .await?;

                // Mark task as completed
                if let Some(pr_info) = result.pr_info {
                    self.task_service
                        .complete_task(
                            task_id,
                            pr_info.pr_number,
                            pr_info.pr_url,
                            pr_info.branch_name,
                        )
                        .await?;
                } else {
                    // If no PR was created, mark as failed
                    self.task_service
                        .fail_task(task_id, "Task completed but no PR was created".to_string())
                        .await?;
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    task_id = task_id,
                    execution_id = execution.id,
                    error = %e,
                    "Task execution failed"
                );

                // Update execution record as failed
                self.execution_service
                    .fail_execution(execution.id, e.to_string(), String::new(), String::new())
                    .await?;

                // Mark task as failed (will auto-retry if retries available)
                self.task_service.fail_task(task_id, e.to_string()).await?;

                Err(e)
            }
        }
    }

    /// Execute task in Docker container
    async fn execute_in_container(
        &self,
        workspace: &workspace::Model,
        agent: &agent::Model,
        task: &task::Model,
        command: &str,
    ) -> Result<ExecutionResult> {
        info!(
            workspace_id = workspace.id,
            agent_id = agent.id,
            task_id = task.id,
            "Executing task in container"
        );

        // Check if container exists
        if workspace.container_id.is_none() {
            return Err(VibeRepoError::Validation(
                "Workspace has no container".to_string(),
            ));
        }

        let container_id = workspace.container_id.as_ref().unwrap();

        info!(
            container_id = container_id,
            command = command,
            "Executing command in container"
        );

        // Execute command in container using docker exec
        let mut child = Command::new("docker")
            .args(["exec", "-i", container_id, "sh", "-c", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| VibeRepoError::Internal(format!("Failed to spawn docker exec: {}", e)))?;

        // Stream output
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| VibeRepoError::Internal("Failed to capture stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| VibeRepoError::Internal("Failed to capture stderr".to_string()))?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        // Read output lines
        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        while let Ok(Some(line)) = stdout_reader.next_line().await {
            info!(task_id = task.id, "STDOUT: {}", line);
            stdout_lines.push(line);
        }

        while let Ok(Some(line)) = stderr_reader.next_line().await {
            warn!(task_id = task.id, "STDERR: {}", line);
            stderr_lines.push(line);
        }

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to wait for process: {}", e)))?;

        let exit_code = status.code();

        // Parse output to extract PR information
        let pr_info = self.parse_pr_info(&stdout_lines)?;

        Ok(ExecutionResult {
            exit_code,
            stdout: stdout_lines.join("\n"),
            stderr: stderr_lines.join("\n"),
            pr_info,
        })
    }

    /// Build execution command for agent
    fn build_execution_command(&self, agent: &agent::Model, task: &task::Model) -> Result<String> {
        // Build environment variables
        let env_vars = if let Some(env_obj) = agent.env_vars.as_object() {
            env_obj
                .iter()
                .map(|(k, v)| format!("export {}='{}'", k, v.as_str().unwrap_or_default()))
                .collect::<Vec<_>>()
                .join(" && ")
        } else {
            String::new()
        };

        // Build task context
        let task_context = format!(
            "TASK_ID={} ISSUE_NUMBER={} ISSUE_TITLE='{}' ISSUE_BODY='{}'",
            task.id,
            task.issue_number,
            task.issue_title.replace('\'', "\\'"),
            task.issue_body
                .as_ref()
                .unwrap_or(&String::new())
                .replace('\'', "\\'")
        );

        // Combine everything
        let command = if !env_vars.is_empty() {
            format!("{} && {} && {}", env_vars, task_context, agent.command)
        } else {
            format!("{} && {}", task_context, agent.command)
        };

        Ok(command)
    }

    /// Parse PR information from command output
    fn parse_pr_info(&self, output_lines: &[String]) -> Result<Option<PrInfo>> {
        // Look for PR information in output
        // Expected format: "PR_NUMBER=123 PR_URL=https://... BRANCH_NAME=feature/..."
        for line in output_lines {
            if line.contains("PR_NUMBER=") && line.contains("PR_URL=") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut pr_number = None;
                let mut pr_url = None;
                let mut branch_name = None;

                for part in parts {
                    if let Some(num) = part.strip_prefix("PR_NUMBER=") {
                        pr_number = num.parse::<i32>().ok();
                    } else if let Some(url) = part.strip_prefix("PR_URL=") {
                        pr_url = Some(url.to_string());
                    } else if let Some(branch) = part.strip_prefix("BRANCH_NAME=") {
                        branch_name = Some(branch.to_string());
                    }
                }

                if let (Some(num), Some(url), Some(branch)) = (pr_number, pr_url, branch_name) {
                    return Ok(Some(PrInfo {
                        pr_number: num,
                        pr_url: url,
                        branch_name: branch,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Get next pending task for a workspace
    pub async fn get_next_pending_task(&self, workspace_id: i32) -> Result<Option<task::Model>> {
        let tasks = Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_id))
            .filter(task::Column::TaskStatus.eq("pending"))
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

struct ExecutionResult {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    pr_info: Option<PrInfo>,
}

struct PrInfo {
    pr_number: i32,
    pr_url: String,
    branch_name: String,
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
        let executor = TaskExecutorService::new(db.clone());

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
    async fn test_build_execution_command() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let workspace = create_test_workspace(db).await;
        let agent_service = AgentService::new(db.clone());
        let task_service = TaskService::new(db.clone());
        let executor = TaskExecutorService::new(db.clone());

        let agent = agent_service
            .create_agent(
                workspace.id,
                "Test Agent",
                "opencode",
                "opencode solve-issue",
                json!({"API_KEY": "test-key"}),
                1800,
            )
            .await
            .unwrap();

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

        // Act
        let command = executor.build_execution_command(&agent, &task);

        // Assert
        assert!(command.is_ok());
        let cmd = command.unwrap();
        assert!(cmd.contains("API_KEY='test-key'"));
        assert!(cmd.contains("TASK_ID="));
        assert!(cmd.contains("ISSUE_NUMBER=201"));
        assert!(cmd.contains("opencode solve-issue"));
    }

    #[tokio::test]
    async fn test_parse_pr_info_success() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let executor = TaskExecutorService::new(db.clone());

        let output = vec![
            "Starting task execution...".to_string(),
            "PR_NUMBER=123 PR_URL=https://git.example.com/owner/repo/pulls/123 BRANCH_NAME=feature/test".to_string(),
            "Task completed".to_string(),
        ];

        // Act
        let result = executor.parse_pr_info(&output);

        // Assert
        assert!(result.is_ok());
        let pr_info = result.unwrap();
        assert!(pr_info.is_some());
        let info = pr_info.unwrap();
        assert_eq!(info.pr_number, 123);
        assert_eq!(info.pr_url, "https://git.example.com/owner/repo/pulls/123");
        assert_eq!(info.branch_name, "feature/test");
    }

    #[tokio::test]
    async fn test_parse_pr_info_not_found() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let executor = TaskExecutorService::new(db.clone());

        let output = vec![
            "Starting task execution...".to_string(),
            "Task completed".to_string(),
        ];

        // Act
        let result = executor.parse_pr_info(&output);

        // Assert
        assert!(result.is_ok());
        let pr_info = result.unwrap();
        assert!(pr_info.is_none());
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
        let executor = TaskExecutorService::new(db.clone());

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
        let executor = TaskExecutorService::new(db.clone());

        // Initialize semaphore
        let _sem = executor
            .concurrency_manager
            .get_semaphore(1, 3)
            .await;

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
            workspace_status: Set("Active".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
            ..Default::default()
        };
        Workspace::insert(ws).exec_with_returning(db).await.unwrap()
    }

    async fn create_test_repository(db: &DatabaseConnection) -> repository::Model {
        use crate::entities::repo_provider;

        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec_with_returning(db)
            .await
            .unwrap();

        let repo = repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.id),
            ..Default::default()
        };
        Repository::insert(repo)
            .exec_with_returning(db)
            .await
            .unwrap()
    }
}
