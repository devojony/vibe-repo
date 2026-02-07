# Proposal: Per-Repository Provider Configuration

## Why

The current architecture is Provider-Centric: users configure a Git provider, the system auto-syncs ALL repositories, then users must archive unwanted ones. This creates a poor UX for selective usage (e.g., 100 repos available, only want 3 → requires 102 API operations). The architecture also adds unnecessary complexity: 4 database tables, 100+ provider_id references, and 2 database queries per operation. Since auto-discovery is not needed for personal use, we should move to a Repository-Centric architecture where users explicitly add only the repositories they want, with each repository storing its own provider configuration.

## What Changes

- **BREAKING**: Remove `repo_providers` table - provider configuration moves to repository level
- **BREAKING**: Remove `webhook_configs` table - webhook secrets stored per repository
- **BREAKING**: Remove `provider_id` foreign key from `repositories` table
- Add fields to `repositories` table: `provider_type`, `provider_base_url`, `access_token`, `webhook_secret`
- Add new API endpoint: `POST /api/repositories` for manual repository addition
- Remove API endpoints: batch operations by `provider_id` (batch-initialize, batch-archive, etc.)
- Remove query parameter: `?provider_id=` filter from `GET /api/repositories`
- Simplify query path: 1 database query per operation (down from 2)
- Delete entities: `repo_provider.rs`, `webhook_config.rs`
- Update all services to use repository fields directly instead of querying provider

**Note**: This is a breaking change, but no data migration is needed (fresh start is acceptable for pre-1.0).

## Capabilities

### New Capabilities

- `repository-manual-add`: Ability to manually add a repository by providing full configuration (provider type, base URL, access token, repository name). System validates token permissions, creates workspace, initializes branch/labels, and sets up webhook in a single operation.

### Modified Capabilities

- `repository-management`: Changes from auto-discovery model (sync all repos from provider, archive unwanted) to explicit addition model (add only desired repos). Removes provider-level batch operations.
- `webhook-handling`: Changes from using separate `webhook_configs` table to storing webhook secret directly in repository record. Webhook verification logic simplified to single-table lookup.

## Impact

**Database Schema**:
- Tables: 4 → 2 (remove `repo_providers`, `webhook_configs`)
- Foreign keys: Remove `repositories.provider_id` constraint
- New fields: `repositories.{provider_type, provider_base_url, access_token, webhook_secret}`

**Code Changes**:
- ~30 files to modify
- ~500-800 lines of code changes
- ~50 tests to rewrite
- Remove: Provider sync service, provider management API, webhook config API
- Simplify: GitClientFactory, all service operations (1 query vs 2)

**API Changes**:
- New: `POST /api/repositories` (manual add)
- Removed: `POST /api/repositories/batch-initialize?provider_id=X`
- Removed: `POST /api/repositories/batch-archive?provider_id=X`
- Removed: `GET /api/repositories?provider_id=X` filter
- Modified: All repository responses (remove `provider_id`, add `provider_type`, `provider_base_url`)

**Performance**:
- Database queries per operation: 2 → 1 (50% reduction)
- Response time improvement: ~5-10ms per operation

**User Experience**:
- Adding 3 of 100 repos: 102 operations → 3 operations (97% reduction)
- No auto-discovery (must know exact repository name)
- Per-repository token management (more granular, more work)

**Migration**:
- Estimated effort: 2-3 days
- Risk: Low (no data migration needed)
- Timing: Pre-1.0, breaking changes acceptable
