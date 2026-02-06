# Minimal API Surface

## Purpose

This specification defines the minimal API surface for VibeRepo v0.4.0-mvp. The system exposes only 10 core endpoints necessary for Issue-to-PR automation, removing all management and monitoring APIs.

## Requirements

### Requirement: API SHALL expose exactly 10 core endpoints

The system SHALL provide exactly 10 API endpoints covering the essential Issue-to-PR automation workflow. All other endpoints SHALL be removed.

#### Scenario: Repository list endpoint available
- **WHEN** a GET request is made to /api/repositories
- **THEN** the system SHALL return a list of repositories

#### Scenario: Repository details endpoint available
- **WHEN** a GET request is made to /api/repositories/:id
- **THEN** the system SHALL return repository details

#### Scenario: Repository initialization endpoint available
- **WHEN** a POST request is made to /api/repositories/:id/initialize
- **THEN** the system SHALL initialize the repository with required branches and labels

#### Scenario: Batch initialization endpoint available
- **WHEN** a POST request is made to /api/repositories/batch-initialize
- **THEN** the system SHALL initialize multiple repositories

#### Scenario: Webhook endpoint available
- **WHEN** a POST request is made to /api/webhooks/:repository_id
- **THEN** the system SHALL process the webhook payload and create tasks as needed

#### Scenario: Task list endpoint available
- **WHEN** a GET request is made to /api/tasks
- **THEN** the system SHALL return a list of tasks with optional filtering

#### Scenario: Task details endpoint available
- **WHEN** a GET request is made to /api/tasks/:id
- **THEN** the system SHALL return task details including status and logs

#### Scenario: Task creation endpoint available
- **WHEN** a POST request is made to /api/tasks
- **THEN** the system SHALL create a new task

#### Scenario: Task update endpoint available
- **WHEN** a PATCH request is made to /api/tasks/:id
- **THEN** the system SHALL update task priority

#### Scenario: Task deletion endpoint available
- **WHEN** a DELETE request is made to /api/tasks/:id
- **THEN** the system SHALL soft delete the specified task

#### Scenario: Task execution endpoint available
- **WHEN** a POST request is made to /api/tasks/:id/execute
- **THEN** the system SHALL start executing the specified task

#### Scenario: PR creation endpoint available
- **WHEN** a POST request is made to /api/tasks/:id/create-pr
- **THEN** the system SHALL manually create a pull request for the task

#### Scenario: Issue closure endpoint available
- **WHEN** a POST request is made to /api/tasks/:id/close-issue
- **THEN** the system SHALL manually close the associated issue

#### Scenario: Health check endpoint available
- **WHEN** a GET request is made to /health
- **THEN** the system SHALL return health status

### Requirement: Provider configuration SHALL use environment variables

The system SHALL NOT provide API endpoints for managing Git providers. Provider configuration SHALL be done via environment variables or configuration files.

#### Scenario: No provider management endpoints
- **WHEN** attempting to access /api/settings/providers endpoints
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
- **THEN** the system SHALL configure the agent using environment variables (e.g., DEFAULT_AGENT_COMMAND, DEFAULT_AGENT_TIMEOUT)

### Requirement: Webhook configuration SHALL be automatic

The system SHALL NOT provide API endpoints for configuring webhooks. Webhook URLs and secrets SHALL be configured via environment variables.

#### Scenario: No webhook configuration endpoints
- **WHEN** attempting to access /api/webhooks/config endpoints
- **THEN** the system SHALL return 404 Not Found

#### Scenario: Webhook secret configured via environment
- **WHEN** a webhook is received
- **THEN** the system SHALL validate the signature using the WEBHOOK_SECRET environment variable

### Requirement: Statistics and monitoring endpoints SHALL be removed

The system SHALL NOT provide API endpoints for statistics, metrics, or advanced monitoring. Users SHALL rely on external monitoring tools or database queries.

#### Scenario: No statistics endpoints
- **WHEN** attempting to access /api/stats endpoints
- **THEN** the system SHALL return 404 Not Found

#### Scenario: No advanced health check endpoints
- **WHEN** attempting to access /api/health/detailed endpoints
- **THEN** the system SHALL return 404 Not Found

### Requirement: API documentation SHALL reflect minimal surface

The system SHALL provide OpenAPI documentation that includes only the 10 core endpoints.

#### Scenario: OpenAPI spec includes only core endpoints
- **WHEN** accessing /swagger-ui or /api-docs/openapi.json
- **THEN** the documentation SHALL list exactly 10 endpoints plus health check

#### Scenario: Removed endpoints not documented
- **WHEN** viewing API documentation
- **THEN** removed endpoints (providers, workspaces, agents, webhook config, stats) SHALL NOT appear

### Requirement: Task filtering SHALL support basic queries

The GET /api/tasks endpoint SHALL support basic filtering by status, workspace, and priority.

#### Scenario: Filter tasks by status
- **WHEN** a GET request is made to /api/tasks?status=pending
- **THEN** the system SHALL return only tasks with Pending status

#### Scenario: Filter tasks by workspace
- **WHEN** a GET request is made to /api/tasks?workspace_id=123
- **THEN** the system SHALL return only tasks for workspace 123

#### Scenario: Filter tasks by priority
- **WHEN** a GET request is made to /api/tasks?priority=High
- **THEN** the system SHALL return only high-priority tasks

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
