## ADDED Requirements

### Requirement: Task status SHALL be represented as an enum type

The system SHALL use a type-safe `TaskStatus` enum to represent task states instead of arbitrary string values. The enum SHALL include the following states: Pending, Running, Completed, Failed, and Cancelled. The Assigned state SHALL be removed.

#### Scenario: Task created with valid status
- **WHEN** a new task is created
- **THEN** the task status SHALL be set to Pending

#### Scenario: Task status retrieved as enum
- **WHEN** a task is retrieved from the database
- **THEN** the task_status field SHALL be a TaskStatus enum value

#### Scenario: Invalid status string rejected
- **WHEN** an attempt is made to set task status to an invalid string value
- **THEN** the system SHALL return a validation error

### Requirement: State transitions SHALL be validated

The system SHALL validate all state transitions before allowing status changes. Only transitions defined in the simplified state machine SHALL be permitted.

#### Scenario: Valid transition from Pending to Running
- **WHEN** start_task() is called on a task with status Pending
- **THEN** the task status SHALL transition to Running

#### Scenario: Invalid transition from Completed to Failed rejected
- **WHEN** fail_task() is called on a task with status Completed
- **THEN** the system SHALL return a validation error indicating the transition is not allowed

#### Scenario: Invalid transition from Cancelled to Running rejected
- **WHEN** start_task() is called on a task with status Cancelled
- **THEN** the system SHALL return a validation error indicating the transition is not allowed

### Requirement: Pending state transitions

Tasks in Pending state SHALL only transition to Running or Cancelled states. The transition to Assigned state SHALL be removed.

#### Scenario: Pending to Running transition
- **WHEN** start_task() is called on a Pending task
- **THEN** the task status SHALL transition to Running

#### Scenario: Pending to Cancelled transition
- **WHEN** cancel_task() is called on a Pending task
- **THEN** the task status SHALL transition to Cancelled

#### Scenario: Direct execution without assignment
- **WHEN** a task is created with workspace configuration
- **THEN** the task SHALL be executable directly from Pending state without requiring assignment

### Requirement: Running state transitions

Tasks in Running state SHALL only transition to Completed, Failed, or Cancelled states.

#### Scenario: Running to Completed transition
- **WHEN** complete_task() is called on a Running task with PR information
- **THEN** the task status SHALL transition to Completed

#### Scenario: Running to Failed transition
- **WHEN** fail_task() is called on a Running task
- **THEN** the task status SHALL transition to Failed

#### Scenario: Running to Cancelled transition
- **WHEN** cancel_task() is called on a Running task
- **THEN** the task status SHALL transition to Cancelled

#### Scenario: Running to Pending transition rejected
- **WHEN** retry_task() is called on a Running task
- **THEN** the system SHALL return a validation error

### Requirement: Failed state SHALL be terminal

Tasks in Failed state SHALL be terminal and SHALL NOT transition to any other state. Retry functionality SHALL be removed.

#### Scenario: Failed state is terminal
- **WHEN** any state transition method is called on a Failed task
- **THEN** the system SHALL return a validation error indicating the task is in a terminal state

#### Scenario: No retry from Failed state
- **WHEN** retry_task() is called on a Failed task
- **THEN** the system SHALL return a validation error indicating retry is not supported

#### Scenario: Manual retry requires new task
- **WHEN** a user wants to retry a failed task
- **THEN** the user SHALL create a new task with the same parameters

### Requirement: Terminal states SHALL not transition

Tasks in Completed, Failed, or Cancelled states SHALL be terminal and SHALL NOT transition to any other state.

#### Scenario: Completed state is terminal
- **WHEN** any state transition method is called on a Completed task
- **THEN** the system SHALL return a validation error indicating the task is in a terminal state

#### Scenario: Cancelled state is terminal
- **WHEN** any state transition method is called on a Cancelled task
- **THEN** the system SHALL return a validation error indicating the task is in a terminal state

### Requirement: State transition validation SHALL occur before database update

The system SHALL validate state transitions before attempting to update the database to prevent invalid states from being persisted.

#### Scenario: Validation error prevents database update
- **WHEN** an invalid state transition is attempted
- **THEN** the system SHALL return a validation error without modifying the database

#### Scenario: Valid transition updates database
- **WHEN** a valid state transition is attempted
- **THEN** the system SHALL update the task status in the database

### Requirement: State machine SHALL provide introspection

The TaskStatus enum SHALL provide methods to query allowed transitions and validate transition requests.

#### Scenario: Query allowed transitions from current state
- **WHEN** allowed_transitions() is called on a TaskStatus value
- **THEN** the system SHALL return a list of valid target states

#### Scenario: Check if specific transition is allowed
- **WHEN** can_transition_to() is called with a target state
- **THEN** the system SHALL return true if the transition is valid, false otherwise

#### Scenario: Check if state is terminal
- **WHEN** is_terminal() is called on a TaskStatus value
- **THEN** the system SHALL return true for Completed, Failed, and Cancelled, false for Pending and Running

### Requirement: Database migration SHALL preserve existing data

The migration SHALL convert existing task statuses to the simplified state machine, mapping Assigned tasks to Pending.

#### Scenario: Existing pending tasks preserved
- **WHEN** the migration runs on a database with tasks having status "pending"
- **THEN** those tasks SHALL have status TaskStatus::Pending after migration

#### Scenario: Existing assigned tasks converted to pending
- **WHEN** the migration runs on a database with tasks having status "assigned"
- **THEN** those tasks SHALL have status TaskStatus::Pending after migration

#### Scenario: Existing running tasks preserved
- **WHEN** the migration runs on a database with tasks having status "running"
- **THEN** those tasks SHALL have status TaskStatus::Running after migration

#### Scenario: Existing completed tasks preserved
- **WHEN** the migration runs on a database with tasks having status "completed"
- **THEN** those tasks SHALL have status TaskStatus::Completed after migration

#### Scenario: Existing failed tasks preserved
- **WHEN** the migration runs on a database with tasks having status "failed"
- **THEN** those tasks SHALL have status TaskStatus::Failed after migration

#### Scenario: Existing cancelled tasks preserved
- **WHEN** the migration runs on a database with tasks having status "cancelled"
- **THEN** those tasks SHALL have status TaskStatus::Cancelled after migration

### Requirement: API responses SHALL use enum serialization

All API endpoints that return task information SHALL serialize the TaskStatus enum as a string value matching the enum variant name in lowercase.

#### Scenario: Task status serialized in API response
- **WHEN** a task is returned in an API response
- **THEN** the task_status field SHALL be a string matching the enum variant (e.g., "pending", "running", "completed")

#### Scenario: API accepts enum values in requests
- **WHEN** an API request includes a task_status filter
- **THEN** the system SHALL accept enum variant names as valid values

#### Scenario: Assigned status not accepted
- **WHEN** an API request includes task_status="assigned"
- **THEN** the system SHALL return a validation error indicating "assigned" is not a valid status

### Requirement: Error messages SHALL indicate invalid transitions

When a state transition is rejected, the error message SHALL clearly indicate the current state, attempted target state, and why the transition is not allowed.

#### Scenario: Descriptive error for invalid transition
- **WHEN** an invalid state transition is attempted
- **THEN** the error message SHALL include the current state, target state, and list of allowed transitions

#### Scenario: Error includes transition rules
- **WHEN** a validation error occurs
- **THEN** the error SHALL indicate which state transitions are valid from the current state
