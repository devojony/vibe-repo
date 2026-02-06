## ADDED Requirements

### Requirement: API SHALL expose exactly 8 core endpoints

The system SHALL provide exactly 8 API endpoints covering the essential Issue-to-PR automation workflow. All other endpoints SHALL be removed.

#### Scenario: Repository management endpoint available
- **WHEN** a POST request is made to /repositories
- **THEN** the system SHALL accept repository configuration and create a repository record

#### Scenario: Webhook endpoint available
- **WHEN** a POST request is made to /webhooks/github
- **THEN** the system SHALL process the webhook payload and create tasks as needed

#### Scenario: Task list endpoint available
- **WHEN** a GET request is made to /tasks
- **THEN** the system SHALL return a list of tasks with optional filtering

#### Scenario: Task creation endpoint available
- **WHEN** a POST request is made to /tasks
- **THEN** the system SHALL create a new task

#### Scenario: Task execution endpoint available
- **WHEN** a POST request is made to /tasks/:id/execute
- **THEN** the system SHALL start executing the specified task

#### Scenario: Task logs endpoint available
- **WHEN** a GET request is made to /tasks/:id/logs
- **THEN** the system SHALL return the task's execution log

#### Scenario: Task status endpoint available
- **WHEN** a GET request is made to /tasks/:id/status
- **THEN** the system SHALL return the task's current status and metadata

#### Scenario: Task deletion endpoint available
- **WHEN** a DELETE request is made to /tasks/:id
- **THEN** the system SHALL delete the specified task

### Requirement: Provider configuration SHALL use environment variables

The system SHALL NOT provide API endpoints for managing Git providers. Provider configuration SHALL be done via environment variables or configuration files.

#### Scenario: No provider management endpoints
- **WHEN** attempting to access /api/providers endpoints
- **THEN** the system SHALL return 404 Not Found

#### Scenario: Provider configured via environment
- **WHEN** the system starts
- **THEN** it SHALL read provider configuration from environment variables (e.g., GITHUB_TOKEN, GITHUB_BASE_URL)

### Requirement: Workspace configuration SHALL be automatic

The system SHALL NOT provide API endpoints for managing workspaces. Workspaces SHALL be automatically created when needed based on repository configuration.

#### Scenario: No workspace management endpoints
- **WHEN** attempting to access /api/workspaces endpoints
- **THEN** the system SHALL return 404 Not Found

#### Scenario: Workspace created automatically
- **WHEN** a task is created for a repository
- **THEN** the system SHALL automatically create or reuse a workspace for that repository

### Requirement: Agent configuration SHALL use environment variables

The system SHALL NOT provide API endpoints for managing agents. Agent configuration SHALL be done via environment variables or configuration files.

#### Scenario: No agent management endpoints
- **WHEN** attempting to access /api/agents endpoints
- **THEN** the system SHALL return 404 Not Found

#### Scenario: Agent configured via environment
- **WHEN** a workspace is created
- **THEN** the system SHALL configure the agent using environment variables (e.g., AGENT_COMMAND, AGENT_TIMEOUT)

### Requirement: Webhook configuration SHALL be automatic

The system SHALL NOT provide API endpoints for configuring webhooks. Webhook URLs and secrets SHALL be configured via environment variables.

#### Scenario: No webhook configuration endpoints
- **WHEN** attempting to access /api/webhooks/config endpoints
- **THEN** the system SHALL return 404 Not Found

#### Scenario: Webhook secret configured via environment
- **WHEN** a webhook is received
- **THEN** the system SHALL validate the signature using the WEBHOOK_SECRET environment variable

### Requirement: Statistics and monitoring endpoints SHALL be removed

The system SHALL NOT provide API endpoints for statistics, metrics, or monitoring. Users SHALL rely on external monitoring tools or database queries.

#### Scenario: No statistics endpoints
- **WHEN** attempting to access /api/stats endpoints
- **THEN** the system SHALL return 404 Not Found

#### Scenario: No health check endpoints
- **WHEN** attempting to access /api/health endpoints
- **THEN** the system SHALL return 404 Not Found

### Requirement: API documentation SHALL reflect minimal surface

The system SHALL provide OpenAPI documentation that includes only the 8 core endpoints.

#### Scenario: OpenAPI spec includes only core endpoints
- **WHEN** accessing /swagger-ui or /openapi.json
- **THEN** the documentation SHALL list exactly 8 endpoints

#### Scenario: Removed endpoints not documented
- **WHEN** viewing API documentation
- **THEN** removed endpoints (providers, workspaces, agents, webhooks config, stats) SHALL NOT appear

### Requirement: Task filtering SHALL support basic queries

The GET /tasks endpoint SHALL support basic filtering by status, repository, and date range.

#### Scenario: Filter tasks by status
- **WHEN** a GET request is made to /tasks?status=pending
- **THEN** the system SHALL return only tasks with Pending status

#### Scenario: Filter tasks by repository
- **WHEN** a GET request is made to /tasks?repository_id=123
- **THEN** the system SHALL return only tasks for repository 123

#### Scenario: Filter tasks by date range
- **WHEN** a GET request is made to /tasks?created_after=2024-01-01
- **THEN** the system SHALL return only tasks created after the specified date

### Requirement: Error responses SHALL be consistent

All API endpoints SHALL return consistent error responses with appropriate HTTP status codes and error messages.

#### Scenario: Not found error
- **WHEN** a resource is not found
- **THEN** the system SHALL return 404 with a JSON error message

#### Scenario: Validation error
- **WHEN** request validation fails
- **THEN** the system SHALL return 400 with a JSON error message describing the validation failure

#### Scenario: Server error
- **WHEN** an internal error occurs
- **THEN** the system SHALL return 500 with a JSON error message
