## ADDED Requirements

### Requirement: Task execution SHALL store only the most recent log

The system SHALL store only the most recent execution log for each task in the tasks table, rather than maintaining a separate execution history table. The log SHALL be stored in a `last_log` TEXT field with a maximum size of 10MB.

#### Scenario: Task execution stores log in task record
- **WHEN** a task is executed
- **THEN** the execution log SHALL be stored in the task's last_log field

#### Scenario: New execution overwrites previous log
- **WHEN** a task is executed multiple times
- **THEN** each new execution SHALL overwrite the previous log in the last_log field

#### Scenario: Log size limit enforced
- **WHEN** a task execution generates more than 10MB of log data
- **THEN** the system SHALL truncate the log to 10MB and append a truncation notice

### Requirement: Task execution SHALL NOT track detailed execution history

The system SHALL NOT maintain a separate task_executions table or detailed execution history. Only the current task status and most recent log SHALL be preserved.

#### Scenario: No execution history table
- **WHEN** querying task execution data
- **THEN** the system SHALL only return data from the tasks table, not from a separate executions table

#### Scenario: Historical execution data not available
- **WHEN** a user requests execution history for a task
- **THEN** the system SHALL only return the most recent log, not historical logs

### Requirement: Task execution SHALL NOT perform failure analysis

The system SHALL NOT categorize failures into specific types or provide automated failure analysis. Task failures SHALL be recorded with a simple error message string.

#### Scenario: Task failure stores error message
- **WHEN** a task execution fails
- **THEN** the system SHALL store the error message in the task's last_log field

#### Scenario: No failure categorization
- **WHEN** a task fails
- **THEN** the system SHALL NOT categorize the failure type (e.g., ContainerError, GitError, etc.)

#### Scenario: No automated recommendations
- **WHEN** a task fails
- **THEN** the system SHALL NOT generate automated recommendations for fixing the failure

### Requirement: Task execution SHALL use simplified retry logic

The system SHALL NOT maintain retry counters or complex retry strategies. Failed tasks MAY be manually retried by creating a new task.

#### Scenario: No automatic retry
- **WHEN** a task fails
- **THEN** the system SHALL NOT automatically retry the task

#### Scenario: Manual retry creates new task
- **WHEN** a user wants to retry a failed task
- **THEN** the user SHALL create a new task with the same parameters

#### Scenario: No retry count tracking
- **WHEN** a task is created or executed
- **THEN** the system SHALL NOT track how many times the task has been retried

### Requirement: Task logs SHALL be queryable via REST API

The system SHALL provide a REST API endpoint to retrieve task logs, replacing the WebSocket streaming interface.

#### Scenario: Get task log via REST
- **WHEN** a GET request is made to /tasks/:id/logs
- **THEN** the system SHALL return the task's last_log content

#### Scenario: Log polling for real-time monitoring
- **WHEN** a client wants to monitor task execution in real-time
- **THEN** the client SHALL poll the /tasks/:id/logs endpoint at regular intervals (e.g., every 2-5 seconds)

#### Scenario: Empty log for new tasks
- **WHEN** a task has not been executed yet
- **THEN** the /tasks/:id/logs endpoint SHALL return an empty string or null

### Requirement: Task execution SHALL maintain basic status tracking

The system SHALL track task status (Pending, Running, Completed, Failed, Cancelled) and basic metadata (created_at, started_at, completed_at).

#### Scenario: Task status updated during execution
- **WHEN** a task execution progresses
- **THEN** the system SHALL update the task status accordingly

#### Scenario: Execution timestamps recorded
- **WHEN** a task starts execution
- **THEN** the system SHALL record the started_at timestamp
- **WHEN** a task completes or fails
- **THEN** the system SHALL record the completed_at timestamp

#### Scenario: Task metadata queryable
- **WHEN** a GET request is made to /tasks/:id/status
- **THEN** the system SHALL return the task status and timestamps

### Requirement: Task execution SHALL preserve PR creation functionality

The system SHALL continue to extract PR information from agent output and create pull requests, maintaining the core Issue-to-PR automation workflow.

#### Scenario: PR information extracted from output
- **WHEN** a task completes successfully and the agent output contains PR information
- **THEN** the system SHALL extract the PR number and URL

#### Scenario: PR created automatically
- **WHEN** PR information is extracted from agent output
- **THEN** the system SHALL create a pull request via the Git provider API

#### Scenario: Task updated with PR information
- **WHEN** a PR is created successfully
- **THEN** the system SHALL update the task with pr_number and pr_url fields
