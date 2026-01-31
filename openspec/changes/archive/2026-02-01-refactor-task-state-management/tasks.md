## 1. Add TaskStatus Enum Type

- [x] 1.1 Add TaskStatus enum to `backend/src/entities/task.rs` with DeriveActiveEnum macro
- [x] 1.2 Implement state machine methods: `can_transition_to()`, `allowed_transitions()`, `is_terminal()`
- [x] 1.3 Add unit tests for TaskStatus enum state transition logic
- [x] 1.4 Add Display and FromStr implementations for TaskStatus enum

## 2. Add Error Handling for Invalid Transitions

- [x] 2.1 Add `InvalidStateTransition` variant to VibeRepoError enum in `backend/src/error.rs`
- [x] 2.2 Implement Display for InvalidStateTransition with current state, target state, and allowed transitions
- [x] 2.3 Add HTTP status code mapping (400 Bad Request) for InvalidStateTransition errors
- [x] 2.4 Add unit tests for InvalidStateTransition error formatting

## 3. Update Task Entity Model

- [x] 3.1 Change `task_status` field type from String to TaskStatus in `backend/src/entities/task.rs`
- [x] 3.2 Update task::Model default values to use TaskStatus::Pending
- [x] 3.3 Verify entity compiles with SeaORM macros
- [x] 3.4 Run `cargo build` to identify all compilation errors from type change

## 4. Create Database Migration

- [x] 4.1 Create new migration file in `backend/src/migration/`
- [x] 4.2 Add validation query to check all existing task statuses are valid enum values
- [x] 4.3 Add error handling for invalid statuses found during migration
- [x] 4.4 Test migration on development database with sample data
- [x] 4.5 Test migration rollback functionality

## 5. Update TaskService Methods

- [x] 5.1 Add state transition validation to `assign_agent()` method
- [x] 5.2 Add state transition validation to `start_task()` method
- [x] 5.3 Add state transition validation to `complete_task()` method
- [x] 5.4 Add state transition validation to `fail_task()` method
- [x] 5.5 Add state transition validation to `retry_task()` method
- [x] 5.6 Add state transition validation to `cancel_task()` method
- [x] 5.7 Update `update_task_status()` to validate transitions
- [x] 5.8 Update `list_tasks_with_filters()` to use TaskStatus enum for filtering

## 6. Update TaskExecutorService

- [x] 6.1 Update status checks in `execute_task()` to use TaskStatus enum
- [x] 6.2 Update status comparisons to use enum variants instead of string literals
- [x] 6.3 Update error messages to use enum Display implementation
- [x] 6.4 Verify all status-related logic uses enum types

## 7. Update TaskSchedulerService

- [x] 7.1 Update task status filtering to use TaskStatus enum
- [x] 7.2 Update priority sorting logic to handle enum types
- [x] 7.3 Update status checks in scheduling logic
- [x] 7.4 Verify scheduler correctly filters tasks by enum status

## 8. Update API Layer

- [x] 8.1 Update task API models in `backend/src/api/task/models.rs` to use TaskStatus
- [x] 8.2 Update CreateTaskRequest to use TaskStatus enum
- [x] 8.3 Update TaskResponse to serialize TaskStatus as string
- [x] 8.4 Update API handlers to use TaskStatus enum
- [x] 8.5 Update OpenAPI documentation (utoipa schemas) for TaskStatus enum
- [x] 8.6 Verify Swagger UI displays enum values correctly

## 9. Update TaskService Tests

- [x] 9.1 Update `test_create_task_success` to use TaskStatus::Pending
- [x] 9.2 Update `test_update_task_status_success` to use TaskStatus enum
- [x] 9.3 Update `test_assign_agent_success` to use TaskStatus::Assigned
- [x] 9.4 Update `test_start_task_success` to use TaskStatus::Running
- [x] 9.5 Update `test_complete_task_success` to use TaskStatus::Completed
- [x] 9.6 Update `test_fail_task_with_retry` to use TaskStatus enum
- [x] 9.7 Update `test_fail_task_max_retries` to use TaskStatus::Failed
- [x] 9.8 Update `test_retry_task_success` to use TaskStatus enum
- [x] 9.9 Update `test_cancel_task_success` to use TaskStatus::Cancelled
- [x] 9.10 Update `test_list_tasks_with_filters_by_status` to use TaskStatus enum
- [x] 9.11 Add test for invalid transition from Completed to Failed
- [x] 9.12 Add test for invalid transition from Cancelled to Running
- [x] 9.13 Add test for invalid transition from Pending to Running
- [x] 9.14 Add test for terminal state validation (Completed cannot transition)
- [x] 9.15 Add test for terminal state validation (Cancelled cannot transition)

## 10. Update TaskExecutorService Tests

- [x] 10.1 Update status checks in executor tests to use TaskStatus enum
- [x] 10.2 Update test assertions to compare enum values
- [x] 10.3 Add test for validation error when executing non-pending/assigned task
- [x] 10.4 Verify all executor tests pass with enum types

## 11. Update Integration Tests

- [x] 11.1 Update task API integration tests to use TaskStatus enum in requests
- [x] 11.2 Update integration test assertions to check enum values in responses
- [x] 11.3 Add integration test for invalid state transition via API (expect 400 error)
- [x] 11.4 Add integration test for state transition error message format
- [x] 11.5 Verify all integration tests pass

## 12. Update Other Services

- [x] 12.1 Update IssuePollingService to use TaskStatus enum when creating tasks
- [x] 12.2 Update EventHandler to use TaskStatus enum
- [x] 12.3 Update PRCreationService status checks to use TaskStatus enum
- [x] 12.4 Update IssueClosureService status checks to use TaskStatus enum
- [x] 12.5 Search codebase for remaining string literal status values and update

## 13. Documentation and Cleanup

- [x] 13.1 Update API documentation to reflect TaskStatus enum values
- [x] 13.2 Add migration guide for API clients
- [x] 13.3 Update CHANGELOG.md with breaking changes
- [x] 13.4 Remove any unused string status constants
- [x] 13.5 Run `cargo clippy` and fix any warnings related to enum usage

## 14. Final Verification

- [x] 14.1 Run full test suite: `cargo test`
- [x] 14.2 Run integration tests with test database
- [x] 14.3 Test migration on fresh database
- [x] 14.4 Test migration on database with existing tasks
- [x] 14.5 Verify API responses serialize TaskStatus correctly
- [x] 14.6 Verify invalid state transitions return proper error responses
- [x] 14.7 Run `cargo build --release` to ensure production build succeeds
- [x] 14.8 Manual testing: Create task, assign agent, start, complete workflow
- [x] 14.9 Manual testing: Attempt invalid state transitions and verify errors
- [x] 14.10 Review all changes for consistency and completeness
