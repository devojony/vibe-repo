# Capability: Repository Management

## ADDED Requirements

### Requirement: Repository operations use self-contained configuration

The system SHALL perform all repository operations (initialize, refresh, archive, delete) using only the repository record without querying separate provider or webhook configuration tables.

#### Scenario: Initialize repository without provider query
- **WHEN** system initializes repository
- **THEN** system creates GitClient using repository's provider_type, provider_base_url, and access_token fields without additional database queries

#### Scenario: Refresh repository validation without provider query
- **WHEN** system refreshes repository validation status
- **THEN** system creates GitClient using repository's own fields without querying provider table

#### Scenario: Archive repository without provider dependency
- **WHEN** system archives repository
- **THEN** system updates repository status without checking provider table

#### Scenario: Delete repository without cascade constraints
- **WHEN** system deletes repository
- **THEN** system soft-deletes repository record without foreign key cascade operations

### Requirement: List repositories without provider filter

The system SHALL list repositories without provider_id filter, supporting only validation_status and status filters.

#### Scenario: List all repositories
- **WHEN** user requests GET /api/repositories without filters
- **THEN** system returns all non-deleted repositories

#### Scenario: Filter by validation status
- **WHEN** user requests GET /api/repositories?validation_status=valid
- **THEN** system returns only repositories with validation_status="valid"

#### Scenario: Filter by repository status
- **WHEN** user requests GET /api/repositories?status=idle
- **THEN** system returns only repositories with status="idle"

#### Scenario: Reject provider_id filter
- **WHEN** user requests GET /api/repositories?provider_id=1
- **THEN** system ignores provider_id parameter (no longer supported)

### Requirement: Repository responses include provider configuration

The system SHALL include provider_type and provider_base_url in all repository API responses, replacing provider_id field.

#### Scenario: Repository response includes provider fields
- **WHEN** system returns repository details
- **THEN** response includes provider_type and provider_base_url fields

#### Scenario: Repository response excludes provider_id
- **WHEN** system returns repository details
- **THEN** response does NOT include provider_id field (removed)

#### Scenario: Repository response excludes access_token
- **WHEN** system returns repository details
- **THEN** response does NOT include access_token field (security)

#### Scenario: Repository response excludes webhook_secret
- **WHEN** system returns repository details
- **THEN** response does NOT include webhook_secret field (security)

## REMOVED Requirements

### Requirement: System auto-syncs repositories from provider

**Reason**: Removed auto-discovery functionality - users now explicitly add only desired repositories

**Migration**: Use POST /api/repositories to manually add each repository with full configuration

### Requirement: Batch operations by provider_id

**Reason**: Removed provider-level batch operations - no longer have provider entity to group by

**Migration**: Use repository-based batch operations with repository_ids array, or call individual repository endpoints

### Requirement: Provider-level configuration management

**Reason**: Removed repo_providers table - configuration now stored per repository

**Migration**: Provide provider configuration when adding each repository via POST /api/repositories
