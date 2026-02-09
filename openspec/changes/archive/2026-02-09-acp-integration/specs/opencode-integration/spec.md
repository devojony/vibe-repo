## ADDED Requirements

### Requirement: OpenCode SHALL be installed via Bun
The system SHALL install OpenCode using Bun package manager for fast startup and execution.

#### Scenario: Install OpenCode in Docker image
- **WHEN** Docker image is built
- **THEN** Bun SHALL be installed from official source
- **THEN** OpenCode SHALL be installed globally via "bun install -g opencode-ai"
- **THEN** OpenCode version SHALL be verified with "opencode --version"

#### Scenario: Verify OpenCode ACP support
- **WHEN** OpenCode is installed
- **THEN** system SHALL verify "opencode acp" command is available
- **THEN** system SHALL test ACP initialization handshake

### Requirement: OpenCode SHALL be configured with API keys
The system SHALL provide LLM provider API keys to OpenCode via environment variables.

#### Scenario: Configure Anthropic API key
- **WHEN** repository uses Anthropic models
- **THEN** ANTHROPIC_API_KEY environment variable SHALL be set
- **THEN** OpenCode SHALL use Claude models for task execution

#### Scenario: Configure OpenAI API key
- **WHEN** repository uses OpenAI models
- **THEN** OPENAI_API_KEY environment variable SHALL be set
- **THEN** OpenCode SHALL use GPT models for task execution

#### Scenario: Configure multiple providers
- **WHEN** repository supports multiple LLM providers
- **THEN** all relevant API keys SHALL be set
- **THEN** OpenCode SHALL select provider based on model configuration

#### Scenario: Handle missing API keys
- **WHEN** required API key is not configured
- **THEN** Agent Manager SHALL fail task with clear error message
- **THEN** error message SHALL indicate which API key is missing

### Requirement: OpenCode SHALL execute in workspace context
The system SHALL run OpenCode with proper working directory and file access.

#### Scenario: Set working directory
- **WHEN** OpenCode agent is spawned
- **THEN** working directory SHALL be set to workspace path
- **THEN** OpenCode SHALL have access to repository files

#### Scenario: Provide AGENTS.md context
- **WHEN** repository has AGENTS.md file
- **THEN** OpenCode SHALL read and use project guidelines
- **THEN** OpenCode SHALL follow coding patterns from AGENTS.md

#### Scenario: Access git repository
- **WHEN** OpenCode needs to interact with git
- **THEN** OpenCode SHALL have access to .git directory
- **THEN** OpenCode SHALL be able to run git commands

### Requirement: OpenCode SHALL support custom configuration
The system SHALL allow repository-specific OpenCode configuration.

#### Scenario: Configure default model
- **WHEN** repository specifies preferred model
- **THEN** OpenCode SHALL use configured model
- **THEN** OpenCode SHALL fall back to system default if not specified

#### Scenario: Configure timeout
- **WHEN** repository specifies custom timeout
- **THEN** OpenCode SHALL respect timeout setting
- **THEN** OpenCode SHALL be terminated if timeout exceeded

#### Scenario: Configure tools
- **WHEN** repository specifies allowed tools
- **THEN** OpenCode SHALL only use permitted tools
- **THEN** OpenCode SHALL deny access to restricted tools

### Requirement: OpenCode SHALL integrate with MCP servers
The system SHALL support OpenCode's MCP server integration for extended capabilities.

#### Scenario: Use built-in MCP servers
- **WHEN** OpenCode is configured with MCP servers
- **THEN** OpenCode SHALL connect to configured MCP servers
- **THEN** OpenCode SHALL have access to MCP tools

#### Scenario: Use Docker MCP Gateway
- **WHEN** MCP servers are available via Docker Gateway
- **THEN** OpenCode SHALL connect to containerized MCP servers
- **THEN** OpenCode SHALL use tools from MCP catalog

### Requirement: OpenCode SHALL provide structured output
The system SHALL capture and parse OpenCode's structured output for PR creation.

#### Scenario: Extract PR information
- **WHEN** OpenCode completes task successfully
- **THEN** system SHALL extract branch name from git operations
- **THEN** system SHALL extract commit messages
- **THEN** system SHALL use information for PR creation

#### Scenario: Extract error information
- **WHEN** OpenCode fails to complete task
- **THEN** system SHALL extract error messages
- **THEN** system SHALL extract failure reason
- **THEN** system SHALL store error details in task record

### Requirement: OpenCode SHALL support cancellation
The system SHALL be able to cancel running OpenCode tasks.

#### Scenario: Cancel via ACP protocol
- **WHEN** user or system requests task cancellation
- **THEN** Agent Manager SHALL send cancel request via ACP
- **THEN** OpenCode SHALL stop execution gracefully
- **THEN** OpenCode SHALL return partial results if available

#### Scenario: Force termination
- **WHEN** OpenCode does not respond to cancel request
- **THEN** Agent Manager SHALL force kill OpenCode process
- **THEN** system SHALL clean up partial work
- **THEN** task SHALL be marked as cancelled
