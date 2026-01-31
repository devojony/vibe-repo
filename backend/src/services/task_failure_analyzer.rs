//! Task Failure Analyzer Service
//!
//! Analyzes task failures and provides actionable recommendations.

use crate::entities::{prelude::*, task::{self, TaskStatus}, task_execution};
use crate::error::{Result, VibeRepoError};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailureAnalysis {
    pub task_id: i32,
    pub failure_category: FailureCategory,
    pub root_cause: String,
    pub recommendations: Vec<String>,
    pub similar_failures_count: i32,
    pub is_recurring: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FailureCategory {
    /// Container or Docker related issues
    ContainerError,
    /// Agent command or configuration issues
    AgentError,
    /// Git operations failed
    GitError,
    /// Build or compilation errors
    BuildError,
    /// Test failures
    TestError,
    /// Timeout exceeded
    Timeout,
    /// Permission or access issues
    PermissionError,
    /// Network or connectivity issues
    NetworkError,
    /// Unknown or unclassified error
    Unknown,
}

#[derive(Clone)]
pub struct TaskFailureAnalyzer {
    db: DatabaseConnection,
}

impl TaskFailureAnalyzer {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Analyze a failed task and provide recommendations
    pub async fn analyze_failure(&self, task_id: i32) -> Result<FailureAnalysis> {
        // Get task
        let task = Task::find_by_id(task_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| VibeRepoError::NotFound(format!("Task {} not found", task_id)))?;

        // Get latest execution
        let execution = TaskExecution::find()
            .filter(task_execution::Column::TaskId.eq(task_id))
            .order_by_desc(task_execution::Column::StartedAt)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        // Analyze error message and output
        let (category, root_cause) = if let Some(exec) = &execution {
            self.categorize_failure(exec)
        } else {
            (
                FailureCategory::Unknown,
                task.error_message
                    .clone()
                    .unwrap_or_else(|| "No error message available".to_string()),
            )
        };

        // Generate recommendations
        let recommendations = self.generate_recommendations(&category, &root_cause, &execution);

        // Check for similar failures
        let similar_failures_count = self
            .count_similar_failures(task.workspace_id, &category)
            .await?;

        // Check if this is a recurring failure
        let is_recurring = self.is_recurring_failure(task_id).await?;

        Ok(FailureAnalysis {
            task_id,
            failure_category: category,
            root_cause,
            recommendations,
            similar_failures_count,
            is_recurring,
        })
    }

    /// Categorize failure based on error messages and output
    fn categorize_failure(&self, execution: &task_execution::Model) -> (FailureCategory, String) {
        let error_msg = execution
            .error_message
            .as_ref()
            .unwrap_or(&String::new())
            .to_lowercase();
        let stderr = execution
            .stderr_summary
            .as_ref()
            .unwrap_or(&String::new())
            .to_lowercase();
        let combined = format!("{} {}", error_msg, stderr);

        // Container errors
        if combined.contains("container")
            || combined.contains("docker")
            || combined.contains("no such container")
        {
            return (
                FailureCategory::ContainerError,
                "Container is not running or not accessible".to_string(),
            );
        }

        // Agent errors
        if combined.contains("agent")
            || combined.contains("command not found")
            || combined.contains("no such file or directory")
        {
            return (
                FailureCategory::AgentError,
                "Agent command failed or not found".to_string(),
            );
        }

        // Git errors
        if combined.contains("git")
            || combined.contains("repository")
            || combined.contains("clone")
            || combined.contains("push")
            || combined.contains("pull")
        {
            return (
                FailureCategory::GitError,
                "Git operation failed".to_string(),
            );
        }

        // Build errors
        if combined.contains("build")
            || combined.contains("compile")
            || combined.contains("cargo")
            || combined.contains("npm")
            || combined.contains("make")
        {
            return (
                FailureCategory::BuildError,
                "Build or compilation failed".to_string(),
            );
        }

        // Test errors
        if combined.contains("test")
            || combined.contains("assertion")
            || combined.contains("expected")
        {
            return (FailureCategory::TestError, "Tests failed".to_string());
        }

        // Timeout
        if combined.contains("timeout") || combined.contains("timed out") {
            return (
                FailureCategory::Timeout,
                "Execution exceeded timeout limit".to_string(),
            );
        }

        // Permission errors
        if combined.contains("permission")
            || combined.contains("denied")
            || combined.contains("forbidden")
            || combined.contains("unauthorized")
        {
            return (
                FailureCategory::PermissionError,
                "Permission or access denied".to_string(),
            );
        }

        // Network errors
        if combined.contains("network")
            || combined.contains("connection")
            || combined.contains("timeout")
            || combined.contains("unreachable")
        {
            return (
                FailureCategory::NetworkError,
                "Network connectivity issue".to_string(),
            );
        }

        // Unknown
        (
            FailureCategory::Unknown,
            execution
                .error_message
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string()),
        )
    }

    /// Generate recommendations based on failure category
    fn generate_recommendations(
        &self,
        category: &FailureCategory,
        _root_cause: &str,
        execution: &Option<task_execution::Model>,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        match category {
            FailureCategory::ContainerError => {
                recommendations.push("Check if the workspace container is running".to_string());
                recommendations.push(
                    "Restart the container using POST /api/workspaces/:id/restart".to_string(),
                );
                recommendations.push("Verify Docker daemon is running".to_string());
                recommendations.push("Check container logs for startup errors".to_string());
            }
            FailureCategory::AgentError => {
                recommendations.push("Verify agent command is correct and executable".to_string());
                recommendations
                    .push("Check if required tools are installed in the container".to_string());
                recommendations
                    .push("Review agent configuration and environment variables".to_string());
                recommendations.push("Test agent command manually in the container".to_string());
            }
            FailureCategory::GitError => {
                recommendations.push("Verify Git credentials and access token".to_string());
                recommendations.push("Check repository permissions".to_string());
                recommendations.push("Ensure Git is configured in the container".to_string());
                recommendations.push("Verify branch names and remote URLs".to_string());
            }
            FailureCategory::BuildError => {
                recommendations.push("Review build logs for specific error messages".to_string());
                recommendations.push("Check if all dependencies are installed".to_string());
                recommendations.push("Verify build configuration files".to_string());
                recommendations.push("Try building manually to reproduce the issue".to_string());
            }
            FailureCategory::TestError => {
                recommendations.push("Review test failure details in execution logs".to_string());
                recommendations.push("Run tests locally to reproduce the issue".to_string());
                recommendations.push("Check if test data or fixtures are correct".to_string());
                recommendations.push("Verify test environment configuration".to_string());
            }
            FailureCategory::Timeout => {
                recommendations.push("Increase agent timeout setting".to_string());
                recommendations.push("Optimize task to reduce execution time".to_string());
                recommendations.push("Check for infinite loops or blocking operations".to_string());
                recommendations.push("Consider breaking task into smaller subtasks".to_string());
            }
            FailureCategory::PermissionError => {
                recommendations.push("Verify access token has required permissions".to_string());
                recommendations.push("Check repository access settings".to_string());
                recommendations.push("Ensure agent has write permissions".to_string());
                recommendations.push("Review organization or team permissions".to_string());
            }
            FailureCategory::NetworkError => {
                recommendations.push("Check network connectivity from container".to_string());
                recommendations.push("Verify firewall rules and proxy settings".to_string());
                recommendations.push("Check if external services are accessible".to_string());
                recommendations
                    .push("Retry the task after network issues are resolved".to_string());
            }
            FailureCategory::Unknown => {
                recommendations.push("Review full execution logs for details".to_string());
                recommendations.push("Check stderr output for error messages".to_string());
                recommendations.push("Enable debug logging for more information".to_string());
                recommendations.push("Contact support if issue persists".to_string());
            }
        }

        // Add execution-specific recommendations
        if let Some(exec) = execution {
            if let Some(exit_code) = exec.exit_code {
                if exit_code != 0 {
                    recommendations.push(format!(
                        "Process exited with code {}. Check documentation for this exit code.",
                        exit_code
                    ));
                }
            }

            if let Some(duration_ms) = exec.duration_ms {
                let duration_sec = duration_ms / 1000;
                if duration_sec > 1800 {
                    // > 30 minutes
                    recommendations.push(
                        "Task took over 30 minutes. Consider optimizing or splitting the task."
                            .to_string(),
                    );
                }
            }
        }

        recommendations
    }

    /// Count similar failures in the same workspace
    async fn count_similar_failures(
        &self,
        workspace_id: i32,
        category: &FailureCategory,
    ) -> Result<i32> {
        // Get all failed tasks in workspace
        let tasks = Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_id))
            .filter(task::Column::TaskStatus.eq("failed"))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        let mut count = 0;
        for task in tasks {
            // Get latest execution for each task
            if let Ok(Some(exec)) = TaskExecution::find()
                .filter(task_execution::Column::TaskId.eq(task.id))
                .order_by_desc(task_execution::Column::StartedAt)
                .one(&self.db)
                .await
            {
                let (exec_category, _) = self.categorize_failure(&exec);
                if exec_category == *category {
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Check if this task has failed multiple times
    async fn is_recurring_failure(&self, task_id: i32) -> Result<bool> {
        let executions = TaskExecution::find()
            .filter(task_execution::Column::TaskId.eq(task_id))
            .filter(task_execution::Column::Status.eq("failed"))
            .all(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(executions.len() >= 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{repository, workspace};
    use crate::test_utils::db::TestDatabase;
    use chrono::Utc;
    use sea_orm::Set;

    #[tokio::test]
    async fn test_categorize_container_error() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let analyzer = TaskFailureAnalyzer::new(db.clone());

        let execution = task_execution::Model {
            id: 1,
            task_id: 1,
            agent_id: None,
            status: "failed".to_string(),
            command: "test".to_string(),
            exit_code: Some(1),
            stdout_summary: None,
            stderr_summary: Some("Error: No such container: abc123".to_string()),
            stdout_file_path: None,
            stderr_file_path: None,
            error_message: Some("Container not found".to_string()),
            pr_number: None,
            pr_url: None,
            branch_name: None,
            duration_ms: Some(1000),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let (category, _) = analyzer.categorize_failure(&execution);
        assert_eq!(category, FailureCategory::ContainerError);
    }

    #[tokio::test]
    async fn test_categorize_git_error() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let analyzer = TaskFailureAnalyzer::new(db.clone());

        let execution = task_execution::Model {
            id: 1,
            task_id: 1,
            agent_id: None,
            status: "failed".to_string(),
            command: "test".to_string(),
            exit_code: Some(128),
            stdout_summary: None,
            stderr_summary: Some("fatal: could not read from remote repository".to_string()),
            stdout_file_path: None,
            stderr_file_path: None,
            error_message: Some("Git push failed".to_string()),
            pr_number: None,
            pr_url: None,
            branch_name: None,
            duration_ms: Some(5000),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let (category, _) = analyzer.categorize_failure(&execution);
        assert_eq!(category, FailureCategory::GitError);
    }

    #[tokio::test]
    async fn test_generate_recommendations_container_error() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let analyzer = TaskFailureAnalyzer::new(db.clone());

        let recommendations = analyzer.generate_recommendations(
            &FailureCategory::ContainerError,
            "Container not running",
            &None,
        );

        assert!(!recommendations.is_empty());
        assert!(recommendations
            .iter()
            .any(|r| r.contains("container") || r.contains("Docker")));
    }

    #[tokio::test]
    async fn test_is_recurring_failure() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        let task = create_test_task(db).await;
        let analyzer = TaskFailureAnalyzer::new(db.clone());

        // Create multiple failed executions
        for _ in 0..3 {
            let exec = task_execution::ActiveModel {
                task_id: Set(task.id),
                agent_id: Set(None),
                status: Set("failed".to_string()),
                command: Set("test".to_string()),
                error_message: Set(Some("Test error".to_string())),
                ..Default::default()
            };
            TaskExecution::insert(exec)
                .exec_with_returning(db)
                .await
                .unwrap();
        }

        let is_recurring = analyzer.is_recurring_failure(task.id).await.unwrap();
        assert!(is_recurring);
    }

    async fn create_test_task(db: &DatabaseConnection) -> task::Model {
        let workspace = create_test_workspace(db).await;

        let task = task::ActiveModel {
            workspace_id: Set(workspace.id),
            issue_number: Set(123),
            issue_title: Set("Test task".to_string()),
            issue_body: Set(None),
            task_status: Set(TaskStatus::Failed),
            priority: Set("medium".to_string()),
            ..Default::default()
        };

        Task::insert(task).exec_with_returning(db).await.unwrap()
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
