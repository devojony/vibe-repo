//! Repository Service
//!
//! Background service for managing repository operations.
//!
//! This service provides methods for adding, initializing, and managing repositories.

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};
use std::sync::Arc;

use crate::entities::{agent, prelude::*, repository, workspace};
use crate::error::{Result, VibeRepoError};
use crate::git_provider::{
    CreateBranchRequest, CreateWebhookRequest, GitClientFactory, GitProvider, GitProviderError,
    WebhookEvent,
};
use crate::services::BackgroundService;
use crate::state::AppState;

/// Repository service manages repository operations
///
/// This service is designed to be stateless and thread-safe.
pub struct RepositoryService {
    db: DatabaseConnection,
    config: Arc<crate::config::AppConfig>,
}

impl RepositoryService {
    /// Create a new repository service
    pub fn new(db: DatabaseConnection, config: Arc<crate::config::AppConfig>) -> Self {
        Self { db, config }
    }

    /// Get a clone of the database connection
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
    }

    /// Add a new repository with provider configuration
    ///
    /// This method performs the following steps atomically:
    /// 1. Validates token by fetching repository info from provider
    /// 2. Validates permissions (branches, labels, PRs, issues, webhooks)
    /// 3. Generates random webhook secret
    /// 4. Stores repository record
    /// 5. Creates workspace and agent
    /// 6. Initializes branch and labels
    /// 7. Creates webhook on provider
    ///
    /// # Arguments
    /// * `provider_type` - Provider type (github, gitea, gitlab)
    /// * `provider_base_url` - Provider API base URL
    /// * `access_token` - Access token for authentication
    /// * `full_name` - Repository full name (owner/repo)
    /// * `branch_name` - Work branch name to create
    ///
    /// # Returns
    /// The created repository model
    ///
    /// # Errors
    /// - `Validation` - Invalid parameters or repository already exists
    /// - `Unauthorized` - Invalid token
    /// - `Forbidden` - Insufficient permissions
    /// - `NotFound` - Repository not found on provider
    /// - `ServiceUnavailable` - Provider unreachable
    pub async fn add_repository(
        &self,
        provider_type: String,
        provider_base_url: String,
        access_token: String,
        full_name: String,
        branch_name: String,
    ) -> Result<repository::Model> {
        tracing::info!("Adding repository: {}", full_name);

        // Start transaction for atomic operation
        let txn = self.db.begin().await?;

        // 1. Check if repository already exists
        let existing = Repository::find()
            .filter(repository::Column::FullName.eq(&full_name))
            .filter(repository::Column::ProviderType.eq(&provider_type))
            .filter(repository::Column::ProviderBaseUrl.eq(&provider_base_url))
            .one(&txn)
            .await?;

        if existing.is_some() {
            return Err(VibeRepoError::Conflict(format!(
                "Repository {} already exists",
                full_name
            )));
        }

        // 2. Create GitClient with provided credentials
        let git_client =
            GitClientFactory::create(&provider_type, &provider_base_url, &access_token).map_err(
                |e| VibeRepoError::Validation(format!("Invalid provider configuration: {}", e)),
            )?;

        // 3. Fetch repository info from provider (validates token + repo exists)
        let (owner, repo_name) = self.parse_full_name(&full_name)?;
        let git_repo = git_client
            .get_repository(owner, repo_name)
            .await
            .map_err(|e| match e {
                GitProviderError::Unauthorized(_) => {
                    VibeRepoError::Forbidden("Invalid access token".to_string())
                }
                GitProviderError::Forbidden(_) => VibeRepoError::Forbidden(
                    "Insufficient permissions to access repository".to_string(),
                ),
                GitProviderError::NotFound(_) => {
                    VibeRepoError::NotFound(format!("Repository {} not found", full_name))
                }
                GitProviderError::NetworkError(_) => {
                    VibeRepoError::ServiceUnavailable("Git provider unreachable".to_string())
                }
                _ => VibeRepoError::Internal(format!("Failed to fetch repository: {}", e)),
            })?;

        // 4. Validate permissions
        let permissions = self
            .validate_permissions(&git_client, owner, repo_name)
            .await?;
        if !permissions.can_write {
            return Err(VibeRepoError::Forbidden(
                "Token must have write permissions (branches, labels, PRs, issues, webhooks)"
                    .to_string(),
            ));
        }

        // 5. Generate random webhook secret (32 bytes = 64 hex chars)
        let webhook_secret = self.generate_webhook_secret();

        // 6. Store repository record
        let repo = repository::ActiveModel {
            provider_type: ActiveValue::Set(provider_type),
            provider_base_url: ActiveValue::Set(provider_base_url),
            access_token: ActiveValue::Set(access_token),
            webhook_secret: ActiveValue::Set(Some(webhook_secret.clone())),
            name: ActiveValue::Set(git_repo.name.clone()),
            full_name: ActiveValue::Set(full_name.clone()),
            clone_url: ActiveValue::Set(git_repo.clone_url.clone()),
            default_branch: ActiveValue::Set(git_repo.default_branch.clone()),
            branches: ActiveValue::Set(serde_json::json!([])),
            validation_status: ActiveValue::Set(repository::ValidationStatus::Pending),
            status: ActiveValue::Set(repository::RepositoryStatus::Uninitialized),
            webhook_status: ActiveValue::Set(repository::WebhookStatus::Pending),
            has_workspace: ActiveValue::Set(false),
            has_required_branches: ActiveValue::Set(false),
            has_required_labels: ActiveValue::Set(false),
            can_manage_prs: ActiveValue::Set(permissions.can_write),
            can_manage_issues: ActiveValue::Set(permissions.can_write),
            validation_message: ActiveValue::Set(None),
            deleted_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(chrono::Utc::now()),
            updated_at: ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        let repo = repo.insert(&txn).await?;

        // 7. Create workspace
        let workspace = workspace::ActiveModel {
            repository_id: ActiveValue::Set(repo.id),
            container_id: ActiveValue::Set(None),
            workspace_status: ActiveValue::Set("idle".to_string()),
            created_at: ActiveValue::Set(chrono::Utc::now()),
            updated_at: ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };
        let workspace = workspace.insert(&txn).await?;

        // 8. Create agent
        let agent = agent::ActiveModel {
            workspace_id: ActiveValue::Set(workspace.id),
            name: ActiveValue::Set("default".to_string()),
            tool_type: ActiveValue::Set("opencode".to_string()),
            command: ActiveValue::Set(self.config.agent.default_command.clone()),
            env_vars: ActiveValue::Set(serde_json::json!({})),
            timeout: ActiveValue::Set(self.config.agent.default_timeout as i32),
            created_at: ActiveValue::Set(chrono::Utc::now()),
            updated_at: ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };
        agent.insert(&txn).await?;

        // Update repository has_workspace flag
        let mut repo_active: repository::ActiveModel = repo.clone().into();
        repo_active.has_workspace = ActiveValue::Set(true);
        let repo = repo_active.update(&txn).await?;

        // Commit transaction before external API calls
        txn.commit().await?;

        // 9. Initialize branch and labels (best-effort, don't fail if these fail)
        let init_result = self
            .initialize_repository_external(
                repo.id,
                &branch_name,
                Some(self.config.webhook.domain.clone()),
                Some(webhook_secret),
            )
            .await;

        match init_result {
            Ok(updated_repo) => {
                tracing::info!(
                    "Repository {} added and initialized successfully",
                    full_name
                );
                Ok(updated_repo)
            }
            Err(e) => {
                tracing::error!(
                    "Repository {} added but initialization failed: {}",
                    full_name,
                    e
                );
                // Return the repository even if initialization failed
                // User can retry initialization later
                Ok(repo)
            }
        }
    }

    /// Generate a cryptographically secure random webhook secret
    fn generate_webhook_secret(&self) -> String {
        use rand::Rng;
        let mut rng = rand::rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
        hex::encode(bytes)
    }

    /// Create webhook for a repository
    ///
    /// Creates a webhook on the Git provider and updates the webhook status in the database.
    /// This method is idempotent - if a webhook already exists for this repository, it returns success.
    ///
    /// # Arguments
    /// * `repo` - The repository model
    /// * `webhook_url` - The webhook endpoint URL
    /// * `webhook_secret` - The secret for signing webhooks
    ///
    /// # Returns
    /// Ok(()) on success
    async fn create_webhook_for_repository(
        &self,
        repo: &repository::Model,
        webhook_url: String,
        webhook_secret: String,
    ) -> Result<()> {
        // Create Git client
        let client = GitClientFactory::from_repository(repo)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create git client: {}", e)))?;

        // Parse repository owner and name from full_name
        let parts: Vec<&str> = repo.full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(VibeRepoError::Validation(format!(
                "Invalid repository full_name format: {}",
                repo.full_name
            )));
        }
        let (owner, repo_name) = (parts[0], parts[1]);

        // Create webhook on Git provider
        let webhook_request = CreateWebhookRequest {
            url: webhook_url.clone(),
            secret: webhook_secret.clone(),
            events: vec![WebhookEvent::IssueComment, WebhookEvent::PullRequestComment],
            active: true,
        };

        tracing::info!(
            repository_id = repo.id,
            repository = %repo.full_name,
            webhook_url = %webhook_url,
            "Creating webhook on Git provider"
        );

        client
            .create_webhook(owner, repo_name, webhook_request)
            .await
            .map_err(|e| match e {
                GitProviderError::Forbidden(_) => VibeRepoError::Forbidden(format!(
                    "Insufficient permissions to create webhook for repository {}",
                    repo.full_name
                )),
                GitProviderError::NotFound(_) => VibeRepoError::NotFound(format!(
                    "Repository {} not found on Git provider",
                    repo.full_name
                )),
                GitProviderError::NetworkError(_) => VibeRepoError::ServiceUnavailable(format!(
                    "Git provider unreachable while creating webhook for {}",
                    repo.full_name
                )),
                _ => VibeRepoError::Internal(format!(
                    "Failed to create webhook for {}: {}",
                    repo.full_name, e
                )),
            })?;

        tracing::info!(repository_id = repo.id, "Webhook created successfully");

        Ok(())
    }

    /// Initialize a single repository by creating work branch and required labels (external API calls)
    ///
    /// This method creates the specified work branch from the default branch's latest commit,
    /// creates all required labels with vibe/ prefix, creates a webhook for the repository,
    /// updates the database state, and re-validates the repository.
    ///
    /// The operation is idempotent - if the branch, labels, or webhook already exist, it will update
    /// the database state to ensure consistency and return success.
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to initialize
    /// * `branch_name` - The name of the work branch to create (e.g., "vibe-dev")
    /// * `webhook_domain` - Optional webhook domain for creating webhooks (e.g., "https://gitautodev.example.com")
    /// * `webhook_secret` - Optional webhook secret for signing webhooks
    ///
    /// # Returns
    /// The updated repository model on success
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    /// - `Forbidden` - Insufficient permissions to create branch or labels
    /// - `ServiceUnavailable` - Git provider unreachable
    /// - `Validation` - Default branch not found
    async fn initialize_repository_external(
        &self,
        repo_id: i32,
        branch_name: &str,
        webhook_domain: Option<String>,
        webhook_secret: Option<String>,
    ) -> Result<repository::Model> {
        tracing::info!("Initializing repository {}", repo_id);

        // 1. Fetch repository from database
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        // 2. Create GitProvider client
        let git_client = GitClientFactory::from_repository(&repo)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create git client: {}", e)))?;

        // 3. Parse owner/repo from full_name
        let (owner, repo_name) = self.parse_full_name(&repo.full_name)?;

        // 4. Try to create work branch
        let create_result = self
            .create_work_branch(
                &git_client,
                owner,
                repo_name,
                &repo.default_branch,
                branch_name,
            )
            .await;

        // 5. Handle branch creation result
        match create_result {
            Ok(_) => {
                tracing::info!("Created {} branch for repository {}", branch_name, repo_id);
            }
            Err(VibeRepoError::Conflict(_)) => {
                // Branch already exists - this is fine (idempotent operation)
                tracing::info!(
                    "{} branch already exists for repository {}",
                    branch_name,
                    repo_id
                );
            }
            Err(e) => {
                // Store error message and return
                self.update_validation_message_field(repo_id, Some(e.to_string()))
                    .await?;
                return Err(e);
            }
        }

        // 6. Try to create required labels
        if let Err(e) = self
            .create_required_labels(&git_client, owner, repo_name)
            .await
        {
            tracing::warn!(
                "Failed to create some labels for repository {}: {}",
                repo_id,
                e
            );
            // Continue - label creation failure should not block initialization
        }

        // 7. Create webhook if domain and secret are provided
        let mut webhook_status = repository::WebhookStatus::Pending;
        if let (Some(domain), Some(secret)) = (webhook_domain, webhook_secret) {
            let webhook_url = format!("{}/api/webhooks/{}", domain, repo.id);

            match self
                .create_webhook_for_repository(&repo, webhook_url, secret)
                .await
            {
                Ok(_) => {
                    tracing::info!(repository_id = repo.id, "Webhook created successfully");
                    webhook_status = repository::WebhookStatus::Active;
                }
                Err(e) => {
                    tracing::error!(
                        repository_id = repo.id,
                        error = %e,
                        "Failed to create webhook"
                    );
                    webhook_status = repository::WebhookStatus::Failed;

                    // Don't return error - webhook creation failure shouldn't block initialization
                }
            }
        }

        // 8. Re-fetch branches and update database
        let updated_branches = git_client
            .list_branches(owner, repo_name)
            .await
            .map_err(|e| self.map_git_error(e))?;

        let branch_names: Vec<String> = updated_branches.iter().map(|b| b.name.clone()).collect();
        let has_work_branch = branch_names.contains(&branch_name.to_string());

        // 10. Re-validate repository (check labels and permissions)
        let validation = self
            .validate_repository(&git_client, &repo.full_name)
            .await?;

        // 11. Calculate validation status
        // Valid only when all four conditions are met
        let is_valid = has_work_branch
            && validation.has_required_labels
            && validation.can_manage_prs
            && validation.can_manage_issues;

        let status = if is_valid {
            repository::ValidationStatus::Valid
        } else {
            repository::ValidationStatus::Invalid
        };

        // 12. Update repository in database
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        let mut active: repository::ActiveModel = repo.into();
        active.branches = ActiveValue::Set(serde_json::json!(branch_names));
        active.has_required_branches = ActiveValue::Set(has_work_branch);
        active.has_required_labels = ActiveValue::Set(validation.has_required_labels);
        active.can_manage_prs = ActiveValue::Set(validation.can_manage_prs);
        active.can_manage_issues = ActiveValue::Set(validation.can_manage_issues);
        active.validation_status = ActiveValue::Set(status);
        active.validation_message = ActiveValue::Set(None);
        active.webhook_status = ActiveValue::Set(webhook_status);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());

        let updated = active.update(&self.db).await?;

        tracing::info!("Repository {} initialized successfully", repo_id);

        Ok(updated)
    }

    /// Initialize a single repository by creating work branch and required labels
    ///
    /// This is the public API for repository initialization.
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to initialize
    /// * `branch_name` - The name of the work branch to create (e.g., "vibe-dev")
    /// * `webhook_domain` - Optional webhook domain for creating webhooks
    /// * `webhook_secret` - Optional webhook secret for signing webhooks
    ///
    /// # Returns
    /// The updated repository model on success
    pub async fn initialize_repository(
        &self,
        repo_id: i32,
        branch_name: &str,
        webhook_domain: Option<String>,
        webhook_secret: Option<String>,
    ) -> Result<repository::Model> {
        self.initialize_repository_external(repo_id, branch_name, webhook_domain, webhook_secret)
            .await
    }

    /// Create work branch from default branch
    ///
    /// # Arguments
    /// * `git_client` - The GitProvider client
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `default_branch` - The default branch to create from
    /// * `branch_name` - The name of the work branch to create
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// - `Conflict` - Branch already exists (idempotent - not a real error)
    /// - `Forbidden` - Insufficient permissions
    /// - `ServiceUnavailable` - Git provider unreachable
    /// - `Validation` - Default branch not found
    async fn create_work_branch<P: GitProvider>(
        &self,
        git_client: &P,
        owner: &str,
        repo: &str,
        default_branch: &str,
        branch_name: &str,
    ) -> Result<()> {
        let create_req = CreateBranchRequest {
            name: branch_name.to_string(),
            source: default_branch.to_string(),
        };

        git_client
            .create_branch(owner, repo, create_req)
            .await
            .map_err(|e| match e {
                GitProviderError::BranchAlreadyExists(_) => {
                    VibeRepoError::Conflict("Branch already exists".to_string())
                }
                other => self.map_git_error(other),
            })?;

        Ok(())
    }

    /// Map GitProviderError to VibeRepoError with appropriate status codes
    ///
    /// # Error Mapping
    /// - `Unauthorized` / `Forbidden` -> `Forbidden` (403)
    /// - `NetworkError` -> `ServiceUnavailable` (503)
    /// - `NotFound` (branch/ref) -> `Validation` (400)
    /// - `NotFound` (other) -> `NotFound` (404)
    /// - `BranchAlreadyExists` -> `Conflict` (409)
    /// - Others -> `Internal` (500)
    pub(crate) fn map_git_error(&self, error: GitProviderError) -> VibeRepoError {
        match error {
            GitProviderError::Unauthorized(_) | GitProviderError::Forbidden(_) => {
                VibeRepoError::Forbidden("Insufficient permissions to create branch".to_string())
            }
            GitProviderError::NotFound(msg) => {
                // Check if the error is about a branch or ref (source branch not found)
                let msg_lower = msg.to_lowercase();
                if msg_lower.contains("branch")
                    || msg_lower.contains("ref")
                    || msg_lower.contains("reference")
                {
                    VibeRepoError::Validation("Default branch not found".to_string())
                } else {
                    VibeRepoError::NotFound(msg)
                }
            }
            GitProviderError::NetworkError(_) => {
                VibeRepoError::ServiceUnavailable("Git provider unreachable".to_string())
            }
            GitProviderError::BranchAlreadyExists(_) => {
                VibeRepoError::Conflict("Branch already exists".to_string())
            }
            _ => VibeRepoError::Internal(error.to_string()),
        }
    }

    /// Update validation message for a repository
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository
    /// * `message` - The validation message to set (None to clear)
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    async fn update_validation_message_field(
        &self,
        repo_id: i32,
        message: Option<String>,
    ) -> Result<()> {
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        let mut active: repository::ActiveModel = repo.into();
        active.validation_message = ActiveValue::Set(message);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());
        active.update(&self.db).await?;

        Ok(())
    }

    /// Parse owner and repo from full_name (format: "owner/repo")
    fn parse_full_name<'a>(&self, full_name: &'a str) -> Result<(&'a str, &'a str)> {
        let parts: Vec<&str> = full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(VibeRepoError::Internal(format!(
                "Invalid repository full_name: {}",
                full_name
            )));
        }
        Ok((parts[0], parts[1]))
    }

    /// Validate a repository
    async fn validate_repository<P: GitProvider>(
        &self,
        git_client: &P,
        repo_full_name: &str,
    ) -> Result<ValidationInfo> {
        // Parse owner and repo from full_name (format: "owner/repo")
        let parts: Vec<&str> = repo_full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(VibeRepoError::Internal(format!(
                "Invalid repository full_name: {}",
                repo_full_name
            )));
        }
        let (owner, repo) = (parts[0], parts[1]);

        // Check branches (using default vibe-dev branch)
        let branch_info = self
            .check_branches(git_client, owner, repo, "vibe-dev")
            .await?;

        // Check labels
        let has_labels = self.check_labels(git_client, owner, repo).await?;

        // Check permissions
        let permissions = self.validate_permissions(git_client, owner, repo).await?;

        Ok(ValidationInfo {
            branches: branch_info.branches,
            has_required_branches: branch_info.has_required,
            has_required_labels: has_labels,
            can_manage_prs: permissions.can_write,
            can_manage_issues: permissions.can_write,
        })
    }

    /// Validate token has necessary permissions for a repository
    pub(crate) async fn validate_permissions<P: GitProvider>(
        &self,
        git_client: &P,
        owner: &str,
        repo: &str,
    ) -> Result<PermissionInfo> {
        let repository = git_client
            .get_repository(owner, repo)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to check permissions: {}", e)))?;

        Ok(PermissionInfo {
            can_read: repository.permissions.pull,
            can_write: repository.permissions.push,
            can_admin: repository.permissions.admin,
        })
    }

    /// Check if repository has required branches
    ///
    /// A repository has required branches if it contains the specified work branch.
    /// This branch is used for automated development tasks.
    ///
    /// # Arguments
    /// * `git_client` - The GitProvider client
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `branch_name` - The name of the work branch to check for
    ///
    /// # Returns
    /// BranchInfo containing all branches and whether the required branch exists
    pub(crate) async fn check_branches<P: GitProvider>(
        &self,
        git_client: &P,
        owner: &str,
        repo: &str,
        branch_name: &str,
    ) -> Result<BranchInfo> {
        let branches = git_client
            .list_branches(owner, repo)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to fetch branches: {}", e)))?;

        let branch_names: Vec<String> = branches.iter().map(|b| b.name.clone()).collect();

        // Check for specified work branch
        let has_required = branch_names.iter().any(|name| name == branch_name);

        Ok(BranchInfo {
            branches: branch_names,
            has_required,
        })
    }

    /// Check if repository has required issue labels with vibe/ prefix
    ///
    /// Requirements: 3.1, 3.2, 3.3
    pub(crate) async fn check_labels<P: GitProvider>(
        &self,
        git_client: &P,
        owner: &str,
        repo: &str,
    ) -> Result<bool> {
        use crate::api::repositories::models::REQUIRED_LABELS;

        let labels = git_client
            .list_labels(owner, repo)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to fetch labels: {}", e)))?;

        let label_names: Vec<String> = labels.iter().map(|l| l.name.clone()).collect();

        // Check for all required labels with vibe/ prefix
        let has_all_required = REQUIRED_LABELS
            .iter()
            .all(|req| label_names.iter().any(|name| name == req));

        Ok(has_all_required)
    }

    /// Create all required labels if they don't exist
    ///
    /// This method is idempotent - it will skip labels that already exist.
    /// If some labels fail to create, it will log errors but continue with
    /// remaining labels.
    ///
    /// Requirements: 1.5, 1.6, 1.7, 5.5
    pub(crate) async fn create_required_labels<P: GitProvider>(
        &self,
        git_client: &P,
        owner: &str,
        repo: &str,
    ) -> Result<()> {
        use crate::api::repositories::models::REQUIRED_LABELS;
        use crate::git_provider::{CreateLabelRequest, GitProviderError};

        // First, get existing labels to avoid unnecessary API calls
        let existing_labels = git_client
            .list_labels(owner, repo)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to fetch labels: {}", e)))?;

        let existing_names: Vec<String> = existing_labels.iter().map(|l| l.name.clone()).collect();

        // Iterate through required labels and create missing ones
        for label_name in REQUIRED_LABELS {
            if existing_names.contains(&label_name.to_string()) {
                tracing::debug!("Label {} already exists, skipping", label_name);
                continue;
            }

            let create_req = CreateLabelRequest {
                name: label_name.to_string(),
                color: "0366d6".to_string(), // Default blue color
                description: Some(format!("Workflow label: {}", label_name)),
            };

            match git_client.create_label(owner, repo, create_req).await {
                Ok(_) => {
                    tracing::info!("Created label: {}", label_name);
                }
                Err(GitProviderError::LabelAlreadyExists(_)) => {
                    // Label was created between our check and now (race condition)
                    tracing::debug!("Label {} already exists (race condition)", label_name);
                    // Continue - this is fine (idempotent operation)
                }
                Err(e) => {
                    // Log error but continue with other labels
                    tracing::error!("Failed to create label {}: {}", label_name, e);
                    // Don't return error - continue with remaining labels (Requirement 5.5)
                }
            }
        }

        Ok(())
    }

    /// Archive a repository
    ///
    /// Sets the repository status to Archived. Archived repositories:
    /// - Are skipped during provider sync
    /// - Cannot be modified
    /// - Cannot have workspaces
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to archive
    ///
    /// # Returns
    /// The updated repository model
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    /// - `Conflict` - Repository has a workspace or is already archived
    pub async fn archive_repository(&self, repo_id: i32) -> Result<repository::Model> {
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        // Check if repository has workspace
        if repo.has_workspace {
            return Err(VibeRepoError::Conflict(
                "Cannot archive repository with workspace. Delete workspace first.".to_string(),
            ));
        }

        // Check if already archived
        if repo.status == repository::RepositoryStatus::Archived {
            return Err(VibeRepoError::Conflict(
                "Repository is already archived".to_string(),
            ));
        }

        // Update status to archived
        let mut active: repository::ActiveModel = repo.into();
        active.status = ActiveValue::Set(repository::RepositoryStatus::Archived);
        let updated = active.update(&self.db).await?;

        tracing::info!("Archived repository {}", repo_id);
        Ok(updated)
    }

    /// Unarchive a repository
    ///
    /// Restores an archived repository to Idle or Unavailable status based on validation.
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to unarchive
    ///
    /// # Returns
    /// The updated repository model
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    /// - `Conflict` - Repository is not archived
    pub async fn unarchive_repository(&self, repo_id: i32) -> Result<repository::Model> {
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        // Check if repository is archived
        if repo.status != repository::RepositoryStatus::Archived {
            return Err(VibeRepoError::Conflict(
                "Repository is not archived".to_string(),
            ));
        }

        // Determine new status based on validation
        let new_status = if repo.validation_status == repository::ValidationStatus::Valid {
            repository::RepositoryStatus::Idle
        } else {
            repository::RepositoryStatus::Unavailable
        };

        // Update status
        let mut active: repository::ActiveModel = repo.into();
        active.status = ActiveValue::Set(new_status.clone());
        let updated = active.update(&self.db).await?;

        tracing::info!(
            "Unarchived repository {} to status {:?}",
            repo_id,
            new_status
        );
        Ok(updated)
    }

    /// Soft delete a repository
    ///
    /// Marks a repository as deleted by setting deleted_at timestamp.
    /// Soft-deleted repositories will be automatically restored if they appear
    /// in the next provider sync.
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to delete
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    /// - `Conflict` - Repository has a workspace
    pub async fn soft_delete_repository(&self, repo_id: i32) -> Result<()> {
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        // Check if repository has workspace
        if repo.has_workspace {
            return Err(VibeRepoError::Conflict(
                "Cannot delete repository with workspace. Delete workspace first.".to_string(),
            ));
        }

        // Set deleted_at timestamp
        let mut active: repository::ActiveModel = repo.into();
        active.deleted_at = ActiveValue::Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        tracing::info!("Soft deleted repository {}", repo_id);
        Ok(())
    }

    /// Delete a repository and its webhook
    ///
    /// This method performs a hard delete:
    /// 1. Deletes the webhook from the Git provider (best-effort)
    /// 2. Deletes the repository record
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to delete
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    ///
    /// # Notes
    /// - Webhook deletion from Git provider is best-effort; repository deletion proceeds even if it fails
    pub async fn delete_repository(&self, repo_id: i32) -> Result<()> {
        tracing::info!(repository_id = repo_id, "Deleting repository");

        // Get repository
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound(format!("Repository {} not found", repo_id)))?;

        // Try to delete webhook from Git provider (best-effort)
        if let Err(e) = self.delete_webhook_from_provider(&repo).await {
            tracing::error!(
                repository_id = repo_id,
                error = %e,
                "Failed to delete webhook from Git provider, continuing with repository deletion"
            );
            // Continue with deletion even if webhook deletion fails
        }

        // Delete repository
        let repo_active: repository::ActiveModel = repo.into();
        repo_active.delete(&self.db).await?;

        tracing::info!(repository_id = repo_id, "Repository deleted successfully");

        Ok(())
    }

    /// Delete webhook from Git provider
    ///
    /// # Arguments
    /// * `repo` - The repository model
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// - Various errors from Git provider API
    async fn delete_webhook_from_provider(&self, repo: &repository::Model) -> Result<()> {
        // Create Git client
        let client = GitClientFactory::from_repository(repo)
            .map_err(|e| VibeRepoError::Internal(format!("Failed to create git client: {}", e)))?;

        // Parse repository owner and name
        let parts: Vec<&str> = repo.full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(VibeRepoError::Validation(format!(
                "Invalid repository full_name format: {}",
                repo.full_name
            )));
        }
        let (owner, repo_name) = (parts[0], parts[1]);

        // List webhooks to find the one we created
        let webhooks = client
            .list_webhooks(owner, repo_name)
            .await
            .map_err(|e| VibeRepoError::Internal(format!("Failed to list webhooks: {}", e)))?;

        // Find webhook by URL pattern (contains our repo_id)
        let webhook_url_pattern = format!("/api/webhooks/{}", repo.id);
        let webhook_to_delete = webhooks
            .iter()
            .find(|w| w.url.contains(&webhook_url_pattern));

        if let Some(webhook) = webhook_to_delete {
            // Delete webhook from Git provider
            match client.delete_webhook(owner, repo_name, &webhook.id).await {
                Ok(_) => {
                    tracing::info!(
                        webhook_id = %webhook.id,
                        repository_id = repo.id,
                        "Webhook deleted from Git provider"
                    );
                    Ok(())
                }
                Err(GitProviderError::NotFound(_)) => {
                    tracing::warn!(
                        webhook_id = %webhook.id,
                        "Webhook not found on Git provider, may have been deleted manually"
                    );
                    // Not an error - webhook already gone
                    Ok(())
                }
                Err(e) => Err(VibeRepoError::Internal(format!(
                    "Failed to delete webhook from Git provider: {}",
                    e
                ))),
            }
        } else {
            tracing::warn!(repository_id = repo.id, "No webhook found for repository");
            Ok(())
        }
    }

    /// Restore a soft-deleted repository
    ///
    /// Clears the deleted_at timestamp and resets status to Uninitialized.
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to restore
    ///
    /// # Returns
    /// The updated repository model
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    /// - `Conflict` - Repository is not deleted
    pub async fn restore_repository(&self, repo_id: i32) -> Result<repository::Model> {
        // Find repository including deleted ones
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        // Check if repository is deleted
        if repo.deleted_at.is_none() {
            return Err(VibeRepoError::Conflict(
                "Repository is not deleted".to_string(),
            ));
        }

        // Clear deleted_at and reset status
        let mut active: repository::ActiveModel = repo.into();
        active.deleted_at = ActiveValue::Set(None);
        active.status = ActiveValue::Set(repository::RepositoryStatus::Uninitialized);
        let updated = active.update(&self.db).await?;

        tracing::info!("Restored repository {}", repo_id);
        Ok(updated)
    }

    /// Update repository metadata
    ///
    /// Updates the repository name. This is a simple metadata update
    /// that doesn't affect the repository's status or validation.
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to update
    /// * `name` - The new repository name
    ///
    /// # Returns
    /// The updated repository model
    ///
    /// # Errors
    /// - `NotFound` - Repository not found
    /// - `Conflict` - Repository is archived (read-only)
    pub async fn update_repository_metadata(
        &self,
        repo_id: i32,
        name: &str,
    ) -> Result<repository::Model> {
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| VibeRepoError::NotFound("Repository not found".to_string()))?;

        // Check if repository is archived
        if repo.status == repository::RepositoryStatus::Archived {
            return Err(VibeRepoError::Conflict(
                "Cannot modify archived repository. Unarchive it first.".to_string(),
            ));
        }

        // Update name
        let mut active: repository::ActiveModel = repo.into();
        active.name = ActiveValue::Set(name.to_string());
        let updated = active.update(&self.db).await?;

        tracing::info!("Updated repository {} metadata", repo_id);
        Ok(updated)
    }
}

#[async_trait]
impl BackgroundService for RepositoryService {
    fn name(&self) -> &'static str {
        "repository_service"
    }

    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        tracing::info!("RepositoryService started (no periodic sync in per-repo-provider-config)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("RepositoryService stopped");
        Ok(())
    }

    async fn health_check(&self) -> bool {
        // Check database connection
        self.db.ping().await.is_ok()
    }
}

/// Validation information for a repository
#[derive(Debug)]
#[allow(dead_code)]
struct ValidationInfo {
    branches: Vec<String>,
    has_required_branches: bool,
    has_required_labels: bool,
    can_manage_prs: bool,
    can_manage_issues: bool,
}

/// Permission information for a repository
#[derive(Debug)]
pub struct PermissionInfo {
    pub can_read: bool,
    pub can_write: bool,
    pub can_admin: bool,
}

/// Branch information for a repository
#[derive(Debug)]
pub struct BranchInfo {
    pub branches: Vec<String>,
    pub has_required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::repositories::models::REQUIRED_LABELS;
    use crate::test_utils::db::create_test_database;
    use sea_orm::Set;

    // Helper function to create test service with default config
    fn create_test_service(db: DatabaseConnection) -> RepositoryService {
        let config = Arc::new(crate::config::AppConfig::default());
        RepositoryService::new(db, config)
    }

    #[test]
    fn test_required_labels_constant_has_vibe_prefix() {
        // Test that all required labels have the vibe/ prefix
        for label in REQUIRED_LABELS {
            assert!(
                label.starts_with("vibe/"),
                "Label {} should start with vibe/ prefix",
                label
            );
        }
    }

    #[test]
    fn test_required_labels_constant_has_all_expected_labels() {
        // Test that REQUIRED_LABELS contains exactly the expected labels
        assert_eq!(REQUIRED_LABELS.len(), 5);
        assert!(REQUIRED_LABELS.contains(&"vibe/pending-ack"));
        assert!(REQUIRED_LABELS.contains(&"vibe/todo-ai"));
        assert!(REQUIRED_LABELS.contains(&"vibe/in-progress"));
        assert!(REQUIRED_LABELS.contains(&"vibe/review-required"));
        assert!(REQUIRED_LABELS.contains(&"vibe/failed"));
    }

    #[test]
    fn test_required_labels_are_case_sensitive() {
        // Test that labels are case-sensitive (no uppercase variants)
        assert!(!REQUIRED_LABELS.contains(&"VIBE/pending-ack"));
        assert!(!REQUIRED_LABELS.contains(&"Vibe/todo-ai"));
        assert!(!REQUIRED_LABELS.contains(&"vibe/PENDING-ACK"));
    }

    // Helper function to create test repository
    async fn create_test_repo(
        db: &DatabaseConnection,
        name: &str,
        status: repository::RepositoryStatus,
        has_workspace: bool,
    ) -> repository::Model {
        let repo = repository::ActiveModel {
            name: Set(name.to_string()),
            full_name: Set(format!("owner/{}", name)),
            clone_url: Set(format!("https://gitea.example.com/owner/{}.git", name)),
            default_branch: Set("main".to_string()),
            branches: Set(serde_json::json!(["main"])),
            validation_status: Set(repository::ValidationStatus::Valid),
            status: Set(status),
            has_workspace: Set(has_workspace),
            has_required_branches: Set(true),
            has_required_labels: Set(true),
            can_manage_prs: Set(true),
            can_manage_issues: Set(true),
            validation_message: Set(None),
            deleted_at: Set(None),
            provider_type: Set("gitea".to_string()),
            provider_base_url: Set("https://gitea.example.com".to_string()),
            access_token: Set("test_token".to_string()),
            ..Default::default()
        };
        repo.insert(db).await.unwrap()
    }

    // Test archive_repository
    #[tokio::test]
    async fn test_archive_repository_success() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create idle repository without workspace
        let repo =
            create_test_repo(&db, "test-repo", repository::RepositoryStatus::Idle, false).await;

        // Archive the repository
        let result = service.archive_repository(repo.id).await;
        assert!(result.is_ok());
        let archived = result.unwrap();
        assert_eq!(archived.status, repository::RepositoryStatus::Archived);
    }

    #[tokio::test]
    async fn test_archive_repository_with_workspace_fails() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create active repository with workspace
        let repo =
            create_test_repo(&db, "test-repo", repository::RepositoryStatus::Active, true).await;

        // Try to archive - should fail
        let result = service.archive_repository(repo.id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VibeRepoError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_archive_already_archived_fails() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create archived repository
        let repo = create_test_repo(
            &db,
            "test-repo",
            repository::RepositoryStatus::Archived,
            false,
        )
        .await;

        // Try to archive again - should fail
        let result = service.archive_repository(repo.id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VibeRepoError::Conflict(_)));
    }

    // Test unarchive_repository
    #[tokio::test]
    async fn test_unarchive_repository_to_idle() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create archived repository with valid status
        let repo = create_test_repo(
            &db,
            "test-repo",
            repository::RepositoryStatus::Archived,
            false,
        )
        .await;

        // Unarchive the repository
        let result = service.unarchive_repository(repo.id).await;
        assert!(result.is_ok());
        let unarchived = result.unwrap();
        assert_eq!(unarchived.status, repository::RepositoryStatus::Idle);
    }

    #[tokio::test]
    async fn test_unarchive_non_archived_fails() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create idle repository
        let repo =
            create_test_repo(&db, "test-repo", repository::RepositoryStatus::Idle, false).await;

        // Try to unarchive - should fail
        let result = service.unarchive_repository(repo.id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VibeRepoError::Conflict(_)));
    }

    // Test soft_delete_repository
    #[tokio::test]
    async fn test_soft_delete_repository_success() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create idle repository without workspace
        let repo =
            create_test_repo(&db, "test-repo", repository::RepositoryStatus::Idle, false).await;

        // Soft delete the repository
        let result = service.soft_delete_repository(repo.id).await;
        assert!(result.is_ok());

        // Verify deleted_at is set
        let deleted = Repository::find_by_id(repo.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert!(deleted.deleted_at.is_some());
    }

    #[tokio::test]
    async fn test_soft_delete_with_workspace_fails() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create active repository with workspace
        let repo =
            create_test_repo(&db, "test-repo", repository::RepositoryStatus::Active, true).await;

        // Try to delete - should fail
        let result = service.soft_delete_repository(repo.id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VibeRepoError::Conflict(_)));
    }

    // Test restore_repository
    #[tokio::test]
    async fn test_restore_repository_success() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create and soft delete a repository
        let repo =
            create_test_repo(&db, "test-repo", repository::RepositoryStatus::Idle, false).await;
        service.soft_delete_repository(repo.id).await.unwrap();

        // Restore the repository
        let result = service.restore_repository(repo.id).await;
        assert!(result.is_ok());
        let restored = result.unwrap();
        assert!(restored.deleted_at.is_none());
        assert_eq!(restored.status, repository::RepositoryStatus::Uninitialized);
    }

    #[tokio::test]
    async fn test_restore_non_deleted_fails() {
        let db = create_test_database().await.unwrap();
        let service = create_test_service(db.clone());

        // Create active repository (not deleted)
        let repo =
            create_test_repo(&db, "test-repo", repository::RepositoryStatus::Idle, false).await;

        // Try to restore - should fail
        let result = service.restore_repository(repo.id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VibeRepoError::Conflict(_)));
    }
}
