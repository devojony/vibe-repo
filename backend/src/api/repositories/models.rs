//! Repository API models
//!
//! Request and response DTOs for the Repository API.

#[cfg(test)]
use crate::entities::repository::RepositoryStatus;
use crate::entities::repository::{Model as RepositoryModel, ValidationStatus};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Required labels with vibe/ prefix for workflow management
pub const REQUIRED_LABELS: &[&str] = &[
    "vibe/pending-ack",
    "vibe/todo-ai",
    "vibe/in-progress",
    "vibe/review-required",
    "vibe/failed",
];

/// Repository response DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RepositoryResponse {
    /// Repository ID
    pub id: i32,
    /// Provider type (github, gitea, gitlab)
    pub provider_type: String,
    /// Provider base URL
    pub provider_base_url: String,
    /// Repository name (e.g., "my-repo")
    pub name: String,
    /// Full repository name (e.g., "owner/my-repo")
    pub full_name: String,
    /// Git clone URL
    pub clone_url: String,
    /// Default branch name
    pub default_branch: String,
    /// All branch names (JSON array)
    pub branches: Vec<String>,
    /// Validation status (valid, invalid, pending)
    pub validation_status: ValidationStatus,
    /// Has required branches (main/dev/developer)
    pub has_required_branches: bool,
    /// Has required issue labels
    pub has_required_labels: bool,
    /// Token has PR management permission
    pub can_manage_prs: bool,
    /// Token has Issue management permission
    pub can_manage_issues: bool,
    /// Validation error message (if invalid)
    pub validation_message: Option<String>,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last update timestamp (ISO 8601)
    pub updated_at: String,
}

impl RepositoryResponse {
    /// Convert entity model to response DTO
    ///
    /// Requirements: 12.5, 13.3
    /// Note: Excludes sensitive fields (access_token, webhook_secret)
    pub fn from_model(model: RepositoryModel) -> Self {
        // Parse branches JSON array
        let branches: Vec<String> =
            serde_json::from_value(model.branches).unwrap_or_else(|_| vec![]);

        Self {
            id: model.id,
            provider_type: model.provider_type,
            provider_base_url: model.provider_base_url,
            name: model.name,
            full_name: model.full_name,
            clone_url: model.clone_url,
            default_branch: model.default_branch,
            branches,
            validation_status: model.validation_status,
            has_required_branches: model.has_required_branches,
            has_required_labels: model.has_required_labels,
            can_manage_prs: model.can_manage_prs,
            can_manage_issues: model.can_manage_issues,
            validation_message: model.validation_message,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }
}

/// Request body for adding a new repository
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddRepositoryRequest {
    /// Provider type (github, gitea, gitlab)
    pub provider_type: String,
    /// Provider API base URL (e.g., "https://api.github.com" or "https://gitea.example.com")
    pub provider_base_url: String,
    /// Access token for authentication
    pub access_token: String,
    /// Full repository name (e.g., "owner/repo")
    pub full_name: String,
    /// Custom branch name for automated development (defaults to "vibe-dev")
    #[serde(default = "default_branch_name")]
    pub branch_name: String,
}

/// Request body for single repository initialization
#[derive(Debug, Deserialize, ToSchema)]
pub struct InitializeRepositoryRequest {
    /// Custom branch name for automated development (defaults to "vibe-dev")
    #[serde(default = "default_branch_name")]
    pub branch_name: String,
}

/// Default branch name for initialization
fn default_branch_name() -> String {
    "vibe-dev".to_string()
}

/// Request body for updating repository metadata
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRepositoryRequest {
    /// New repository name (optional)
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::repository::WebhookStatus;
    use chrono::Utc;

    #[test]
    fn test_from_model_converts_all_fields() {
        // Arrange: Create a test repository model
        let branches_json = serde_json::json!(["main", "dev", "feature-1"]);
        let now = Utc::now();

        let model = RepositoryModel {
            id: 1,
            provider_type: "gitea".to_string(),
            provider_base_url: "https://gitea.example.com".to_string(),
            access_token: "test_token".to_string(),
            webhook_secret: Some("test_secret".to_string()),
            name: "test-repo".to_string(),
            full_name: "owner/test-repo".to_string(),
            clone_url: "https://gitea.example.com/owner/test-repo.git".to_string(),
            default_branch: "main".to_string(),
            branches: branches_json.clone(),
            validation_status: ValidationStatus::Valid,
            status: RepositoryStatus::Idle,
            has_workspace: false,
            has_required_branches: true,
            has_required_labels: true,
            can_manage_prs: true,
            can_manage_issues: true,
            validation_message: None,
            webhook_status: WebhookStatus::Pending,
            agent_command: None,
            agent_timeout: 600,
            agent_env_vars: None,
            docker_image: "ubuntu:22.04".to_string(),
            deleted_at: None,
            created_at: now,
            updated_at: now,
        };

        // Act: Convert to response DTO
        let response = RepositoryResponse::from_model(model.clone());

        // Assert: All fields are correctly converted
        assert_eq!(response.id, 1);
        assert_eq!(response.provider_type, "gitea");
        assert_eq!(response.provider_base_url, "https://gitea.example.com");
        assert_eq!(response.name, "test-repo");
        assert_eq!(response.full_name, "owner/test-repo");
        assert_eq!(
            response.clone_url,
            "https://gitea.example.com/owner/test-repo.git"
        );
        assert_eq!(response.default_branch, "main");
        assert_eq!(response.branches, vec!["main", "dev", "feature-1"]);
        assert_eq!(response.validation_status, ValidationStatus::Valid);
        assert!(response.has_required_branches);
        assert!(response.has_required_labels);
        assert!(response.can_manage_prs);
        assert!(response.can_manage_issues);
        assert_eq!(response.validation_message, None);
        assert!(!response.created_at.is_empty());
        assert!(!response.updated_at.is_empty());
    }

    #[test]
    fn test_from_model_handles_invalid_status() {
        // Arrange: Create a repository with invalid status
        let now = Utc::now();

        let model = RepositoryModel {
            id: 2,
            provider_type: "gitea".to_string(),
            provider_base_url: "https://gitea.example.com".to_string(),
            access_token: "test_token".to_string(),
            webhook_secret: Some("test_secret".to_string()),
            name: "invalid-repo".to_string(),
            full_name: "owner/invalid-repo".to_string(),
            clone_url: "https://gitea.example.com/owner/invalid-repo.git".to_string(),
            default_branch: "main".to_string(),
            branches: serde_json::json!([]),
            validation_status: ValidationStatus::Invalid,
            status: RepositoryStatus::Unavailable,
            has_workspace: false,
            has_required_branches: false,
            has_required_labels: false,
            can_manage_prs: false,
            can_manage_issues: false,
            validation_message: Some("Missing required branches".to_string()),
            webhook_status: WebhookStatus::Pending,
            agent_command: None,
            agent_timeout: 600,
            agent_env_vars: None,
            docker_image: "ubuntu:22.04".to_string(),
            deleted_at: None,
            created_at: now,
            updated_at: now,
        };

        // Act: Convert to response DTO
        let response = RepositoryResponse::from_model(model);

        // Assert: Invalid status and message are preserved
        assert_eq!(response.validation_status, ValidationStatus::Invalid);
        assert!(!response.has_required_branches);
        assert!(!response.has_required_labels);
        assert!(!response.can_manage_prs);
        assert!(!response.can_manage_issues);
        assert_eq!(
            response.validation_message,
            Some("Missing required branches".to_string())
        );
    }

    #[test]
    fn test_from_model_formats_timestamps_as_iso8601() {
        // Arrange: Create a repository model
        let now = Utc::now();

        let model = RepositoryModel {
            id: 3,
            provider_type: "gitea".to_string(),
            provider_base_url: "https://gitea.example.com".to_string(),
            access_token: "test_token".to_string(),
            webhook_secret: Some("test_secret".to_string()),
            name: "test-repo".to_string(),
            full_name: "owner/test-repo".to_string(),
            clone_url: "https://gitea.example.com/owner/test-repo.git".to_string(),
            default_branch: "main".to_string(),
            branches: serde_json::json!([]),
            validation_status: ValidationStatus::Pending,
            status: RepositoryStatus::Uninitialized,
            has_workspace: false,
            has_required_branches: false,
            has_required_labels: false,
            can_manage_prs: false,
            can_manage_issues: false,
            validation_message: None,
            webhook_status: WebhookStatus::Pending,
            agent_command: None,
            agent_timeout: 600,
            agent_env_vars: None,
            docker_image: "ubuntu:22.04".to_string(),
            deleted_at: None,
            created_at: now,
            updated_at: now,
        };

        // Act: Convert to response DTO
        let response = RepositoryResponse::from_model(model);

        // Assert: Timestamps are in ISO 8601 format (RFC3339)
        assert!(response.created_at.contains('T'));
        assert!(response.created_at.contains('Z') || response.created_at.contains('+'));
        assert!(response.updated_at.contains('T'));
        assert!(response.updated_at.contains('Z') || response.updated_at.contains('+'));
    }

    #[test]
    fn test_from_model_handles_empty_branches() {
        // Arrange: Create a repository with empty branches
        let now = Utc::now();

        let model = RepositoryModel {
            id: 4,
            provider_type: "gitea".to_string(),
            provider_base_url: "https://gitea.example.com".to_string(),
            access_token: "test_token".to_string(),
            webhook_secret: Some("test_secret".to_string()),
            name: "empty-branches".to_string(),
            full_name: "owner/empty-branches".to_string(),
            clone_url: "https://gitea.example.com/owner/empty-branches.git".to_string(),
            default_branch: "main".to_string(),
            branches: serde_json::json!([]),
            validation_status: ValidationStatus::Pending,
            status: RepositoryStatus::Uninitialized,
            has_workspace: false,
            has_required_branches: false,
            has_required_labels: false,
            can_manage_prs: false,
            can_manage_issues: false,
            validation_message: None,
            webhook_status: WebhookStatus::Pending,
            agent_command: None,
            agent_timeout: 600,
            agent_env_vars: None,
            docker_image: "ubuntu:22.04".to_string(),
            deleted_at: None,
            created_at: now,
            updated_at: now,
        };

        // Act: Convert to response DTO
        let response = RepositoryResponse::from_model(model);

        // Assert: Empty branches array is handled correctly
        assert_eq!(response.branches, Vec::<String>::new());
    }

    #[test]
    fn test_initialize_request_default_branch_name() {
        // Arrange & Act: Deserialize empty JSON object
        let json = "{}";
        let request: InitializeRepositoryRequest = serde_json::from_str(json).unwrap();

        // Assert: Default branch name is "vibe-dev"
        assert_eq!(request.branch_name, "vibe-dev");
    }

    #[test]
    fn test_initialize_request_custom_branch_name() {
        // Arrange & Act: Deserialize JSON with custom branch name
        let json = r#"{"branch_name": "custom-branch"}"#;
        let request: InitializeRepositoryRequest = serde_json::from_str(json).unwrap();

        // Assert: Custom branch name is preserved
        assert_eq!(request.branch_name, "custom-branch");
    }

    #[test]
    fn test_required_labels_constant() {
        // Assert: REQUIRED_LABELS contains all expected labels with vibe/ prefix
        assert_eq!(REQUIRED_LABELS.len(), 5);
        assert!(REQUIRED_LABELS.contains(&"vibe/pending-ack"));
        assert!(REQUIRED_LABELS.contains(&"vibe/todo-ai"));
        assert!(REQUIRED_LABELS.contains(&"vibe/in-progress"));
        assert!(REQUIRED_LABELS.contains(&"vibe/review-required"));
        assert!(REQUIRED_LABELS.contains(&"vibe/failed"));
    }
}
