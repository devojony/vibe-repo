# Webhook Repository Association Refactor Plan

## Goal
Refactor webhook system to use repository_id instead of provider_id in webhook URLs, making the association clearer and more logical.

## Context
Currently, webhook URLs use provider_id (`/api/webhooks/{provider_id}`), but webhooks are actually per-repository. This creates confusion because:
1. Multiple repositories under one provider can't be distinguished by URL alone
2. The webhook_config table has both provider_id and repository_id, creating ambiguity
3. Cleanup logic checks repository existence, indicating the real association is with repository

## Tasks

### Task 1: Update Webhook URL Generation in RepositoryService
**File**: `backend/src/services/repository_service.rs`

**Changes**:
- Line ~405: Change webhook URL from `format!("{}/api/webhooks/{}", domain, provider.id)` to `format!("{}/api/webhooks/{}", domain, repo.id)`
- Update any related comments to reflect repository-based URLs

**Tests**:
- Run existing repository initialization tests
- Verify webhook URL format in test assertions

**Acceptance Criteria**:
- Webhook URLs use repository_id
- All existing tests pass
- No breaking changes to webhook creation logic

---

### Task 2: Update Webhook API Handler
**File**: `backend/src/api/webhooks/handlers.rs`

**Changes**:
- Line ~107-112: Change `Path(provider_id): Path<i32>` to `Path(repository_id): Path<i32>`
- Line ~18-86: Update `verify_webhook_request()` function:
  - Accept `repository_id` instead of `provider_id`
  - Query webhook_config by repository_id to get webhook_secret
  - Get provider from repository.provider_id for signature verification
- Update all logging statements to use repository_id
- Update OpenAPI documentation path parameter

**Tests**:
- Update webhook handler tests to use repository_id
- Verify signature verification still works
- Test error cases (repository not found, webhook config not found)

**Acceptance Criteria**:
- Handler accepts repository_id parameter
- Signature verification uses correct secret from webhook_config
- All webhook tests pass
- OpenAPI docs reflect new parameter

---

### Task 3: Update Webhook API Routes
**File**: `backend/src/api/webhooks/routes.rs`

**Changes**:
- Update route definition from `/webhooks/:provider_id` to `/webhooks/:repository_id`
- Update any route documentation

**Tests**:
- Verify route registration
- Test route matching with repository_id

**Acceptance Criteria**:
- Route uses repository_id parameter
- Route correctly maps to handler

---

### Task 4: Update Webhook Retry Service
**File**: `backend/src/services/webhook_retry_service.rs`

**Changes**:
- Line ~111-119: Update webhook URL generation in `retry_single_webhook()`:
  - Change from using provider_id to repository_id
  - Ensure webhook URL matches new format

**Tests**:
- Test webhook retry with new URL format
- Verify retry logic still works correctly

**Acceptance Criteria**:
- Retry service generates correct webhook URLs
- Retry tests pass
- Failed webhooks can be retried successfully

---

### Task 5: Update OpenAPI Documentation
**File**: `backend/src/api/mod.rs`

**Changes**:
- Update OpenAPI path documentation for webhook endpoint
- Ensure parameter descriptions are accurate

**Tests**:
- Verify OpenAPI spec generation
- Check Swagger UI displays correct parameter

**Acceptance Criteria**:
- OpenAPI spec shows repository_id parameter
- Swagger UI documentation is accurate

---

### Task 6: Update Integration Tests
**Files**: 
- `backend/tests/webhooks/*.rs`
- Any other tests that create or use webhooks

**Changes**:
- Update all test code that constructs webhook URLs
- Update mock webhook requests to use repository_id
- Update assertions that check webhook URLs

**Tests**:
- Run full test suite
- Verify all webhook-related tests pass

**Acceptance Criteria**:
- All integration tests pass
- Test coverage maintained
- No test regressions

---

### Task 7: Update Database Comments and Documentation
**Files**:
- `backend/src/entities/webhook_config.rs`
- `AGENTS.md`

**Changes**:
- Add comments clarifying that webhook_config.provider_id is a redundant field for optimization
- Update AGENTS.md to document the webhook URL format
- Add documentation explaining the relationship between webhook, repository, and provider

**Tests**:
- No code changes, documentation only

**Acceptance Criteria**:
- Clear documentation of webhook associations
- provider_id redundancy explained
- AGENTS.md updated with correct webhook URL format

---

## Migration Strategy

**Note**: This is a breaking change for existing webhooks. Options:

1. **Clean slate**: Delete all existing webhook_configs and let retry service recreate them
2. **Migration script**: Update webhook URLs in Git providers (requires API calls)
3. **Dual support**: Support both URL formats temporarily (complex)

**Recommendation**: Clean slate approach - simpler and safer for pre-1.0 project.

## Verification Steps

After all tasks complete:

1. Run full test suite: `cargo test`
2. Build project: `cargo build`
3. Manual verification:
   - Initialize a repository
   - Check webhook URL format in database
   - Trigger a webhook event (if possible)
   - Verify webhook is received and processed

## Rollback Plan

If issues arise:
1. Revert commits in reverse order
2. Existing webhooks will continue to work with old URL format
3. No data loss (only URL format changes)
