## ADDED Requirements

### Requirement: Agent Manager SHALL spawn ACP-compatible agents
The Agent Manager SHALL create and configure agent subprocesses based on repository and task configuration.

#### Scenario: Spawn OpenCode agent with Bun
- **WHEN** task requires OpenCode agent
- **THEN** Agent Manager SHALL spawn subprocess with command "bun opencode acp"
- **THEN** Agent Manager SHALL set working directory to workspace path
- **THEN** Agent Manager SHALL pass API keys via environment variables

#### Scenario: Spawn Claude Code agent with adapter
- **WHEN** task requires Claude Code agent
- **THEN** Agent Manager SHALL spawn subprocess with command "bun claude-code-acp"
- **THEN** Agent Manager SHALL set ANTHROPIC_API_KEY environment variable
- **THEN** Agent Manager SHALL configure adapter-specific options

#### Scenario: Configure agent from repository settings
- **WHEN** repository has agent_type configured
- **THEN** Agent Manager SHALL use repository's agent preference
- **THEN** Agent Manager SHALL fall back to system default if not configured

### Requirement: Agent Manager SHALL manage agent lifecycle
The Agent Manager SHALL track agent processes and handle their lifecycle events.

#### Scenario: Track active agents
- **WHEN** agent is spawned
- **THEN** Agent Manager SHALL store agent process handle with task_id
- **THEN** Agent Manager SHALL track agent status (starting, running, stopping, stopped)

#### Scenario: Reuse agent for multiple prompts
- **WHEN** task requires multiple interactions
- **THEN** Agent Manager SHALL reuse same agent process
- **THEN** Agent Manager SHALL maintain session state across prompts

#### Scenario: Clean up completed agents
- **WHEN** task execution completes
- **THEN** Agent Manager SHALL terminate agent process gracefully
- **THEN** Agent Manager SHALL remove agent from active tracking
- **THEN** Agent Manager SHALL release system resources

#### Scenario: Handle agent timeout
- **WHEN** agent exceeds configured timeout
- **THEN** Agent Manager SHALL send cancel signal to agent
- **THEN** Agent Manager SHALL force kill after grace period
- **THEN** Agent Manager SHALL mark task as failed with timeout error

### Requirement: Agent Manager SHALL implement permission policy
The Agent Manager SHALL evaluate permission requests against configured policy and respond appropriately.

#### Scenario: Allow read operations
- **WHEN** agent requests read permission
- **THEN** Agent Manager SHALL automatically allow
- **THEN** Agent Manager SHALL log permission grant

#### Scenario: Allow workspace writes
- **WHEN** agent requests write permission for file in workspace
- **THEN** Agent Manager SHALL verify path is within workspace directory
- **THEN** Agent Manager SHALL allow if path is safe
- **THEN** Agent Manager SHALL deny if path is outside workspace

#### Scenario: Allow safe shell commands
- **WHEN** agent requests execute permission
- **THEN** Agent Manager SHALL check command against allowlist (git, cargo, npm, etc.)
- **THEN** Agent Manager SHALL allow if command is safe
- **THEN** Agent Manager SHALL deny if command is dangerous (rm -rf, dd, etc.)

#### Scenario: Deny delete operations
- **WHEN** agent requests delete permission
- **THEN** Agent Manager SHALL automatically deny
- **THEN** Agent Manager SHALL log denied operation with reason

#### Scenario: Log all permission decisions
- **WHEN** any permission request is processed
- **THEN** Agent Manager SHALL log request details (tool, path, decision)
- **THEN** Agent Manager SHALL store log in database for audit

### Requirement: Agent Manager SHALL stream events to database
The Agent Manager SHALL capture agent events and store them for progress tracking and debugging.

#### Scenario: Store plan updates
- **WHEN** agent sends plan update
- **THEN** Agent Manager SHALL extract plan entries
- **THEN** Agent Manager SHALL update tasks.plans JSONB field
- **THEN** Agent Manager SHALL preserve plan history

#### Scenario: Store tool call events
- **WHEN** agent executes a tool
- **THEN** Agent Manager SHALL capture tool name, arguments, and timestamp
- **THEN** Agent Manager SHALL append to tasks.events JSONB array
- **THEN** Agent Manager SHALL limit events to last 100 entries

#### Scenario: Store agent messages
- **WHEN** agent sends a message
- **THEN** Agent Manager SHALL capture message content and timestamp
- **THEN** Agent Manager SHALL append to tasks.last_log field
- **THEN** Agent Manager SHALL truncate log if exceeds size limit

#### Scenario: Store completion status
- **WHEN** agent completes execution
- **THEN** Agent Manager SHALL capture stop_reason
- **THEN** Agent Manager SHALL update task status (completed, failed, cancelled)
- **THEN** Agent Manager SHALL store final event with completion details

### Requirement: Agent Manager SHALL handle concurrent tasks
The Agent Manager SHALL support multiple concurrent task executions with resource limits.

#### Scenario: Enforce concurrency limit
- **WHEN** number of active agents reaches configured limit
- **THEN** Agent Manager SHALL queue new tasks
- **THEN** Agent Manager SHALL start queued tasks when slots available

#### Scenario: Isolate agent processes
- **WHEN** multiple agents are running
- **THEN** each agent SHALL run in separate process
- **THEN** agents SHALL NOT share state or interfere with each other

#### Scenario: Monitor resource usage
- **WHEN** agents are running
- **THEN** Agent Manager SHALL track CPU and memory usage per agent
- **THEN** Agent Manager SHALL log resource metrics
- **THEN** Agent Manager SHALL kill agent if exceeds resource limits

### Requirement: Agent Manager SHALL provide health monitoring
The Agent Manager SHALL monitor agent health and recover from failures.

#### Scenario: Detect agent crash
- **WHEN** agent process exits unexpectedly
- **THEN** Agent Manager SHALL capture exit code and stderr
- **THEN** Agent Manager SHALL mark task as failed
- **THEN** Agent Manager SHALL log crash details for debugging

#### Scenario: Detect agent hang
- **WHEN** agent stops responding to heartbeat
- **THEN** Agent Manager SHALL detect unresponsive state
- **THEN** Agent Manager SHALL attempt graceful shutdown
- **THEN** Agent Manager SHALL force kill if unresponsive

#### Scenario: Retry on transient failures
- **WHEN** agent fails with retryable error (network timeout, rate limit)
- **THEN** Agent Manager SHALL retry with exponential backoff
- **THEN** Agent Manager SHALL limit retry attempts to configured maximum
- **THEN** Agent Manager SHALL mark task as failed if max retries exceeded
