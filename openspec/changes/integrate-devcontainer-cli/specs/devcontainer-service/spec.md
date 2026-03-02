## ADDED Requirements

### Requirement: DevContainer service SHALL wrap @devcontainers/cli for container operations
The DevContainer service SHALL provide a Rust interface to @devcontainers/cli for creating, managing, and removing workspace containers.

#### Scenario: Initialize service with CLI path
- **WHEN** service is created
- **THEN** service SHALL accept devcontainer CLI path from configuration
- **THEN** service SHALL default to "devcontainer" if not specified
- **THEN** service SHALL store workspace base directory path

#### Scenario: Verify CLI availability
- **WHEN** service starts
- **THEN** service SHALL check if devcontainer CLI is available in PATH
- **THEN** service SHALL return error if CLI not found
- **THEN** service SHALL log CLI version for debugging

### Requirement: DevContainer service SHALL create workspace containers
The DevContainer service SHALL create workspace containers using devcontainer CLI with proper configuration and error handling.

#### Scenario: Create workspace with devcontainer.json
- **WHEN** repository has .devcontainer/devcontainer.json
- **THEN** service SHALL run: npx @devcontainers/cli up --workspace-folder <path>
- **THEN** service SHALL use --log-format json for structured output
- **THEN** service SHALL add --id-label for container identification
- **THEN** service SHALL parse last line of output as JSON result
- **THEN** service SHALL extract container_id from result
- **THEN** service SHALL return WorkspaceInfo with container details

#### Scenario: Create workspace with default configuration
- **WHEN** repository has no .devcontainer/devcontainer.json
- **THEN** service SHALL use default configuration (ubuntu:22.04 base image)
- **THEN** service SHALL create temporary devcontainer.json
- **THEN** service SHALL run devcontainer up with temporary config
- **THEN** service SHALL clean up temporary config after creation

#### Scenario: Handle container creation timeout
- **WHEN** devcontainer up exceeds configured timeout
- **THEN** service SHALL terminate CLI process
- **THEN** service SHALL return timeout error
- **THEN** service SHALL clean up any partially created containers

#### Scenario: Handle container creation failure
- **WHEN** devcontainer up fails with non-zero exit code
- **THEN** service SHALL capture stderr output
- **THEN** service SHALL parse error message if JSON format
- **THEN** service SHALL return user-friendly error with details
- **THEN** service SHALL log full CLI output for debugging

### Requirement: DevContainer service SHALL manage container lifecycle
The DevContainer service SHALL provide operations for starting, stopping, and removing workspace containers.

#### Scenario: Check if container is running
- **WHEN** service needs to verify container status
- **THEN** service SHALL run: docker inspect <container_id>
- **THEN** service SHALL parse State.Running field
- **THEN** service SHALL return boolean status

#### Scenario: Remove workspace container
- **WHEN** workspace is no longer needed
- **THEN** service SHALL run: docker rm -f <container_id>
- **THEN** service SHALL verify container is removed
- **THEN** service SHALL clean up any associated resources
- **THEN** service SHALL ignore errors if container already removed

#### Scenario: Handle container removal failure
- **WHEN** docker rm fails
- **THEN** service SHALL log error details
- **THEN** service SHALL retry removal once after 2 second delay
- **THEN** service SHALL return error if retry fails

### Requirement: DevContainer service SHALL parse CLI output reliably
The DevContainer service SHALL parse devcontainer CLI JSON output and extract required information.

#### Scenario: Parse successful devcontainer up output
- **WHEN** devcontainer up completes successfully
- **THEN** service SHALL read all stdout lines
- **THEN** service SHALL extract last line as result JSON
- **THEN** service SHALL parse JSON into DevContainerOutput struct
- **THEN** service SHALL extract containerId field (required)
- **THEN** service SHALL extract remoteUser field (optional)
- **THEN** service SHALL extract remoteWorkspaceFolder field (optional)

#### Scenario: Handle missing required fields
- **WHEN** JSON output is missing containerId field
- **THEN** service SHALL return parse error
- **THEN** service SHALL include raw JSON in error message

#### Scenario: Handle malformed JSON output
- **WHEN** last line is not valid JSON
- **THEN** service SHALL return parse error
- **THEN** service SHALL log full CLI output for debugging
- **THEN** service SHALL include helpful error message

### Requirement: DevContainer service SHALL check for devcontainer.json existence
The DevContainer service SHALL detect whether a repository has a devcontainer.json configuration file.

#### Scenario: Check standard location
- **WHEN** service checks for devcontainer.json
- **THEN** service SHALL look for .devcontainer/devcontainer.json in repository root
- **THEN** service SHALL return true if file exists and is readable
- **THEN** service SHALL return false if file does not exist

#### Scenario: Validate devcontainer.json syntax
- **WHEN** devcontainer.json exists
- **THEN** service SHALL attempt to parse as JSON
- **THEN** service SHALL return warning if JSON is invalid
- **THEN** service SHALL continue with default config if invalid

### Requirement: DevContainer service SHALL provide detailed error messages
The DevContainer service SHALL provide actionable error messages for common failure scenarios.

#### Scenario: CLI not found error
- **WHEN** devcontainer CLI is not in PATH
- **THEN** error message SHALL include installation instructions
- **THEN** error message SHALL mention required Node.js dependency
- **THEN** error message SHALL provide link to documentation

#### Scenario: Invalid configuration error
- **WHEN** devcontainer.json has syntax errors
- **THEN** error message SHALL include file path
- **THEN** error message SHALL include JSON parse error details
- **THEN** error message SHALL suggest validation tools

#### Scenario: Docker daemon error
- **WHEN** Docker daemon is not running
- **THEN** error message SHALL indicate Docker is not available
- **THEN** error message SHALL suggest starting Docker daemon
- **THEN** error message SHALL include platform-specific instructions

#### Scenario: Image pull error
- **WHEN** base image cannot be pulled
- **THEN** error message SHALL include image name
- **THEN** error message SHALL indicate network or registry issue
- **THEN** error message SHALL suggest checking Docker Hub status

### Requirement: DevContainer service SHALL log operations for debugging
The DevContainer service SHALL log all operations with appropriate detail levels.

#### Scenario: Log container creation
- **WHEN** creating workspace container
- **THEN** service SHALL log workspace_id and repository path at INFO level
- **THEN** service SHALL log full CLI command at DEBUG level
- **THEN** service SHALL log CLI output at TRACE level
- **THEN** service SHALL log container_id at INFO level on success

#### Scenario: Log errors with context
- **WHEN** operation fails
- **THEN** service SHALL log error at ERROR level
- **THEN** service SHALL include operation context (workspace_id, command)
- **THEN** service SHALL include full error details
- **THEN** service SHALL include CLI stdout and stderr

#### Scenario: Log performance metrics
- **WHEN** operation completes
- **THEN** service SHALL log operation duration at INFO level
- **THEN** service SHALL include operation type (create, remove)
- **THEN** service SHALL include workspace_id for correlation

## MODIFIED Requirements

### Requirement: Workspace creation SHALL use DevContainer service instead of Docker service
The workspace creation flow SHALL be updated to use DevContainerService instead of direct Docker API operations.

#### Scenario: Create workspace via DevContainer service
- **WHEN** WorkspaceService creates a workspace
- **THEN** WorkspaceService SHALL call devcontainer_service.create_workspace()
- **THEN** WorkspaceService SHALL NOT call docker_service methods
- **THEN** WorkspaceService SHALL store returned container_id in database

#### Scenario: Handle workspace creation errors
- **WHEN** DevContainerService returns error
- **THEN** WorkspaceService SHALL propagate error to caller
- **THEN** WorkspaceService SHALL NOT create workspace record in database
- **THEN** WorkspaceService SHALL log error with workspace context

## REMOVED Requirements

### Requirement: Docker service SHALL NOT be used for container operations
The Docker service (docker_service.rs) SHALL be removed and replaced by DevContainerService.

#### Scenario: Remove docker_service.rs
- **WHEN** DevContainerService is fully integrated
- **THEN** docker_service.rs file SHALL be deleted
- **THEN** All imports of docker_service SHALL be removed
- **THEN** All references to DockerService SHALL be removed

### Requirement: Container service SHALL NOT be used for lifecycle management
The Container service (container_service.rs) SHALL be removed and replaced by DevContainerService.

#### Scenario: Remove container_service.rs
- **WHEN** DevContainerService is fully integrated
- **THEN** container_service.rs file SHALL be deleted
- **THEN** All imports of container_service SHALL be removed
- **THEN** All references to ContainerService SHALL be removed

### Requirement: Timeout watchdog SHALL NOT be used for monitoring
The Timeout watchdog service (timeout_watchdog.rs) SHALL be removed as devcontainer CLI handles timeouts.

#### Scenario: Remove timeout_watchdog.rs
- **WHEN** DevContainerService is fully integrated
- **THEN** timeout_watchdog.rs file SHALL be deleted
- **THEN** All imports of timeout_watchdog SHALL be removed
- **THEN** Timeout handling SHALL be done via CLI process timeout
