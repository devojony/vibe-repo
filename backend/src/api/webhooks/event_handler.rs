//! Event handlers for webhook events

use crate::api::webhooks::models::CommentInfo;
use crate::entities::{
    prelude::*,
    repository,
    task::{self, TaskStatus},
    workspace,
};
use crate::error::VibeRepoError;
use crate::state::AppState;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

/// Handle issue or PR comment event
///
/// This function is called asynchronously to process comment events.
/// When a bot mention is detected, it creates a task for the AI agent to process.
pub async fn handle_comment_event(
    comment_info: CommentInfo,
    state: &AppState,
) -> Result<(), VibeRepoError> {
    tracing::info!(
        comment_id = %comment_info.comment_id,
        author = %comment_info.comment_author,
        issue_or_pr = comment_info.issue_or_pr_number,
        repository = %comment_info.repository_full_name,
        action = %comment_info.action,
        comment_type = ?comment_info.comment_type,
        "Processing comment event"
    );

    // Get bot username from config
    let bot_username = &state.config.webhook.bot_username;
    let has_mention = super::mention::detect_mention(&comment_info.comment_body, bot_username);

    if has_mention {
        tracing::info!(
            comment_id = %comment_info.comment_id,
            "Comment mentions bot, triggering workflow"
        );

        // Find repository by full_name
        let repo = Repository::find()
            .filter(repository::Column::FullName.eq(&comment_info.repository_full_name))
            .one(&state.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!(
                    "Repository '{}' not found",
                    comment_info.repository_full_name
                ))
            })?;

        tracing::debug!(
            repository_id = repo.id,
            repository_name = %repo.full_name,
            "Found repository"
        );

        // Find workspace for this repository
        let workspace_model = Workspace::find()
            .filter(workspace::Column::RepositoryId.eq(repo.id))
            .one(&state.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!(
                    "Workspace not found for repository '{}'",
                    comment_info.repository_full_name
                ))
            })?;

        tracing::debug!(workspace_id = workspace_model.id, "Found workspace");

        // Check if task already exists for this issue/PR
        let existing_task = Task::find()
            .filter(task::Column::WorkspaceId.eq(workspace_model.id))
            .filter(task::Column::IssueNumber.eq(comment_info.issue_or_pr_number as i32))
            .one(&state.db)
            .await
            .map_err(VibeRepoError::Database)?;

        if let Some(existing) = existing_task {
            tracing::info!(
                task_id = existing.id,
                issue_number = comment_info.issue_or_pr_number,
                "Task already exists for this issue/PR, skipping creation"
            );
            return Ok(());
        }

        // Create task from issue/PR
        let new_task = task::ActiveModel {
            workspace_id: Set(workspace_model.id),
            issue_number: Set(comment_info.issue_or_pr_number as i32),
            issue_title: Set(format!(
                "{} #{}",
                match comment_info.comment_type {
                    crate::api::webhooks::models::CommentType::Issue => "Issue",
                    crate::api::webhooks::models::CommentType::PullRequest => "PR",
                },
                comment_info.issue_or_pr_number
            )),
            issue_body: Set(Some(comment_info.comment_body.clone())),
            task_status: Set(TaskStatus::Pending),
            priority: Set("medium".to_string()),
            assigned_agent_id: Set(None),
            ..Default::default()
        };

        let created_task = Task::insert(new_task)
            .exec_with_returning(&state.db)
            .await
            .map_err(VibeRepoError::Database)?;

        tracing::info!(
            task_id = created_task.id,
            workspace_id = workspace_model.id,
            issue_number = comment_info.issue_or_pr_number,
            task_status = %created_task.task_status,
            "Created task for AI agent workflow"
        );
    } else {
        tracing::debug!(
            comment_id = %comment_info.comment_id,
            "Comment does not mention bot, skipping"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::webhooks::models::CommentType;
    use crate::config::AppConfig;
    use crate::entities::{repository, workspace};
    use crate::services::{RepositoryService, TaskService};
    use crate::test_utils::db::create_test_database;
    use chrono::Utc;
    use sea_orm::{EntityTrait, Set};
    use std::sync::Arc;

    async fn create_test_state() -> AppState {
        let db = create_test_database()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::default();
        let config_arc = Arc::new(config.clone());
        let repository_service = Arc::new(RepositoryService::new(db.clone(), config_arc));
        AppState::new(db, config, repository_service)
    }

    async fn create_test_repository_and_workspace(
        state: &AppState,
        full_name: &str,
    ) -> (repository::Model, workspace::Model) {
        use crate::entities::repo_provider;

        // Create provider
        let provider = repo_provider::ActiveModel {
            name: Set(format!("Test Provider {}", uuid::Uuid::new_v4())),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec_with_returning(&state.db)
            .await
            .unwrap();

        // Create repository
        let repo = repository::ActiveModel {
            name: Set(full_name.split('/').next_back().unwrap().to_string()),
            full_name: Set(full_name.to_string()),
            clone_url: Set(format!("https://git.example.com/{}.git", full_name)),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.id),
            ..Default::default()
        };
        let repo = Repository::insert(repo)
            .exec_with_returning(&state.db)
            .await
            .unwrap();

        // Create workspace (simplified MVP version)
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
            workspace_status: Set("Active".to_string()),
            ..Default::default()
        };
        let workspace = Workspace::insert(ws)
            .exec_with_returning(&state.db)
            .await
            .unwrap();

        (repo, workspace)
    }

    #[tokio::test]
    async fn test_handle_comment_event_with_mention() {
        // Arrange
        let state = create_test_state().await;
        let bot_username = &state.config.webhook.bot_username;
        let (_, _workspace) = create_test_repository_and_workspace(&state, "owner/repo").await;

        let comment_info = CommentInfo {
            comment_id: "123".to_string(),
            comment_body: format!("@{} please help", bot_username),
            comment_author: "user1".to_string(),
            issue_or_pr_number: 42,
            repository_full_name: "owner/repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::Issue,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Act
        let result = handle_comment_event(comment_info, &state).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_comment_event_without_mention() {
        let state = create_test_state().await;

        let comment_info = CommentInfo {
            comment_id: "124".to_string(),
            comment_body: "Just a regular comment".to_string(),
            comment_author: "user2".to_string(),
            issue_or_pr_number: 43,
            repository_full_name: "owner/repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::PullRequest,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let result = handle_comment_event(comment_info, &state).await;
        assert!(result.is_ok());
    }

    /// Test that a task is created when bot is mentioned on an issue
    /// Requirements: Webhook Integration - AI agent workflow trigger
    #[tokio::test]
    async fn test_handle_comment_event_creates_task_for_issue_mention() {
        // Arrange
        let state = create_test_state().await;
        let bot_username = &state.config.webhook.bot_username;
        let (_, workspace) = create_test_repository_and_workspace(&state, "owner/test-repo").await;

        let comment_info = CommentInfo {
            comment_id: "200".to_string(),
            comment_body: format!("@{} please implement this feature", bot_username),
            comment_author: "developer1".to_string(),
            issue_or_pr_number: 100,
            repository_full_name: "owner/test-repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::Issue,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Act
        let result = handle_comment_event(comment_info, &state).await;

        // Assert
        assert!(result.is_ok());

        // Verify task was created
        let task_service = TaskService::new(state.db.clone());
        let tasks = task_service
            .list_tasks_by_workspace(workspace.id)
            .await
            .unwrap();

        assert_eq!(tasks.len(), 1);
        let task = &tasks[0];
        assert_eq!(task.issue_number, 100);
        assert_eq!(task.task_status, TaskStatus::Pending);
        assert_eq!(task.workspace_id, workspace.id);
    }

    /// Test that a task is created when bot is mentioned on a PR
    /// Requirements: Webhook Integration - AI agent workflow trigger
    #[tokio::test]
    async fn test_handle_comment_event_creates_task_for_pr_mention() {
        // Arrange
        let state = create_test_state().await;
        let bot_username = &state.config.webhook.bot_username;
        let (_, workspace) = create_test_repository_and_workspace(&state, "owner/pr-repo").await;

        let comment_info = CommentInfo {
            comment_id: "201".to_string(),
            comment_body: format!("@{} please review this PR", bot_username),
            comment_author: "reviewer1".to_string(),
            issue_or_pr_number: 50,
            repository_full_name: "owner/pr-repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::PullRequest,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Act
        let result = handle_comment_event(comment_info, &state).await;

        // Assert
        assert!(result.is_ok());

        // Verify task was created
        let task_service = TaskService::new(state.db.clone());
        let tasks = task_service
            .list_tasks_by_workspace(workspace.id)
            .await
            .unwrap();

        assert_eq!(tasks.len(), 1);
        let task = &tasks[0];
        assert_eq!(task.issue_number, 50);
        assert_eq!(task.task_status, TaskStatus::Pending);
    }

    /// Test that no task is created when repository is not found
    /// Requirements: Webhook Integration - error handling
    #[tokio::test]
    async fn test_handle_comment_event_repository_not_found() {
        // Arrange
        let state = create_test_state().await;
        let bot_username = &state.config.webhook.bot_username;

        let comment_info = CommentInfo {
            comment_id: "202".to_string(),
            comment_body: format!("@{} help", bot_username),
            comment_author: "user1".to_string(),
            issue_or_pr_number: 1,
            repository_full_name: "nonexistent/repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::Issue,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Act
        let result = handle_comment_event(comment_info, &state).await;

        // Assert - should return error for non-existent repository
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    /// Test that no task is created when workspace is not found
    /// Requirements: Webhook Integration - error handling
    #[tokio::test]
    async fn test_handle_comment_event_workspace_not_found() {
        // Arrange
        let state = create_test_state().await;
        let bot_username = &state.config.webhook.bot_username;

        // Create repository without workspace
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
            .exec_with_returning(&state.db)
            .await
            .unwrap();

        let repo = repository::ActiveModel {
            name: Set("no-workspace-repo".to_string()),
            full_name: Set("owner/no-workspace-repo".to_string()),
            clone_url: Set("https://git.example.com/owner/no-workspace-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.id),
            ..Default::default()
        };
        Repository::insert(repo)
            .exec_with_returning(&state.db)
            .await
            .unwrap();

        let comment_info = CommentInfo {
            comment_id: "203".to_string(),
            comment_body: format!("@{} help", bot_username),
            comment_author: "user1".to_string(),
            issue_or_pr_number: 1,
            repository_full_name: "owner/no-workspace-repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::Issue,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Act
        let result = handle_comment_event(comment_info, &state).await;

        // Assert - should return error for missing workspace
        assert!(result.is_err());
        match result.unwrap_err() {
            VibeRepoError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    /// Test that duplicate tasks are not created for the same issue
    /// Requirements: Webhook Integration - idempotency
    #[tokio::test]
    async fn test_handle_comment_event_duplicate_task_handling() {
        // Arrange
        let state = create_test_state().await;
        let bot_username = &state.config.webhook.bot_username;
        let (_, workspace) = create_test_repository_and_workspace(&state, "owner/dup-repo").await;

        let comment_info = CommentInfo {
            comment_id: "204".to_string(),
            comment_body: format!("@{} help", bot_username),
            comment_author: "user1".to_string(),
            issue_or_pr_number: 99,
            repository_full_name: "owner/dup-repo".to_string(),
            action: "created".to_string(),
            comment_type: CommentType::Issue,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        // Act - First mention
        let result1 = handle_comment_event(comment_info.clone(), &state).await;
        assert!(result1.is_ok());

        // Act - Second mention (duplicate)
        let result2 = handle_comment_event(comment_info, &state).await;

        // Assert - should succeed but not create duplicate task
        assert!(result2.is_ok());

        // Verify only one task was created
        let task_service = TaskService::new(state.db.clone());
        let tasks = task_service
            .list_tasks_by_workspace(workspace.id)
            .await
            .unwrap();

        assert_eq!(tasks.len(), 1);
    }
}
