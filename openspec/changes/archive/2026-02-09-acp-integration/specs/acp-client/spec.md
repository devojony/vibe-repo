## ADDED Requirements

### Requirement: ACP client SHALL communicate via JSON-RPC over stdin/stdout
The ACP client SHALL implement JSON-RPC 2.0 protocol for bidirectional communication with agent processes using newline-delimited JSON (NDJSON) over stdin/stdout.

#### Scenario: Send JSON-RPC request
- **WHEN** client sends a request to the agent
- **THEN** request SHALL be formatted as valid JSON-RPC 2.0 with id, method, and params fields
- **THEN** request SHALL be terminated with a newline character

#### Scenario: Receive JSON-RPC response
- **WHEN** agent sends a response
- **THEN** client SHALL parse newline-delimited JSON from stdout
- **THEN** client SHALL match response id with pending request

#### Scenario: Handle JSON-RPC notification
- **WHEN** agent sends a notification (no id field)
- **THEN** client SHALL process notification without expecting a response

### Requirement: ACP client SHALL initialize agent sessions
The ACP client SHALL perform the ACP initialization handshake to establish capabilities and create sessions.

#### Scenario: Initialize agent connection
- **WHEN** client spawns agent subprocess
- **THEN** client SHALL send initialize request with client info and capabilities
- **THEN** client SHALL receive agent capabilities in response

#### Scenario: Create new session
- **WHEN** client needs to start a conversation
- **THEN** client SHALL send newSession request with working directory
- **THEN** client SHALL receive session_id for subsequent requests

#### Scenario: Reuse existing session
- **WHEN** client has an active session_id
- **THEN** client SHALL use the same session_id for multiple prompts
- **THEN** agent SHALL maintain conversation context within the session

### Requirement: ACP client SHALL send prompts and handle streaming responses
The ACP client SHALL send user prompts to the agent and process streaming responses including plans, tool calls, and messages.

#### Scenario: Send prompt to agent
- **WHEN** client sends a prompt request
- **THEN** request SHALL include session_id and content fields
- **THEN** client SHALL begin listening for streaming responses

#### Scenario: Receive plan updates
- **WHEN** agent sends sessionUpdate with plan data
- **THEN** client SHALL extract plan entries with step descriptions and status
- **THEN** client SHALL store plan updates for progress tracking

#### Scenario: Receive tool call events
- **WHEN** agent sends sessionUpdate with tool_call data
- **THEN** client SHALL extract tool name, arguments, and call ID
- **THEN** client SHALL log tool execution for audit trail

#### Scenario: Receive agent messages
- **WHEN** agent sends sessionUpdate with message data
- **THEN** client SHALL extract message content
- **THEN** client SHALL append message to conversation history

#### Scenario: Detect completion
- **WHEN** agent sends sessionUpdate with completed status
- **THEN** client SHALL extract stop_reason (end_turn, max_tokens, cancelled, etc.)
- **THEN** client SHALL mark task execution as complete

### Requirement: ACP client SHALL handle permission requests
The ACP client SHALL process permission requests from the agent and respond based on configured policy.

#### Scenario: Receive permission request
- **WHEN** agent sends requestPermission notification
- **THEN** client SHALL extract tool_kind (read, write, execute, delete)
- **THEN** client SHALL evaluate against permission policy

#### Scenario: Auto-approve safe operations
- **WHEN** permission request is for read or search operations
- **THEN** client SHALL automatically respond with allow

#### Scenario: Policy-based approval
- **WHEN** permission request is for write or execute operations
- **THEN** client SHALL check if operation is within workspace directory
- **THEN** client SHALL allow if within workspace, deny otherwise

#### Scenario: Deny dangerous operations
- **WHEN** permission request is for delete operations
- **THEN** client SHALL automatically respond with deny
- **THEN** client SHALL log denied operation for security audit

### Requirement: ACP client SHALL manage subprocess lifecycle
The ACP client SHALL spawn, monitor, and terminate agent subprocesses reliably.

#### Scenario: Spawn agent subprocess
- **WHEN** client needs to start an agent
- **THEN** client SHALL spawn subprocess with configured command and arguments
- **THEN** client SHALL set up stdin/stdout pipes for communication
- **THEN** client SHALL set environment variables (API keys, config)

#### Scenario: Monitor subprocess health
- **WHEN** agent subprocess is running
- **THEN** client SHALL monitor process status
- **THEN** client SHALL detect if process exits unexpectedly

#### Scenario: Handle subprocess crash
- **WHEN** agent subprocess crashes
- **THEN** client SHALL capture exit code and stderr output
- **THEN** client SHALL mark task as failed with error details
- **THEN** client SHALL clean up resources (close pipes, kill zombie processes)

#### Scenario: Terminate subprocess gracefully
- **WHEN** task execution completes or is cancelled
- **THEN** client SHALL send cancel request to agent
- **THEN** client SHALL wait for graceful shutdown with timeout
- **THEN** client SHALL force kill if timeout exceeded

### Requirement: ACP client SHALL handle errors gracefully
The ACP client SHALL detect and handle various error conditions without crashing.

#### Scenario: Handle JSON parse errors
- **WHEN** agent sends malformed JSON
- **THEN** client SHALL log parse error with raw input
- **THEN** client SHALL continue processing subsequent messages

#### Scenario: Handle protocol violations
- **WHEN** agent sends invalid JSON-RPC (missing required fields)
- **THEN** client SHALL log protocol violation
- **THEN** client SHALL send error response if request has id

#### Scenario: Handle timeout
- **WHEN** agent does not respond within configured timeout
- **THEN** client SHALL cancel the request
- **THEN** client SHALL mark task as failed with timeout error

#### Scenario: Handle backpressure
- **WHEN** agent sends messages faster than client can process
- **THEN** client SHALL buffer messages up to configured limit
- **THEN** client SHALL apply backpressure if buffer full
