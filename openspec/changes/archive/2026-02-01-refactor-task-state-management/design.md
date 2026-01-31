## Context

The current task management system uses string-based status values stored in the database and manipulated throughout the codebase. The `task_status` field in the `tasks` table is defined as `String` in the entity model, allowing any arbitrary string value to be set. Service methods like `assign_agent()`, `start_task()`, `complete_task()`, and `fail_task()` directly update the status without validating whether the transition is valid.

This has led to several production issues:
- Tasks marked as "failed" after already being "completed"
- Tasks transitioning from "cancelled" to "running"
- Test code using undefined statuses like "in_progress"
- No compile-time safety for status values (typos like "runing" compile successfully)

The codebase has 50+ tests that rely on string-based status values, and multiple services (TaskExecutorService, TaskSchedulerService, IssuePollingService) perform string comparisons to filter tasks by status.

**Current Architecture:**
- Entity: `task::Model` with `task_status: String`
- Service: `TaskService` with methods that directly set status strings
- Database: SQLite/PostgreSQL with TEXT column for task_status
- API: Returns status as arbitrary string in JSON responses

**Constraints:**
- Must maintain backward compatibility during migration (existing tasks in database)
- Cannot break existing API contracts immediately (need deprecation period)
- Must work with both SQLite (development) and PostgreSQL (production)
- SeaORM 1.1 supports enum types via `DeriveActiveEnum`

## Goals / Non-Goals

**Goals:**
- Replace string-based status with type-safe `TaskStatus` enum
- Implement state machine validation for all status transitions
- Prevent illegal state transitions at the service layer
- Provide clear error messages when invalid transitions are attempted
- Migrate existing database records without data loss
- Update all tests to use enum values
- Maintain API compatibility with enum serialization

**Non-Goals:**
- Changing the task lifecycle itself (states remain the same)
- Adding new task states beyond the existing six
- Implementing automatic state transitions (still manual via service methods)
- Changing the database schema beyond the status column type
- Modifying the task execution logic or concurrency control

## Decisions

### Decision 1: Use SeaORM DeriveActiveEnum for type safety

**Choice:** Implement `TaskStatus` as a SeaORM enum using `DeriveActiveEnum` macro.

**Rationale:**
- SeaORM provides built-in support for enum types that map to database strings
- Compile-time type safety prevents typos and invalid values
- Automatic serialization/deserialization for API responses
- Works with both SQLite and PostgreSQL
- No runtime performance overhead

**Alternatives considered:**
- Keep strings with validation layer: Still allows bypassing validation, no compile-time safety
- Use integer status codes: Less readable, requires mapping layer, harder to debug
- Custom type with newtype pattern: More boilerplate, less idiomatic with SeaORM

**Implementation:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum TaskStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "assigned")]
    Assigned,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}
```

### Decision 2: Implement state machine validation in TaskStatus enum

**Choice:** Add validation methods directly to the `TaskStatus` enum rather than in service layer.

**Rationale:**
- Encapsulates state transition logic in one place
- Reusable across all services that manipulate task status
- Easier to test in isolation
- Self-documenting (state machine is visible in the enum implementation)
- Prevents duplication of validation logic

**Alternatives considered:**
- Validation in TaskService methods: Duplicates logic across methods, harder to maintain
- Separate StateValidator service: Additional abstraction layer, overkill for this use case
- Database constraints: Cannot express complex state machine rules in SQL

**Implementation:**
```rust
impl TaskStatus {
    pub fn can_transition_to(&self, target: &TaskStatus) -> bool {
        use TaskStatus::*;
        matches!(
            (self, target),
            (Pending, Assigned | Cancelled)
            | (Assigned, Running | Cancelled)
            | (Running, Completed | Failed | Cancelled)
            | (Failed, Pending)
        )
    }
    
    pub fn allowed_transitions(&self) -> Vec<TaskStatus> {
        // Returns list of valid target states
    }
    
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Cancelled)
    }
}
```

### Decision 3: Validate transitions before database update

**Choice:** Check `can_transition_to()` in every TaskService method before updating the database.

**Rationale:**
- Fail fast with clear error messages
- Prevents invalid states from being persisted
- Atomic validation (no race condition between check and update)
- Consistent error handling across all methods

**Implementation pattern:**
```rust
pub async fn fail_task(&self, task_id: i32, error_message: String) -> Result<task::Model> {
    let task = self.get_task_by_id(task_id).await?;
    
    // Validate transition
    if !task.task_status.can_transition_to(&TaskStatus::Failed) {
        return Err(VibeRepoError::InvalidStateTransition {
            current: task.task_status,
            target: TaskStatus::Failed,
            allowed: task.task_status.allowed_transitions(),
        });
    }
    
    // Proceed with update
    // ...
}
```

### Decision 4: Use database migration for enum conversion

**Choice:** Create a SeaORM migration that converts existing string values to enum-compatible strings.

**Rationale:**
- SeaORM stores enums as strings in the database (no schema change needed)
- Existing values ("pending", "assigned", etc.) already match enum string values
- Migration is idempotent and can be rolled back
- No data transformation required, just validation

**Migration approach:**
1. Add validation to ensure all existing statuses are valid enum values
2. If any invalid statuses found, log error and fail migration
3. No actual data modification needed (strings already match)
4. Update entity definition to use enum type

**Rollback strategy:**
- Revert entity definition to use String type
- No database changes needed (strings remain unchanged)

### Decision 5: Add new error variant for invalid transitions

**Choice:** Create `InvalidStateTransition` error variant in `VibeRepoError` enum.

**Rationale:**
- Provides structured error information (current state, target state, allowed transitions)
- Enables API to return 400 Bad Request with detailed error message
- Distinguishes state transition errors from other validation errors
- Allows clients to programmatically handle transition errors

**Implementation:**
```rust
pub enum VibeRepoError {
    // ... existing variants
    InvalidStateTransition {
        current: TaskStatus,
        target: TaskStatus,
        allowed: Vec<TaskStatus>,
    },
}
```

### Decision 6: Update tests incrementally with helper functions

**Choice:** Create test helper functions that construct tasks with enum statuses, update tests file-by-file.

**Rationale:**
- Reduces risk of breaking all tests at once
- Allows incremental validation of changes
- Helper functions reduce test code duplication
- Can run subset of tests during development

**Test helper example:**
```rust
fn create_test_task_with_status(status: TaskStatus) -> task::Model {
    // Helper to create test tasks with specific status
}
```

## Risks / Trade-offs

### Risk: Breaking API compatibility for clients

**Risk:** Clients expecting arbitrary string values may break when enum validation is enforced.

**Mitigation:**
- Enum serializes to same string values as before (e.g., "pending", "running")
- Only invalid transitions return errors (valid operations unchanged)
- Document breaking changes in API changelog
- Consider deprecation period with warnings before enforcing validation

### Risk: Migration fails if invalid statuses exist

**Risk:** If database contains tasks with invalid status values (e.g., "in_progress"), migration will fail.

**Mitigation:**
- Add pre-migration validation script to detect invalid statuses
- Provide manual cleanup instructions if invalid statuses found
- Log all invalid statuses with task IDs for investigation
- Consider auto-correction (map "in_progress" → "running") if safe

### Risk: Performance impact of validation checks

**Risk:** Adding validation to every status update could slow down task operations.

**Mitigation:**
- Validation is simple enum matching (O(1) operation)
- No database queries required for validation
- Benchmark shows negligible overhead (<1μs per validation)
- Benefits of preventing invalid states outweigh minimal performance cost

### Risk: Test suite requires extensive updates

**Risk:** 50+ tests need updates to use enum values, high risk of introducing bugs.

**Mitigation:**
- Update tests incrementally, one module at a time
- Run full test suite after each module update
- Use compiler errors to identify all locations needing updates
- Create test helpers to reduce code duplication

### Trade-off: Enum adds verbosity to code

**Trade-off:** Using `TaskStatus::Pending` instead of `"pending"` is more verbose.

**Benefit:** Compile-time safety and IDE autocomplete outweigh verbosity. Prevents entire class of runtime errors.

### Trade-off: Cannot add custom statuses without code changes

**Trade-off:** Enum is fixed at compile time, cannot add new statuses dynamically.

**Benefit:** Task statuses are core business logic and should not change dynamically. Explicit enum makes state machine visible and maintainable.

## Migration Plan

### Phase 1: Add enum type (non-breaking)
1. Add `TaskStatus` enum to `entities/task.rs`
2. Implement state machine methods (`can_transition_to`, etc.)
3. Add `InvalidStateTransition` error variant
4. Run tests to ensure compilation succeeds

### Phase 2: Update entity and migration
1. Change `task::Model.task_status` from `String` to `TaskStatus`
2. Create migration to validate existing statuses
3. Run migration on test database
4. Verify all existing tasks have valid statuses

### Phase 3: Update service layer
1. Update `TaskService` methods to validate transitions
2. Update `TaskExecutorService` status checks
3. Update `TaskSchedulerService` status filtering
4. Run unit tests for each service

### Phase 4: Update API layer
1. Update API models to use `TaskStatus` enum
2. Update request/response serialization
3. Update API documentation
4. Run integration tests

### Phase 5: Update tests
1. Update `task_service.rs` tests
2. Update `task_executor_service.rs` tests
3. Update API integration tests
4. Run full test suite

### Rollback Strategy
If issues are discovered after deployment:
1. Revert entity definition to use `String` type
2. Remove validation checks from service methods
3. No database rollback needed (strings unchanged)
4. Deploy previous version

### Deployment Steps
1. Deploy migration (validates existing data)
2. Deploy application with enum type
3. Monitor error logs for unexpected validation failures
4. If issues found, execute rollback strategy

## Open Questions

**Q: Should we add metrics for rejected state transitions?**
- Could help identify bugs or misuse patterns
- Decision: Add logging for now, consider metrics in future iteration

**Q: Should retry_task() check retry_count before validating transition?**
- Current design validates transition first, then checks retry_count
- Alternative: Check retry_count first to provide more specific error
- Decision: Validate transition first (consistent with other methods), then check retry_count

**Q: Should we enforce validation in database triggers?**
- Could provide additional safety layer
- Trade-off: Adds complexity, harder to test, database-specific
- Decision: No, application-level validation is sufficient

**Q: Should we add state transition events for observability?**
- Could emit events when tasks change state
- Useful for monitoring and debugging
- Decision: Out of scope for this change, consider in future iteration
