# Capability: Repository Manual Add

## ADDED Requirements

### Requirement: User can manually add a repository with full configuration

The system SHALL allow users to add a repository by providing complete provider configuration (provider type, base URL, access token) and repository identifier in a single API call.

#### Scenario: Successfully add a repository with valid configuration
- **WHEN** user submits POST /api/repositories with valid provider_type, provider_base_url, access_token, full_name, and branch_name
- **THEN** system validates token permissions, creates repository record, creates workspace and agent, initializes branch and labels, creates webhook, and returns complete repository details

#### Scenario: Reject invalid provider type
- **WHEN** user submits POST /api/repositories with provider_type not in ["gitea", "github", "gitlab"]
- **THEN** system returns 400 Bad Request with error message "Invalid provider_type"

#### Scenario: Reject invalid access token
- **WHEN** user submits POST /api/repositories with access token that cannot authenticate with the provider
- **THEN** system returns 401 Unauthorized with error message "Invalid access token"

#### Scenario: Reject non-existent repository
- **WHEN** user submits POST /api/repositories with full_name that does not exist on the provider
- **THEN** system returns 404 Not Found with error message "Repository not found"

### Requirement: System validates token permissions before creating repository

The system SHALL validate that the provided access token has sufficient permissions (read repository, create branches, create labels, create webhooks, create pull requests, close issues) before storing the repository record.

#### Scenario: Accept token with all required permissions
- **WHEN** system validates token and token has all required permissions
- **THEN** system proceeds with repository creation

#### Scenario: Reject token with insufficient permissions
- **WHEN** system validates token and token lacks one or more required permissions
- **THEN** system returns 403 Forbidden with detailed error message listing missing permissions

#### Scenario: Validate token can create branches
- **WHEN** system validates token permissions
- **THEN** system attempts to verify branch creation permission and records result in validation_status

#### Scenario: Validate token can create labels
- **WHEN** system validates token permissions
- **THEN** system attempts to verify label creation permission and records result in validation_status

#### Scenario: Validate token can create webhooks
- **WHEN** system validates token permissions
- **THEN** system attempts to verify webhook creation permission and records result in validation_status

#### Scenario: Validate token can manage pull requests
- **WHEN** system validates token permissions
- **THEN** system attempts to verify pull request creation permission and records result in validation_status

#### Scenario: Validate token can manage issues
- **WHEN** system validates token permissions
- **THEN** system attempts to verify issue closure permission and records result in validation_status

### Requirement: System creates repository with self-contained provider configuration

The system SHALL store provider configuration (provider_type, provider_base_url, access_token, webhook_secret) directly in the repository record without foreign key dependencies.

#### Scenario: Store provider configuration in repository record
- **WHEN** system creates repository record
- **THEN** system stores provider_type, provider_base_url, access_token, and webhook_secret as fields in the repositories table

#### Scenario: Generate unique webhook secret per repository
- **WHEN** system creates repository record
- **THEN** system generates a cryptographically random webhook_secret and stores it with the repository

#### Scenario: Repository record is self-contained
- **WHEN** system retrieves repository record
- **THEN** system can create GitClient without additional database queries

### Requirement: System initializes repository in single atomic operation

The system SHALL create repository record, workspace, agent, branch, labels, and webhook as a single atomic operation that either fully succeeds or fully fails.

#### Scenario: All initialization steps succeed
- **WHEN** all initialization steps (repository creation, workspace creation, branch creation, label creation, webhook creation) succeed
- **THEN** system commits transaction and returns 201 Created with complete repository details

#### Scenario: Any initialization step fails
- **WHEN** any initialization step fails (e.g., branch creation fails)
- **THEN** system rolls back entire transaction and returns appropriate error status

#### Scenario: Workspace creation is part of atomic operation
- **WHEN** repository is created successfully
- **THEN** system creates workspace and agent in same transaction

#### Scenario: Branch initialization is part of atomic operation
- **WHEN** repository is created successfully
- **THEN** system creates vibe-dev branch (or custom branch_name) in same transaction

#### Scenario: Label creation is part of atomic operation
- **WHEN** repository is created successfully
- **THEN** system creates all required labels (vibe/pending-ack, vibe/todo-ai, vibe/in-progress, vibe/review-required, vibe/failed) in same transaction

#### Scenario: Webhook creation is part of atomic operation
- **WHEN** repository is created successfully
- **THEN** system creates webhook on provider in same transaction

### Requirement: API response includes complete repository details

The system SHALL return complete repository details including provider configuration (excluding access_token for security) in the API response.

#### Scenario: Response includes provider configuration
- **WHEN** repository is created successfully
- **THEN** response includes provider_type and provider_base_url fields

#### Scenario: Response excludes access token
- **WHEN** repository is created successfully
- **THEN** response does NOT include access_token field (security)

#### Scenario: Response includes validation status
- **WHEN** repository is created successfully
- **THEN** response includes validation_status, has_required_branches, has_required_labels, can_manage_prs, can_manage_issues fields

#### Scenario: Response includes webhook status
- **WHEN** repository is created successfully
- **THEN** response includes webhook_status field indicating webhook creation result
