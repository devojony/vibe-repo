## MODIFIED Requirements

### Requirement: Task execution SHALL use ACP protocol instead of CLI output parsing

The system SHALL execute tasks using the Agent Client Protocol (ACP) for structured communication with AI agents, replacing the previous CLI-based execution with output parsing.

#### Scenario: Task execution spawns ACP agent
- **WHEN** a task is executed
- **THEN** the system SHALL spawn an ACP-compatible agent subprocess
- **THEN** the system SHALL communicate via JSON-RPC over stdin/stdout
- **THEN** the system SHALL NOT use docker exec with CLI commands

#### Scenario: Task execution receives structured events
- **WHEN** agent executes task
- **THEN** the system SHALL receive structured events (plans, tool calls, messages)
- **THEN** the system SHALL NOT parse unstructured CLI output
- **THEN** the system SHALL store events in tasks.events JSONB field

#### Scenario: Task execution tracks real-time progress
- **WHEN** agent sends plan updates
- **THEN** the system SHALL update tasks.plans JSONB field
- **THEN** the system SHALL calculate progress percentage
- **THEN** the system SHALL expose progress via API

#### Scenario: Task execution handles permissions
- **WHEN** agent requests permission
- **THEN** the system SHALL evaluate against permission policy
- **THEN** the system SHALL respond with allow or deny
- **THEN** the system SHALL log permission decision

### Requirement: Task execution SHALL store agent plans and events

The system SHALL store structured agent plans and events in addition to the execution log, providing visibility into agent reasoning and actions.

#### Scenario: Store agent plans
- **WHEN** agent creates or updates execution plan
- **THEN** the system SHALL store plan in tasks.plans JSONB field
- **THEN** plan SHALL include step descriptions, status, and timestamps
- **THEN** plan history SHALL be preserved

#### Scenario: Store tool call events
- **WHEN** agent executes tools
- **THEN** the system SHALL append tool call events to tasks.events JSONB array
- **THEN** events SHALL include tool name, arguments, and results
- **THEN** events SHALL be limited to last 100 entries

#### Scenario: Store agent messages
- **WHEN** agent sends messages
- **THEN** the system SHALL append to tasks.last_log field (unchanged)
- **THEN** messages SHALL include timestamps
- **THEN** log size limit SHALL remain 10MB

### Requirement: Task execution SHALL support agent cancellation

The system SHALL support graceful cancellation of running tasks via ACP protocol.

#### Scenario: Cancel task via ACP
- **WHEN** user or system requests task cancellation
- **THEN** the system SHALL send cancel request to agent via ACP
- **THEN** agent SHALL stop execution gracefully
- **THEN** task status SHALL be updated to Cancelled

#### Scenario: Force kill on timeout
- **WHEN** agent does not respond to cancel request within grace period
- **THEN** the system SHALL force kill agent process
- **THEN** the system SHALL clean up resources
- **THEN** task SHALL be marked as Cancelled with timeout note

### Requirement: Task execution SHALL maintain backward compatibility for PR creation

The system SHALL continue to extract PR information and create pull requests, maintaining the core Issue-to-PR automation workflow.

#### Scenario: PR information extracted from git operations
- **WHEN** task completes successfully
- **THEN** the system SHALL extract branch name from agent's git operations
- **THEN** the system SHALL extract commit messages from git log
- **THEN** the system SHALL use information for PR creation (unchanged)

#### Scenario: PR created automatically
- **WHEN** PR information is available
- **THEN** the system SHALL create pull request via Git provider API (unchanged)
- **THEN** the system SHALL update task with pr_number and pr_url fields (unchanged)

## ADDED Requirements

### Requirement: Task execution SHALL use Bun runtime for agents

The system SHALL use Bun as the JavaScript runtime for spawning ACP-compatible agents, providing faster startup times.

#### Scenario: Spawn agent with Bun
- **WHEN** task execution starts
- **THEN** the system SHALL use "bun" command to spawn agent
- **THEN** the system SHALL NOT use "node" command
- **THEN** agent startup SHALL be 10x faster than Node.js

#### Scenario: Configure agent environment
- **WHEN** spawning agent subprocess
- **THEN** the system SHALL set API keys via environment variables
- **THEN** the system SHALL set working directory to workspace path
- **THEN** the system SHALL configure agent-specific options

### Requirement: Task execution SHALL support multiple agent types

The system SHALL support different ACP-compatible agents (OpenCode, Claude Code, etc.) based on repository configuration.

#### Scenario: Use repository-configured agent
- **WHEN** repository has agent_type configured
- **THEN** the system SHALL spawn the specified agent
- **THEN** the system SHALL use agent-specific command and arguments

#### Scenario: Fall back to default agent
- **WHEN** repository does not specify agent_type
- **THEN** the system SHALL use system default agent (OpenCode)
- **THEN** the system SHALL log agent selection decision

#### Scenario: Validate agent availability
- **WHEN** spawning agent
- **THEN** the system SHALL verify agent command is available
- **THEN** the system SHALL fail task with clear error if agent not found
