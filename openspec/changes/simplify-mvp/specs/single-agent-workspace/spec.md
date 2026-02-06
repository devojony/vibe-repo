## ADDED Requirements

### Requirement: Each workspace SHALL have exactly one agent

The system SHALL enforce a one-to-one relationship between workspaces and agents. Each workspace SHALL have exactly one agent configuration.

#### Scenario: Workspace created with agent configuration
- **WHEN** a workspace is created
- **THEN** the system SHALL create exactly one agent for that workspace

#### Scenario: Second agent creation rejected
- **WHEN** attempting to create a second agent for a workspace
- **THEN** the system SHALL return a validation error indicating that the workspace already has an agent

#### Scenario: Agent deletion removes workspace agent
- **WHEN** an agent is deleted
- **THEN** the workspace SHALL have no agent until a new one is created

### Requirement: Agent configuration SHALL be stored in repository settings

Agent configuration (command, timeout, environment variables) SHALL be stored as part of the repository configuration, not in a separate agents table.

#### Scenario: Repository stores agent command
- **WHEN** a repository is configured
- **THEN** the repository record SHALL include an agent_command field

#### Scenario: Repository stores agent timeout
- **WHEN** a repository is configured
- **THEN** the repository record SHALL include an agent_timeout field

#### Scenario: Repository stores agent environment variables
- **WHEN** a repository is configured
- **THEN** the repository record SHALL include an agent_env_vars JSON field

### Requirement: Workspace SHALL use repository agent configuration

When a workspace is created for a repository, it SHALL automatically use the agent configuration from the repository settings.

#### Scenario: Workspace inherits agent command
- **WHEN** a workspace is created for a repository
- **THEN** the workspace SHALL use the repository's agent_command

#### Scenario: Workspace inherits agent timeout
- **WHEN** a workspace is created for a repository
- **THEN** the workspace SHALL use the repository's agent_timeout

#### Scenario: Workspace inherits agent environment variables
- **WHEN** a workspace is created for a repository
- **THEN** the workspace SHALL use the repository's agent_env_vars

### Requirement: Agent configuration SHALL be immutable per workspace

Once a workspace is created, its agent configuration SHALL NOT be changed. To use a different agent configuration, a new workspace SHALL be created.

#### Scenario: Agent configuration cannot be updated
- **WHEN** attempting to update agent configuration for an existing workspace
- **THEN** the system SHALL return a validation error

#### Scenario: New workspace required for different agent
- **WHEN** a user wants to use a different agent configuration
- **THEN** the user SHALL create a new workspace with the new configuration

### Requirement: Task creation SHALL not require agent selection

When creating a task, the system SHALL automatically use the workspace's agent. No agent selection or assignment SHALL be required.

#### Scenario: Task uses workspace agent automatically
- **WHEN** a task is created for a workspace
- **THEN** the system SHALL automatically assign the workspace's agent to the task

#### Scenario: No agent_id parameter in task creation
- **WHEN** creating a task via POST /tasks
- **THEN** the request SHALL NOT include an agent_id parameter

#### Scenario: Task execution uses workspace agent
- **WHEN** a task is executed
- **THEN** the system SHALL use the workspace's agent configuration

### Requirement: Agent enabled/disabled state SHALL be removed

The system SHALL NOT track whether an agent is enabled or disabled. Agent availability SHALL be determined by workspace existence.

#### Scenario: No enabled field in agent configuration
- **WHEN** querying agent configuration
- **THEN** the configuration SHALL NOT include an enabled field

#### Scenario: Workspace existence indicates agent availability
- **WHEN** a workspace exists
- **THEN** its agent SHALL be considered available for task execution

#### Scenario: Deleted workspace means no agent
- **WHEN** a workspace is deleted
- **THEN** tasks for that workspace SHALL NOT be executable until a new workspace is created

### Requirement: Database schema SHALL enforce one-agent-per-workspace

The database schema SHALL include a UNIQUE constraint on workspace_id in the agents table (if agents table is retained) or SHALL store agent configuration directly in the repositories table.

#### Scenario: Unique constraint prevents duplicate agents
- **WHEN** attempting to insert a second agent with the same workspace_id
- **THEN** the database SHALL reject the insert with a unique constraint violation

#### Scenario: Agent configuration in repository table
- **WHEN** using the simplified schema
- **THEN** agent configuration SHALL be stored in repositories table columns (agent_command, agent_timeout, agent_env_vars)

### Requirement: Agent configuration SHALL support common AI tools

The agent configuration SHALL support common AI coding tools including OpenCode, Aider, and custom commands.

#### Scenario: OpenCode agent configuration
- **WHEN** configuring an agent with OpenCode
- **THEN** the agent_command SHALL be "opencode --model <model-name>"

#### Scenario: Aider agent configuration
- **WHEN** configuring an agent with Aider
- **THEN** the agent_command SHALL be "aider --model <model-name> --yes"

#### Scenario: Custom agent configuration
- **WHEN** configuring a custom agent
- **THEN** the agent_command SHALL accept any valid shell command

#### Scenario: Agent environment variables for API keys
- **WHEN** an agent requires API keys
- **THEN** the agent_env_vars SHALL include the necessary environment variables (e.g., OPENAI_API_KEY)
