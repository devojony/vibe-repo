# PR Creation Implementation Plan

**Date:** 2026-01-21  
**Version:** 0.5.0  
**Status:** Implementation Ready

## Overview

Implement automatic Pull Request creation when tasks complete successfully, with automatic Issue linking and closure when PRs are merged. This completes the final 10% of the Issue-to-PR automation workflow.

## Current State

**Completed (90%):**
- ✅ Issue detection (webhook + polling)
- ✅ Task creation from issues
- ✅ Agent assignment
- ✅ Task execution in containers
- ✅ PR information extraction from execution output
- ✅ Git Provider API (`create_pull_request`, `update_issue`)

**Missing (10%):**
- 🟡 PR creation service after task completion
- 🟡 Issue linking in PR body
- 🟡 Issue closure when PR is merged

## Requirements

### Functional Requirements

1. **PR Creation After Task Completion**
   - When a task completes successfully with `branch_name` extracted
   - Create PR automatically via Git Provider API
   - PR title should match issue title
   - PR body should reference the issue (e.g., "Closes #123")
   - Update task record with `pr_number` and `pr_url`

2. **Issue Linking**
   - PR body must include issue reference for automatic linking
   - Use standard format: "Closes #<issue_number>" or "Fixes #<issue_number>"
   - Preserve any additional context from task execution

3. **Issue Closure on PR Merge**
   - Detect PR merge events via webhook
   - Close linked issue automatically
   - Update task status to "completed"

### Non-Functional Requirements

1. **Reliability**
   - Handle Git Provider API failures gracefully
   - Retry on transient errors
   - Log all PR creation attempts

2. **Idempotency**
   - Don't create duplicate PRs for the same task
   - Check if PR already exists before creating

3. **Testing**
   - Unit tests for PR creation logic
   - Integration tests with mock Git Provider
   - Test error handling scenarios

## Architecture

### Component Overview

```
TaskExecutorService
    ↓ (task completes)
PRCreationService
    ↓ (create PR)
GitProvider API
    ↓ (PR created)
Update Task Record
    ↓ (webhook: PR merged)
IssueClosureService
    ↓ (close issue)
GitProvider API
```

### Data Flow

1. **Task Execution Completes**
   - `TaskExecutorService` finishes execution
   - Extracts `branch_name`, `pr_number`, `pr_url` from output
   - If `branch_name` exists but `pr_number` is None → trigger PR creation

2. **PR Creation**
   - `PRCreationService` receives task_id
   - Loads task, workspace, repository details
   - Constructs PR request:
     - title: task.issue_title
     - body: "Closes #<issue_number>\n\n<additional context>"
     - head: task.branch_name
     - base: repository default branch (usually "main" or "master")
   - Calls `GitProvider::create_pull_request()`
   - Updates task with pr_number and pr_url

3. **PR Merge Detection**
   - Webhook receives "pull_request" event with action="closed" and merged=true
   - Finds task by pr_number
   - Triggers issue closure

4. **Issue Closure**
   - `IssueClosureService` receives task_id
   - Calls `GitProvider::update_issue()` with state="closed"
   - Updates task status to "completed"

## Implementation Tasks

### Task 1: Create PRCreationService

**File:** `backend/src/services/pr_creation_service.rs`

**Responsibilities:**
- Create PR via Git Provider API
- Update task record with PR information
- Handle errors and retries

**Key Methods:**
```rust
pub struct PRCreationService {
    db: DatabaseConnection,
}

impl PRCreationService {
    pub fn new(db: DatabaseConnection) -> Self;
    
    /// Create PR for a completed task
    /// Returns Ok(()) if PR created or already exists
    /// Returns Err if creation fails
    pub async fn create_pr_for_task(&self, task_id: i32) -> Result<()>;
    
    /// Check if PR already exists for this task
    async fn pr_already_exists(&self, task: &task::Model) -> Result<bool>;
    
    /// Build PR body with issue reference
    fn build_pr_body(&self, issue_number: i32, additional_context: Option<&str>) -> String;
    
    /// Get repository default branch
    async fn get_default_branch(&self, workspace: &workspace::Model) -> Result<String>;
}
```

**Implementation Details:**
- Check `task.pr_number` - if already set, skip creation
- Validate `task.branch_name` exists
- Load workspace and repository details
- Parse repository owner/name from clone_url
- Create Git Provider client
- Call `create_pull_request()` with:
  - title: `task.issue_title`
  - body: `format!("Closes #{}\n\n{}", issue_number, context)`
  - head: `task.branch_name`
  - base: default branch (from repository or "main")
- Update task with `pr_number` and `pr_url`
- Log success/failure

**Error Handling:**
- `BranchNotFound`: Log warning, don't fail task
- `PRAlreadyExists`: Update task record, return Ok
- `NetworkError`: Retry up to 3 times
- Other errors: Log and return error

**Tests:**
- `test_create_pr_for_task_success`
- `test_create_pr_skips_if_already_exists`
- `test_create_pr_fails_if_no_branch_name`
- `test_create_pr_builds_correct_body`
- `test_create_pr_handles_network_errors`

---

### Task 2: Integrate PR Creation into TaskExecutorService

**File:** `backend/src/services/task_executor_service.rs`

**Changes:**
- Add `pr_creation_service: PRCreationService` field
- After task completes successfully, check if `branch_name` exists
- If yes and `pr_number` is None, call `pr_creation_service.create_pr_for_task()`
- Log PR creation result

**Implementation Details:**
```rust
// In execute_task() method, after updating task status to completed:
if task.branch_name.is_some() && task.pr_number.is_none() {
    info!(task_id = task.id, "Task completed with branch, creating PR");
    
    match self.pr_creation_service.create_pr_for_task(task.id).await {
        Ok(()) => {
            info!(task_id = task.id, "PR created successfully");
        }
        Err(e) => {
            error!(task_id = task.id, error = %e, "Failed to create PR");
            // Don't fail the task, just log the error
        }
    }
}
```

**Tests:**
- `test_execute_task_creates_pr_on_success`
- `test_execute_task_skips_pr_if_no_branch`
- `test_execute_task_continues_if_pr_creation_fails`

---

### Task 3: Create IssueClosureService

**File:** `backend/src/services/issue_closure_service.rs`

**Responsibilities:**
- Close issue when PR is merged
- Update task status to completed

**Key Methods:**
```rust
pub struct IssueClosureService {
    db: DatabaseConnection,
}

impl IssueClosureService {
    pub fn new(db: DatabaseConnection) -> Self;
    
    /// Close issue for a task whose PR was merged
    pub async fn close_issue_for_task(&self, task_id: i32) -> Result<()>;
    
    /// Close issue via Git Provider API
    async fn close_issue_via_api(
        &self,
        git_client: &GitClient,
        owner: &str,
        repo: &str,
        issue_number: i32,
    ) -> Result<()>;
}
```

**Implementation Details:**
- Load task by ID
- Validate task has `pr_number` (PR was created)
- Load workspace and repository
- Create Git Provider client
- Call `update_issue()` with `state: IssueState::Closed`
- Update task status to "completed"
- Log success/failure

**Error Handling:**
- `IssueNotFound`: Log warning, still mark task completed
- `IssueAlreadyClosed`: Log info, mark task completed
- `NetworkError`: Retry up to 3 times
- Other errors: Log and return error

**Tests:**
- `test_close_issue_for_task_success`
- `test_close_issue_fails_if_no_pr_number`
- `test_close_issue_handles_already_closed`
- `test_close_issue_handles_network_errors`

---

### Task 4: Add PR Merge Webhook Handler

**File:** `backend/src/api/webhooks/handlers.rs`

**Changes:**
- Extend webhook handler to detect PR merge events
- When `event_type == "pull_request"` and `action == "closed"` and `merged == true`:
  - Find task by `pr_number`
  - Call `IssueClosureService::close_issue_for_task()`

**Implementation Details:**
```rust
// In handle_webhook() function:
if event_type == "pull_request" {
    let action = payload.get("action").and_then(|v| v.as_str());
    let merged = payload.get("pull_request")
        .and_then(|pr| pr.get("merged"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    if action == Some("closed") && merged {
        let pr_number = payload.get("pull_request")
            .and_then(|pr| pr.get("number"))
            .and_then(|v| v.as_i64())
            .map(|n| n as i32);
        
        if let Some(pr_num) = pr_number {
            // Find task by pr_number
            let task = Task::find()
                .filter(task::Column::PrNumber.eq(pr_num))
                .filter(task::Column::WorkspaceId.eq(workspace_id))
                .one(&state.db)
                .await?;
            
            if let Some(task) = task {
                let closure_service = IssueClosureService::new(state.db.clone());
                if let Err(e) = closure_service.close_issue_for_task(task.id).await {
                    error!(task_id = task.id, error = %e, "Failed to close issue");
                }
            }
        }
    }
}
```

**Tests:**
- `test_webhook_closes_issue_on_pr_merge`
- `test_webhook_ignores_pr_close_without_merge`
- `test_webhook_handles_missing_task`

---

### Task 5: Add API Endpoints for Manual PR Operations

**File:** `backend/src/api/tasks/handlers.rs`

**New Endpoints:**

1. **POST /api/tasks/{id}/create-pr** - Manually trigger PR creation
2. **POST /api/tasks/{id}/close-issue** - Manually close linked issue

**Implementation:**
```rust
/// Manually create PR for a task
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/create-pr",
    responses(
        (status = 200, description = "PR created successfully"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Task not found"),
    ),
    tag = "tasks"
)]
pub async fn create_pr_for_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = PRCreationService::new(state.db.clone());
    service.create_pr_for_task(id).await?;
    
    let task_service = TaskService::new(state.db.clone());
    let task = task_service.get_task_by_id(id).await?;
    
    Ok(Json(task.into()))
}

/// Manually close issue for a task
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/close-issue",
    responses(
        (status = 200, description = "Issue closed successfully"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Task not found"),
    ),
    tag = "tasks"
)]
pub async fn close_issue_for_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<TaskResponse>> {
    let service = IssueClosureService::new(state.db.clone());
    service.close_issue_for_task(id).await?;
    
    let task_service = TaskService::new(state.db.clone());
    let task = task_service.get_task_by_id(id).await?;
    
    Ok(Json(task.into()))
}
```

**Tests:**
- `test_create_pr_endpoint_success`
- `test_create_pr_endpoint_not_found`
- `test_close_issue_endpoint_success`
- `test_close_issue_endpoint_not_found`

---

### Task 6: Update Documentation

**Files to Update:**
- `docs/api/user-guide.md` - Add PR creation workflow
- `docs/api/api-reference.md` - Document new endpoints
- `docs/roadmap/README.md` - Mark PR creation as completed
- `AGENTS.md` - Update completion status to 100%

**Content:**
- Explain automatic PR creation flow
- Document manual PR creation endpoints
- Add examples of PR body format
- Update architecture diagrams

---

## Testing Strategy

### Unit Tests (per service)
- Test each service method independently
- Mock Git Provider API calls
- Test error handling scenarios
- Test edge cases (missing data, duplicates, etc.)

### Integration Tests
- Test end-to-end flow: task completion → PR creation → issue closure
- Test webhook handling for PR merge events
- Test manual API endpoints
- Use test database and mock Git Provider

### Manual Testing Checklist
- [ ] Create a test repository with an issue
- [ ] Trigger task execution
- [ ] Verify PR is created automatically
- [ ] Verify PR body contains "Closes #<issue_number>"
- [ ] Merge PR manually
- [ ] Verify issue is closed automatically
- [ ] Verify task status is "completed"
- [ ] Test manual PR creation endpoint
- [ ] Test manual issue closure endpoint

## Success Criteria

1. **Automatic PR Creation**
   - ✅ PR created when task completes with branch_name
   - ✅ PR title matches issue title
   - ✅ PR body contains issue reference
   - ✅ Task updated with pr_number and pr_url

2. **Issue Linking**
   - ✅ PR body uses "Closes #<issue_number>" format
   - ✅ Git provider automatically links PR to issue

3. **Automatic Issue Closure**
   - ✅ Issue closed when PR is merged
   - ✅ Task status updated to "completed"

4. **Error Handling**
   - ✅ Graceful handling of API failures
   - ✅ Retry on transient errors
   - ✅ Comprehensive logging

5. **Testing**
   - ✅ All unit tests passing
   - ✅ All integration tests passing
   - ✅ Manual testing completed

## Timeline

- **Task 1**: PRCreationService - 4 hours
- **Task 2**: TaskExecutorService integration - 2 hours
- **Task 3**: IssueClosureService - 3 hours
- **Task 4**: Webhook handler - 2 hours
- **Task 5**: API endpoints - 2 hours
- **Task 6**: Documentation - 1 hour

**Total Estimated Time:** 14 hours (2 days)

## Dependencies

- ✅ Git Provider API (`create_pull_request`, `update_issue`)
- ✅ Task execution engine
- ✅ Webhook infrastructure
- ✅ Database schema (tasks table with pr_number, pr_url fields)

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Git Provider API rate limits | High | Implement exponential backoff, cache results |
| Network failures during PR creation | Medium | Retry mechanism, manual fallback endpoint |
| Duplicate PR creation | Low | Check pr_number before creating |
| Issue already closed | Low | Handle gracefully, log info |
| Branch doesn't exist | Medium | Validate branch before PR creation |

## Future Enhancements

1. **PR Templates**
   - Support custom PR body templates
   - Include task execution summary in PR body

2. **PR Review Automation**
   - Auto-assign reviewers
   - Add labels based on task priority

3. **PR Status Tracking**
   - Track PR review status
   - Notify on review comments

4. **Multi-PR Support**
   - Support multiple PRs per task
   - Track PR dependencies

---

**Document Version:** 1.0  
**Last Updated:** 2026-01-21  
**Author:** AI Agent  
**Status:** Ready for Implementation
