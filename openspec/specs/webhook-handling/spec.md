# Capability: Webhook Handling

## ADDED Requirements

### Requirement: Webhook secret stored in repository record

The system SHALL store webhook secret directly in the repository record instead of separate webhook_configs table.

#### Scenario: Generate webhook secret during repository creation
- **WHEN** system creates repository
- **THEN** system generates cryptographically random webhook_secret and stores it in repository.webhook_secret field

#### Scenario: Use repository webhook secret for verification
- **WHEN** system receives webhook request
- **THEN** system retrieves webhook_secret from repository record for signature verification

#### Scenario: Webhook secret is unique per repository
- **WHEN** system creates multiple repositories
- **THEN** each repository has a unique webhook_secret value

### Requirement: Webhook verification uses single table lookup

The system SHALL verify webhook signatures using only the repository record without querying separate webhook_configs table.

#### Scenario: Verify webhook with single database query
- **WHEN** system receives webhook request for repository
- **THEN** system fetches repository record (including webhook_secret) in single query

#### Scenario: Determine signature algorithm from provider type
- **WHEN** system verifies webhook signature
- **THEN** system uses repository.provider_type to determine signature algorithm (GitHub: HMAC-SHA256, Gitea: HMAC-SHA256)

#### Scenario: Verify signature using repository webhook secret
- **WHEN** system verifies webhook signature
- **THEN** system computes signature using repository.webhook_secret and compares with request signature header

#### Scenario: Reject webhook with invalid signature
- **WHEN** webhook signature does not match computed signature
- **THEN** system returns 401 Unauthorized and does not process webhook

#### Scenario: Accept webhook with valid signature
- **WHEN** webhook signature matches computed signature
- **THEN** system processes webhook event

### Requirement: Webhook creation uses repository configuration

The system SHALL create webhooks on Git provider using repository's provider configuration without querying separate tables.

#### Scenario: Create webhook using repository fields
- **WHEN** system creates webhook for repository
- **THEN** system creates GitClient using repository.provider_type, repository.provider_base_url, and repository.access_token

#### Scenario: Register webhook with repository secret
- **WHEN** system creates webhook on provider
- **THEN** system registers webhook with repository.webhook_secret as the secret

#### Scenario: Webhook URL includes repository identifier
- **WHEN** system creates webhook on provider
- **THEN** webhook URL is /api/webhooks/{repository_id} to route events to correct repository

#### Scenario: Update webhook_status after creation
- **WHEN** webhook creation succeeds
- **THEN** system updates repository.webhook_status to "active"

#### Scenario: Update webhook_status on failure
- **WHEN** webhook creation fails
- **THEN** system updates repository.webhook_status to "failed" with error message

## REMOVED Requirements

### Requirement: Webhook configuration in separate table

**Reason**: Removed webhook_configs table - webhook secret now stored directly in repository record for simplicity

**Migration**: Webhook secrets are now part of repository record, no separate configuration needed

### Requirement: Webhook configuration shared across repositories

**Reason**: Each repository now has its own webhook secret - no sharing of webhook configuration

**Migration**: Each repository gets unique webhook_secret during creation, no manual webhook configuration needed
