## Why

The current task state management system uses string-based status values without validation, allowing illegal state transitions that can corrupt data and break business logic. Tasks can be marked as failed after completion, completed without being started, or retried when already running. This creates race conditions, data inconsistencies, and unpredictable system behavior that undermines the reliability of the entire Issue-to-PR workflow.

## What Changes

- Replace string-based `task_status` field with type-safe `TaskStatus` enum
- Implement state machine with validated transitions between task states
- Add validation logic to prevent illegal state changes in all task service methods
- Update database schema to use enum type for task status
- Add migration to convert existing string statuses to enum values
- Update all API endpoints, tests, and documentation to use new enum type
- **BREAKING**: API responses will return enum values instead of arbitrary strings
- **BREAKING**: Invalid status transitions will now return validation errors instead of silently succeeding

## Capabilities

### New Capabilities
- `task-state-machine`: Defines the valid task states, allowed transitions, and validation rules for task lifecycle management

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

**Affected Code:**
- `backend/src/entities/task.rs` - Add TaskStatus enum
- `backend/src/services/task_service.rs` - Add state transition validation to all methods
- `backend/src/services/task_executor_service.rs` - Update status checks to use enum
- `backend/src/services/task_scheduler_service.rs` - Update status filtering to use enum
- `backend/src/api/task/` - Update API models and handlers
- `backend/src/migration/` - Add migration for enum conversion

**Affected APIs:**
- All task-related endpoints will return enum values in responses
- Invalid state transitions will return 400 Bad Request with validation errors

**Database:**
- Migration required to convert existing string statuses to enum values
- No data loss expected (all existing statuses map to valid enum values)

**Tests:**
- 50+ task-related tests need updates to use enum values
- New tests required for state transition validation
- Integration tests for concurrent state changes

**Dependencies:**
- No external dependency changes
- Internal services that check task status need updates
