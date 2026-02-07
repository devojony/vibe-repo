# Design: Per-Repository Provider Configuration

## Context

**Current Architecture**: Provider-Centric model with 4 tables (`repo_providers`, `repositories`, `webhook_configs`, `workspaces`). Every Git operation requires 2 database queries: first fetch repository, then fetch provider to get credentials. The system auto-syncs all repositories from a provider, forcing users to archive unwanted ones.

**Target Architecture**: Repository-Centric model with 2 tables (`repositories`, `workspaces`). Each repository is self-contained with its own provider configuration. Users explicitly add only the repositories they want.

**Constraints**:
- Pre-1.0 version (v0.4.0-mvp), breaking changes acceptable
- No data migration needed (fresh start acceptable)
- Personal use case (1-10 repositories, not enterprise scale)
- Auto-discovery explicitly not needed (user decision)

**Stakeholders**: Personal use, single developer

## Goals / Non-Goals

**Goals:**
- Simplify architecture: 4 tables → 2 tables, remove 100+ provider_id references
- Improve UX for selective usage: 3 API calls instead of 102 to add 3 of 100 repos
- Reduce runtime queries: 1 database query per operation (down from 2)
- Enable per-repository token management (principle of least privilege)
- Support mixed providers (GitHub + Gitea + GitLab in same workspace)

**Non-Goals:**
- Auto-discovery of repositories (explicitly excluded by user)
- Data migration from existing installations (fresh start acceptable)
- Token encryption (separate future improvement)
- Backward compatibility with v0.3.0 API (breaking change accepted)
- Provider-level batch operations (removed by design)

## Decisions

### Decision 1: Self-Contained Repository Model

**Choice**: Store provider configuration directly in `repositories` table  
**Alternatives Considered**:
- Keep `repo_providers` table, add optional per-repo override → Rejected: adds complexity, unclear mental model
- Hybrid model (provider OR per-repo config) → Rejected: two code paths to maintain, confusing UX

**Rationale**:
- Eliminates foreign key dependency and JOIN queries
- Simplifies code: no need to fetch provider before creating GitClient
- Matches user mental model: "I add a repository" not "I add a provider then sync repos"
- Enables per-repository token rotation without affecting other repos

**Trade-offs**:
- Data duplication: `provider_type` and `provider_base_url` repeated per repo
- Token management: N tokens to manage instead of 1
- Mitigation: Build bulk token update API for common operations

### Decision 2: Remove Auto-Discovery

**Choice**: Remove provider sync service and auto-discovery functionality  
**Alternatives Considered**:
- Keep sync as optional feature → Rejected: adds complexity, user explicitly doesn't need it
- Add "import from URL" as replacement → Accepted: will implement in Phase 2

**Rationale**:
- User explicitly stated auto-discovery not needed
- Removes ~500 lines of sync logic and tests
- Simplifies API surface (no provider management endpoints)
- Forces explicit intent: users add only what they want

**Trade-offs**:
- Users must know exact repository name (can't browse)
- More typing required per repository
- Mitigation: Build "import from URL" API that auto-detects provider config

### Decision 3: Inline Webhook Configuration

**Choice**: Store `webhook_secret` directly in `repositories` table  
**Alternatives Considered**:
- Keep `webhook_configs` table → Rejected: unnecessary indirection for 1:1 relationship
- Generate webhook secret on-the-fly → Rejected: need persistent secret for verification

**Rationale**:
- Webhook config is 1:1 with repository (no sharing)
- Eliminates another table and foreign key
- Simplifies webhook verification: single table lookup
- Reduces query count in webhook handler (hot path)

**Trade-offs**:
- Slightly wider `repositories` table
- Mitigation: Minimal impact, still well within reasonable column count

### Decision 4: Migration Strategy - Clean Break

**Choice**: Drop old tables, recreate `repositories` table with new schema  
**Alternatives Considered**:
- ALTER TABLE to add columns, migrate data → Rejected: no existing data to migrate
- Keep old tables for backward compat → Rejected: adds complexity, pre-1.0 allows breaking changes

**Rationale**:
- No production data to migrate (pre-1.0)
- Cleaner migration: no legacy columns or constraints
- Simpler testing: no need to test migration paths
- Faster implementation: no migration logic to write

**Trade-offs**:
- Users must re-add repositories after upgrade
- Mitigation: Document upgrade process, provide import tool if needed

### Decision 5: GitClientFactory Refactoring

**Choice**: Change `from_provider(provider: &Model)` to `from_repository(repo: &Model)`  
**Alternatives Considered**:
- Keep both methods → Rejected: dead code, confusing API
- Create new `create_from_repo()` → Rejected: inconsistent naming

**Rationale**:
- Single source of truth: repository contains all needed info
- Simplifies call sites: no provider fetch needed
- Type safety: compiler enforces new pattern

**Implementation**:
```rust
// Before
let provider = RepoProvider::find_by_id(repo.provider_id).one(&db).await?;
let client = GitClientFactory::from_provider(&provider)?;

// After
let client = GitClientFactory::from_repository(&repo)?;
```

**Impact**: 27 call sites to update across codebase

### Decision 6: API Design - Single-Step Repository Addition

**Choice**: `POST /api/repositories` creates repo + workspace + webhook in one operation  
**Alternatives Considered**:
- Multi-step: create repo, then initialize → Rejected: poor UX, more API calls
- Separate webhook creation → Rejected: webhook is essential, should be atomic

**Rationale**:
- Better UX: one API call does everything
- Atomic operation: all-or-nothing (no partial state)
- Matches user intent: "add this repository to VibeRepo"

**API Contract**:
```json
POST /api/repositories
{
  "provider_type": "gitea",
  "provider_base_url": "https://gitea.example.com",
  "access_token": "gto_xxxxx",
  "full_name": "owner/repo",
  "branch_name": "vibe-dev"
}
```

**Validation Steps**:
1. Create GitClient with provided credentials
2. Fetch repository info from provider (validates token + repo exists)
3. Validate token permissions (branches, labels, PRs, issues, webhooks)
4. Generate random webhook secret
5. Store repository record
6. Create workspace and agent
7. Initialize branch and labels
8. Create webhook on provider
9. Return complete repository record

### Decision 7: Token Security - Separate Concern

**Choice**: Keep tokens in plaintext for this change, add encryption later  
**Alternatives Considered**:
- Add encryption in this change → Rejected: scope creep, different problem domain
- Use environment variables → Rejected: doesn't scale to multiple repos

**Rationale**:
- Architectural change and security feature are orthogonal
- Encryption can be added to either architecture
- Smaller changes are easier to review and test
- Current architecture also stores tokens in plaintext (no regression)

**Future Work**: Add envelope encryption (master key + per-token keys) in separate change

## Risks / Trade-offs

### Risk: Token Management Burden
**Description**: Users must manage N tokens instead of 1  
**Impact**: Medium - more surface area for token leaks, more work to rotate  
**Mitigation**: 
- Build bulk token update API: `PUT /api/repositories/bulk-update-token`
- Document token rotation best practices
- Consider token expiration warnings in future

### Risk: Loss of Bulk Operations
**Description**: No provider-level batch operations (initialize all, archive all)  
**Impact**: Low - personal use typically involves few repos  
**Mitigation**:
- Implement repository-based bulk operations: `POST /api/repositories/batch-initialize` with `repository_ids` array
- Provide CLI tool for scripting bulk operations

### Risk: No Repository Browsing
**Description**: Users can't see available repositories before adding  
**Impact**: Medium - must know exact repository name  
**Mitigation**:
- Build "import from URL" API that auto-detects config
- Document how to find repository full_name on each provider
- Consider optional "preview" API in future (fetch repos without storing)

### Risk: Data Duplication
**Description**: `provider_type` and `provider_base_url` repeated per repository  
**Impact**: Low - minimal storage overhead, no consistency issues  
**Trade-off**: Accepted for simplicity and performance gains

### Risk: Breaking Change Impact
**Description**: Existing users must re-add all repositories  
**Impact**: Low - pre-1.0, small user base  
**Mitigation**:
- Clear upgrade documentation
- Provide export/import tool if needed
- Announce breaking change in release notes

## Migration Plan

### Phase 1: Database Schema (Day 1, 4 hours)

1. Create new migration file: `m20260207_per_repo_provider.rs`
2. Migration steps:
   ```sql
   DROP TABLE IF EXISTS webhook_configs;
   DROP TABLE IF EXISTS repo_providers;
   DROP TABLE IF EXISTS repositories;
   
   CREATE TABLE repositories (
     id INTEGER PRIMARY KEY,
     provider_type VARCHAR(20) NOT NULL,
     provider_base_url VARCHAR(255) NOT NULL,
     access_token TEXT NOT NULL,
     webhook_secret VARCHAR(255) NOT NULL,
     name VARCHAR(255) NOT NULL,
     full_name VARCHAR(255) NOT NULL,
     -- ... other existing fields
   );
   ```
3. Test migration on clean database
4. Verify all tables created correctly

### Phase 2: Entity Layer (Day 1, 4 hours)

1. Update `backend/src/entities/repository.rs`:
   - Add new fields: `provider_type`, `provider_base_url`, `access_token`, `webhook_secret`
   - Remove `provider_id` field
   - Remove `RepoProvider` relation
2. Delete files:
   - `backend/src/entities/repo_provider.rs`
   - `backend/src/entities/webhook_config.rs`
3. Update `backend/src/entities/mod.rs` to remove deleted entities
4. Run `cargo check` to find all compilation errors

### Phase 3: Factory and Services (Day 2, 8 hours)

1. Update `GitClientFactory` (2 hours):
   - Change `from_provider()` to `from_repository()`
   - Update all 27 call sites
2. Update `RepositoryService` (4 hours):
   - Remove: `sync_all_providers()`, `process_provider()`, `store_repository()`
   - Add: `add_repository()` method
   - Update: All methods that fetch provider
3. Update `PRCreationService` (1 hour):
   - Replace provider fetch with direct repo usage
4. Update `IssueClosureService` (1 hour):
   - Replace provider fetch with direct repo usage

### Phase 4: API Layer (Day 3, 4 hours)

1. Add new handler: `add_repository()` (2 hours)
2. Remove handlers: `batch_initialize_repositories()` and other provider-based batch ops (1 hour)
3. Update `list_repositories()`: remove `provider_id` filter (0.5 hour)
4. Update API models: `RepositoryResponse` (0.5 hour)

### Phase 5: Testing (Day 3, 4 hours)

1. Update test utilities: rewrite `create_test_provider()` helpers (1 hour)
2. Update unit tests: ~50 tests to modify (2 hours)
3. Update integration tests (1 hour)
4. Run full test suite: `cargo test`

### Phase 6: Documentation (Day 3-4, 2 hours)

1. Update API documentation
2. Update user guide with new workflow
3. Document breaking changes
4. Add migration guide for existing users

### Rollback Strategy

If critical issues discovered:
1. Revert to previous git commit
2. Restore database from backup (if any data exists)
3. Restart application

**Prevention**:
- Test thoroughly on staging environment
- Run full test suite before deployment
- Keep backup of database (if any data exists)

### Deployment Steps

1. Announce breaking change to users
2. Stop application
3. Backup database (if any data exists)
4. Pull new code
5. Run migrations: `cargo run` (auto-runs migrations)
6. Verify application starts successfully
7. Test adding a repository via API
8. Monitor logs for errors

## Open Questions

None - all decisions made based on deep exploration and user confirmation.

## Future Enhancements (Out of Scope)

1. **Token Encryption**: Add envelope encryption for access tokens
2. **Import from URL**: Auto-detect provider config from clone URL
3. **Bulk Token Update**: API for updating multiple repository tokens at once
4. **Repository Templates**: Save and reuse repository configurations
5. **Token Expiration Warnings**: Notify when tokens are about to expire
