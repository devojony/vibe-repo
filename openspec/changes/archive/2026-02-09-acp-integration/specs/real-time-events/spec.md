## ADDED Requirements

### Requirement: System SHALL capture agent plan updates
The system SHALL capture and store agent planning information for progress tracking.

#### Scenario: Capture initial plan
- **WHEN** agent creates execution plan
- **THEN** system SHALL extract plan entries with step descriptions
- **THEN** system SHALL store plan in tasks.plans JSONB field
- **THEN** each plan entry SHALL include step, status, and timestamp

#### Scenario: Update plan progress
- **WHEN** agent updates plan status
- **THEN** system SHALL update corresponding plan entry
- **THEN** system SHALL preserve plan history
- **THEN** system SHALL track completion percentage

#### Scenario: Handle plan modifications
- **WHEN** agent modifies plan during execution
- **THEN** system SHALL append new plan entries
- **THEN** system SHALL mark modified entries
- **THEN** system SHALL maintain chronological order

### Requirement: System SHALL capture tool call events
The system SHALL record all tool executions for audit and debugging.

#### Scenario: Capture tool call details
- **WHEN** agent executes a tool
- **THEN** system SHALL record tool name
- **THEN** system SHALL record tool arguments
- **THEN** system SHALL record timestamp
- **THEN** system SHALL record execution result

#### Scenario: Capture file operations
- **WHEN** agent reads or writes files
- **THEN** system SHALL record file path
- **THEN** system SHALL record operation type (read/write)
- **THEN** system SHALL record file size for writes

#### Scenario: Capture shell commands
- **WHEN** agent executes shell command
- **THEN** system SHALL record command string
- **THEN** system SHALL record exit code
- **THEN** system SHALL record stdout/stderr output

#### Scenario: Limit event storage
- **WHEN** events exceed configured limit
- **THEN** system SHALL keep most recent N events
- **THEN** system SHALL discard oldest events
- **THEN** system SHALL log event compaction

### Requirement: System SHALL capture agent messages
The system SHALL store agent communication for conversation history.

#### Scenario: Capture agent responses
- **WHEN** agent sends message to user
- **THEN** system SHALL append message to tasks.last_log
- **THEN** system SHALL include timestamp
- **THEN** system SHALL preserve message formatting

#### Scenario: Capture agent reasoning
- **WHEN** agent uses think tool
- **THEN** system SHALL capture reasoning content
- **THEN** system SHALL store as special event type
- **THEN** system SHALL make available for debugging

#### Scenario: Truncate long logs
- **WHEN** log exceeds size limit
- **THEN** system SHALL truncate oldest content
- **THEN** system SHALL preserve recent messages
- **THEN** system SHALL add truncation marker

### Requirement: System SHALL provide event query interface
The system SHALL allow querying stored events for monitoring and debugging.

#### Scenario: Query events by task
- **WHEN** user requests task events
- **THEN** system SHALL return events from tasks.events field
- **THEN** events SHALL be ordered chronologically
- **THEN** events SHALL include all captured details

#### Scenario: Query events by type
- **WHEN** user filters by event type
- **THEN** system SHALL return only matching events
- **THEN** system SHALL support filtering by tool_call, message, plan, etc.

#### Scenario: Query events by time range
- **WHEN** user specifies time range
- **THEN** system SHALL return events within range
- **THEN** system SHALL use event timestamps for filtering

### Requirement: System SHALL track task progress
The system SHALL calculate and expose task progress based on events.

#### Scenario: Calculate progress from plan
- **WHEN** task has active plan
- **THEN** system SHALL count completed vs total steps
- **THEN** system SHALL calculate progress percentage
- **THEN** system SHALL expose via API

#### Scenario: Estimate time remaining
- **WHEN** task is in progress
- **THEN** system SHALL calculate average step duration
- **THEN** system SHALL estimate remaining time
- **THEN** system SHALL update estimate as task progresses

#### Scenario: Detect stalled tasks
- **WHEN** no events received for configured duration
- **THEN** system SHALL mark task as potentially stalled
- **THEN** system SHALL trigger health check
- **THEN** system SHALL alert if agent is unresponsive

### Requirement: System SHALL support event streaming (future)
The system SHALL provide foundation for real-time event streaming to clients.

#### Scenario: Store events for later retrieval
- **WHEN** events are captured
- **THEN** system SHALL store in database immediately
- **THEN** system SHALL make available via API
- **THEN** clients SHALL poll for updates

#### Scenario: Prepare for WebSocket streaming
- **WHEN** event is captured
- **THEN** system SHALL use format compatible with streaming
- **THEN** system SHALL include all necessary metadata
- **THEN** future WebSocket implementation SHALL reuse event structure
