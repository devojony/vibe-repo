//! Repository Service
//!
//! Background service for managing repository operations and synchronization.
//!
//! This service supports both direct method calls (for API handlers) and
//! periodic background synchronization tasks.

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::sync::Arc;

use crate::entities::{prelude::*, repository};
use crate::error::{GitAutoDevError, Result};
use crate::git_provider::{CreateBranchRequest, GitClientFactory, GitProvider, GitProviderError};
use crate::services::BackgroundService;
use crate::state::AppState;

/// Validation status update parameters
struct ValidationUpdate {
    status: repository::ValidationStatus,
    branches: Vec<String>,
    has_branches: bool,
    has_labels: bool,
    can_manage_prs: bool,
    can_manage_issues: bool,
    message: Option<String>,
}

/// Repository service manages repository synchronization and validation
///
/// This service is designed to be stateless and thread-safe, supporting:
/// - Direct method calls from API handlers via `process_provider()`
/// - Periodic background synchronization via `sync_all_providers()`
pub struct RepositoryService {
    db: DatabaseConnection,
}

impl RepositoryService {
    /// Create a new repository service
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Get a clone of the database connection
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
    }

    /// Sync all providers (called by periodic task)
    pub async fn sync_all_providers(&self) -> Result<()> {
        let providers = RepoProvider::find().all(&self.db).await?;

        tracing::info!("Starting periodic sync for {} providers", providers.len());

        for provider in providers {
            if let Err(e) = self.process_provider(provider.id).await {
                tracing::error!("Failed to sync provider {}: {}", provider.id, e);
                // Continue with other providers
            }
        }

        Ok(())
    }

    /// Initialize a single repository by creating work branch and required labels
    ///
    /// This method creates the specified work branch from the default branch's latest commit,
    /// creates all required labels with vibe/ prefix, updates the database state, and
    /// re-validates the repository.
    ///
    /// The operation is idempotent - if the branch or labels already exist, it will update
    /// the database state to ensure consistency and return success.
    ///
    /// # Arguments
    /// * `repo_id` - The ID of the repository to initialize
    /// * `branch_name` - The name of the work branch to create (e.g., "vibe-dev")
    ///
    /// # Returns
    /// The updated repository model on success
    ///
    /// # Errors
    /// - `NotFound` - Repository or provider not found
    /// - `Forbidden` - Insufficient permissions to create branch or labels
    /// - `ServiceUnavailable` - Git provider unreachable
    /// - `Validation` - Default branch not found
    pub async fn initialize_repository(&self, repo_id: i32, branch_name: &str) -> Result<repository::Model> {
        tracing::info!("Initializing repository {}", repo_id);

        // 1. Fetch repository from database
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| GitAutoDevError::NotFound("Repository not found".to_string()))?;

        // 2. Fetch provider
        let provider = RepoProvider::find_by_id(repo.provider_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| GitAutoDevError::NotFound("Provider not found".to_string()))?;

        // 3. Create GitProvider client
        let git_client = GitClientFactory::from_provider(&provider).map_err(|e| {
            GitAutoDevError::Internal(format!("Failed to create git client: {}", e))
        })?;

        // 4. Parse owner/repo from full_name
        let (owner, repo_name) = self.parse_full_name(&repo.full_name)?;

        // 5. Try to create work branch
        let create_result = self
            .create_work_branch(&git_client, owner, repo_name, &repo.default_branch, branch_name)
            .await;

        // 6. Handle branch creation result
        match create_result {
            Ok(_) => {
                tracing::info!("Created {} branch for repository {}", branch_name, repo_id);
            }
            Err(GitAutoDevError::Conflict(_)) => {
                // Branch already exists - this is fine (idempotent operation)
                tracing::info!("{} branch already exists for repository {}", branch_name, repo_id);
            }
            Err(e) => {
                // Store error message and return
                self.update_validation_message_field(repo_id, Some(e.to_string()))
                    .await?;
                return Err(e);
            }
        }

        // 7. Try to create required labels
        if let Err(e) = self.create_required_labels(&git_client, owner, repo_name).await {
            tracing::warn!("Failed to create some labels for repository {}: {}", repo_id, e);
            // Continue - label creation failure should not block initialization
        }

        // 8. Re-fetch branches and update database
        let updated_branches = git_client
            .list_branches(owner, repo_name)
            .await
            .map_err(|e| self.map_git_error(e))?;

        let branch_names: Vec<String> = updated_branches.iter().map(|b| b.name.clone()).collect();
        let has_work_branch = branch_names.contains(&branch_name.to_string());

        // 9. Re-validate repository (check labels and permissions)
        let validation = self
            .validate_repository(&git_client, &repo.full_name)
            .await?;

        // 10. Calculate validation status
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

        // 11. Update repository in database
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| GitAutoDevError::NotFound("Repository not found".to_string()))?;

        let mut active: repository::ActiveModel = repo.into();
        active.branches = ActiveValue::Set(serde_json::json!(branch_names));
        active.has_required_branches = ActiveValue::Set(has_work_branch);
        active.has_required_labels = ActiveValue::Set(validation.has_required_labels);
        active.can_manage_prs = ActiveValue::Set(validation.can_manage_prs);
        active.can_manage_issues = ActiveValue::Set(validation.can_manage_issues);
        active.validation_status = ActiveValue::Set(status);
        active.validation_message = ActiveValue::Set(None);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());

        let updated = active.update(&self.db).await?;

        tracing::info!("Repository {} initialized successfully", repo_id);

        Ok(updated)
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
                    GitAutoDevError::Conflict("Branch already exists".to_string())
                }
                other => self.map_git_error(other),
            })?;

        Ok(())
    }

    /// Map GitProviderError to GitAutoDevError with appropriate status codes
    ///
    /// # Error Mapping
    /// - `Unauthorized` / `Forbidden` -> `Forbidden` (403)
    /// - `NetworkError` -> `ServiceUnavailable` (503)
    /// - `NotFound` (branch/ref) -> `Validation` (400)
    /// - `NotFound` (other) -> `NotFound` (404)
    /// - `BranchAlreadyExists` -> `Conflict` (409)
    /// - Others -> `Internal` (500)
    pub(crate) fn map_git_error(&self, error: GitProviderError) -> GitAutoDevError {
        match error {
            GitProviderError::Unauthorized(_) | GitProviderError::Forbidden(_) => {
                GitAutoDevError::Forbidden("Insufficient permissions to create branch".to_string())
            }
            GitProviderError::NotFound(msg) => {
                // Check if the error is about a branch or ref (source branch not found)
                let msg_lower = msg.to_lowercase();
                if msg_lower.contains("branch")
                    || msg_lower.contains("ref")
                    || msg_lower.contains("reference")
                {
                    GitAutoDevError::Validation("Default branch not found".to_string())
                } else {
                    GitAutoDevError::NotFound(msg)
                }
            }
            GitProviderError::NetworkError(_) => {
                GitAutoDevError::ServiceUnavailable("Git provider unreachable".to_string())
            }
            GitProviderError::BranchAlreadyExists(_) => {
                GitAutoDevError::Conflict("Branch already exists".to_string())
            }
            _ => GitAutoDevError::Internal(error.to_string()),
        }
    }

    /// Batch initialize all repositories for a provider
    ///
    /// This method initializes all repositories where `has_required_branches` OR
    /// `has_required_labels` is false for the specified provider. It continues
    /// processing even if some repositories fail to initialize.
    ///
    /// # Arguments
    /// * `provider_id` - The ID of the provider whose repositories should be initialized
    /// * `branch_name` - The name of the work branch to create (e.g., "vibe-dev")
    ///
    /// # Returns
    /// Ok(()) on completion (even if some repositories failed)
    ///
    /// # Errors
    /// - Database errors when fetching repositories
    pub async fn batch_initialize(&self, provider_id: i32, branch_name: &str) -> Result<()> {
        // 1. Fetch all repositories where has_required_branches OR has_required_labels is false
        let repos = Repository::find()
            .filter(repository::Column::ProviderId.eq(provider_id))
            .filter(
                sea_orm::Condition::any()
                    .add(repository::Column::HasRequiredBranches.eq(false))
                    .add(repository::Column::HasRequiredLabels.eq(false))
            )
            .all(&self.db)
            .await?;

        tracing::info!(
            "Batch initializing {} repositories for provider {}",
            repos.len(),
            provider_id
        );

        // 2. Initialize each repository, continuing on errors
        for repo in repos {
            match self.initialize_repository(repo.id, branch_name).await {
                Ok(_) => {
                    tracing::info!("Repository {} initialized successfully", repo.id);
                }
                Err(e) => {
                    tracing::error!("Failed to initialize repository {}: {}", repo.id, e);
                    // Continue with remaining repositories (Requirements 3.5)
                }
            }
        }

        tracing::info!(
            "Batch initialization completed for provider {}",
            provider_id
        );

        Ok(())
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
            .ok_or_else(|| GitAutoDevError::NotFound("Repository not found".to_string()))?;

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
            return Err(GitAutoDevError::Internal(format!(
                "Invalid repository full_name: {}",
                full_name
            )));
        }
        Ok((parts[0], parts[1]))
    }

    /// Process a provider - fetch and validate all repositories
    pub async fn process_provider(&self, provider_id: i32) -> Result<()> {
        tracing::info!("Processing provider {}", provider_id);

        // Fetch provider from database
        let provider = RepoProvider::find_by_id(provider_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                GitAutoDevError::NotFound(format!("Provider {} not found", provider_id))
            })?;

        // Create GitProvider client
        let git_client = GitClientFactory::from_provider(&provider).map_err(|e| {
            GitAutoDevError::Internal(format!("Failed to create git client: {}", e))
        })?;

        // Fetch repositories using GitProvider
        let repos = git_client.list_repositories().await.map_err(|e| {
            GitAutoDevError::Internal(format!("Failed to fetch repositories: {}", e))
        })?;

        tracing::info!(
            "Found {} repositories for provider {}",
            repos.len(),
            provider_id
        );

        // Process each repository
        for repo in repos {
            // Store repository with pending status
            let repo_id = self.store_repository(provider_id, &repo).await?;

            // Validate repository
            let validation = self
                .validate_repository(&git_client, &repo.full_name)
                .await;

            // Update validation status
            match validation {
                Ok(info) => {
                    // Valid only when all four conditions are met
                    let is_valid = info.has_required_branches
                        && info.has_required_labels
                        && info.can_manage_prs
                        && info.can_manage_issues;

                    let status = if is_valid {
                        repository::ValidationStatus::Valid
                    } else {
                        repository::ValidationStatus::Invalid
                    };

                    self.update_validation_status(
                        repo_id,
                        ValidationUpdate {
                            status,
                            branches: info.branches,
                            has_branches: info.has_required_branches,
                            has_labels: info.has_required_labels,
                            can_manage_prs: info.can_manage_prs,
                            can_manage_issues: info.can_manage_issues,
                            message: None,
                        },
                    )
                    .await?;
                }
                Err(e) => {
                    self.update_validation_status(
                        repo_id,
                        ValidationUpdate {
                            status: repository::ValidationStatus::Invalid,
                            branches: Vec::new(),
                            has_branches: false,
                            has_labels: false,
                            can_manage_prs: false,
                            can_manage_issues: false,
                            message: Some(e.to_string()),
                        },
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Store or update repository in database
    pub(crate) async fn store_repository(
        &self,
        provider_id: i32,
        repo: &crate::git_provider::GitRepository,
    ) -> Result<i32> {
        // Check if repository already exists
        let existing = Repository::find()
            .filter(repository::Column::ProviderId.eq(provider_id))
            .filter(repository::Column::FullName.eq(&repo.full_name))
            .one(&self.db)
            .await?;

        let repo_id = if let Some(existing_repo) = existing {
            // Update existing repository
            let mut active: repository::ActiveModel = existing_repo.into();
            active.name = ActiveValue::Set(repo.name.clone());
            active.clone_url = ActiveValue::Set(repo.clone_url.clone());
            active.default_branch = ActiveValue::Set(repo.default_branch.clone());
            active.validation_status = ActiveValue::Set(repository::ValidationStatus::Pending);
            active.updated_at = ActiveValue::Set(chrono::Utc::now());

            let updated = active.update(&self.db).await?;
            updated.id
        } else {
            // Create new repository
            let new_repo = repository::ActiveModel {
                provider_id: ActiveValue::Set(provider_id),
                name: ActiveValue::Set(repo.name.clone()),
                full_name: ActiveValue::Set(repo.full_name.clone()),
                clone_url: ActiveValue::Set(repo.clone_url.clone()),
                default_branch: ActiveValue::Set(repo.default_branch.clone()),
                branches: ActiveValue::Set(serde_json::json!([])),
                validation_status: ActiveValue::Set(repository::ValidationStatus::Pending),
                has_required_branches: ActiveValue::Set(false),
                has_required_labels: ActiveValue::Set(false),
                can_manage_prs: ActiveValue::Set(false),
                can_manage_issues: ActiveValue::Set(false),
                validation_message: ActiveValue::Set(None),
                created_at: ActiveValue::Set(chrono::Utc::now()),
                updated_at: ActiveValue::Set(chrono::Utc::now()),
                ..Default::default()
            };

            let inserted = new_repo.insert(&self.db).await?;
            inserted.id
        };

        Ok(repo_id)
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
            return Err(GitAutoDevError::Internal(format!(
                "Invalid repository full_name: {}",
                repo_full_name
            )));
        }
        let (owner, repo) = (parts[0], parts[1]);

        // Check branches (using default vibe-dev branch)
        let branch_info = self.check_branches(git_client, owner, repo, "vibe-dev").await?;

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
        let repository = git_client.get_repository(owner, repo).await.map_err(|e| {
            GitAutoDevError::Internal(format!("Failed to check permissions: {}", e))
        })?;

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
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to fetch branches: {}", e)))?;

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
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to fetch labels: {}", e)))?;

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
            .map_err(|e| GitAutoDevError::Internal(format!("Failed to fetch labels: {}", e)))?;

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

    /// Update repository validation status
    async fn update_validation_status(&self, repo_id: i32, update: ValidationUpdate) -> Result<()> {
        let repo = Repository::find_by_id(repo_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                GitAutoDevError::NotFound(format!("Repository {} not found", repo_id))
            })?;

        let mut active: repository::ActiveModel = repo.into();
        active.validation_status = ActiveValue::Set(update.status);
        active.branches = ActiveValue::Set(serde_json::json!(update.branches));
        active.has_required_branches = ActiveValue::Set(update.has_branches);
        active.has_required_labels = ActiveValue::Set(update.has_labels);
        active.can_manage_prs = ActiveValue::Set(update.can_manage_prs);
        active.can_manage_issues = ActiveValue::Set(update.can_manage_issues);
        active.validation_message = ActiveValue::Set(update.message);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());

        active.update(&self.db).await?;

        Ok(())
    }
}

#[async_trait]
impl BackgroundService for RepositoryService {
    fn name(&self) -> &'static str {
        "repository_service"
    }

    async fn start(&self, _state: Arc<AppState>) -> Result<()> {
        tracing::info!("RepositoryService started");

        // Spawn a periodic sync task (runs every hour)
        let db = self.db.clone();
        tokio::spawn(async move {
            let service = RepositoryService::new(db);
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));

            // Skip the first immediate tick
            interval.tick().await;

            loop {
                interval.tick().await;
                tracing::info!("Starting periodic repository sync");

                if let Err(e) = service.sync_all_providers().await {
                    tracing::error!("Periodic sync failed: {}", e);
                }
            }
        });

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
    use crate::api::repositories::models::REQUIRED_LABELS;

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
}
