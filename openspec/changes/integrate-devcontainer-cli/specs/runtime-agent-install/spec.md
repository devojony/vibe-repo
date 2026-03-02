## ADDED Requirements

### Requirement: System SHALL install agents at runtime after container creation
The system SHALL install AI agents (Bun + OpenCode) at runtime via docker exec after workspace container is created.

#### Scenario: Install agent after container creation
- **WHEN** workspace container is created successfully
- **THEN** system SHALL call devcontainer_service.install_agent()
- **THEN** system SHALL pass container_id and agent configuration
- **THEN** system SHALL wait for installation to complete
- **THEN** system SHALL verify installation succeeded

#### Scenario: Generate agent installation script
- **WHEN** system needs to install agent
- **THEN** system SHALL generate bash installation script
- **THEN** script SHALL install Bun runtime first
- **THEN** script SHALL install OpenCode using Bun
- **THEN** script SHALL verify installations
- **THEN** script SHALL set up PATH environment variables

#### Scenario: Execute installation script in container
- **WHEN** installation script is ready
- **THEN** system SHALL run: docker exec <container_id> bash -c "<script>"
- **THEN** system SHALL capture stdout and stderr
- **THEN** system SHALL wait for script completion
- **THEN** system SHALL check exit code

### Requirement: System SHALL install Bun runtime
The system SHALL install Bun JavaScript runtime as the foundation for running OpenCode.

#### Scenario: Install Bun from official installer
- **WHEN** installing Bun
- **THEN** system SHALL run: curl -fsSL https://bun.sh/install | bash
- **THEN** system SHALL set BUN_INSTALL environment variable
- **THEN** system SHALL add Bun to PATH
- **THEN** system SHALL verify Bun is executable

#### Scenario: Handle Bun installation failure
- **WHEN** Bun installation fails
- **THEN** system SHALL capture error output
- **THEN** system SHALL return installation error
- **THEN** system SHALL include curl error details if network issue
- **THEN** system SHALL suggest checking network connectivity

#### Scenario: Verify Bun installation
- **WHEN** Bun installation completes
- **THEN** system SHALL run: bun --version
- **THEN** system SHALL verify version output is present
- **THEN** system SHALL log Bun version at INFO level

#### Scenario: Skip Bun installation if already present
- **WHEN** Bun is already installed in container
- **THEN** system SHALL detect existing Bun installation
- **THEN** system SHALL skip installation step
- **THEN** system SHALL log that Bun is already present
- **THEN** system SHALL verify version is compatible

### Requirement: System SHALL install OpenCode agent
The system SHALL install OpenCode AI agent using Bun package manager.

#### Scenario: Install OpenCode globally
- **WHEN** installing OpenCode
- **THEN** system SHALL run: bun install -g opencode-ai
- **THEN** system SHALL wait for installation to complete
- **THEN** system SHALL verify opencode command is available

#### Scenario: Handle OpenCode installation failure
- **WHEN** OpenCode installation fails
- **THEN** system SHALL capture error output
- **THEN** system SHALL return installation error
- **THEN** system SHALL include npm registry error if present
- **THEN** system SHALL suggest checking npm registry status

#### Scenario: Verify OpenCode installation
- **WHEN** OpenCode installation completes
- **THEN** system SHALL run: opencode --version
- **THEN** system SHALL verify version output is present
- **THEN** system SHALL log OpenCode version at INFO level

#### Scenario: Verify OpenCode ACP support
- **WHEN** OpenCode is installed
- **THEN** system SHALL run: opencode acp --help
- **THEN** system SHALL verify ACP command is available
- **THEN** system SHALL log that ACP support is confirmed

### Requirement: System SHALL handle agent installation timeouts
The system SHALL enforce timeouts for agent installation to prevent hanging.

#### Scenario: Set installation timeout
- **WHEN** starting agent installation
- **THEN** system SHALL set timeout to 300 seconds (5 minutes)
- **THEN** system SHALL monitor installation progress
- **THEN** system SHALL terminate if timeout exceeded

#### Scenario: Handle installation timeout
- **WHEN** agent installation exceeds timeout
- **THEN** system SHALL kill docker exec process
- **THEN** system SHALL return timeout error
- **THEN** system SHALL include partial output in error
- **THEN** system SHALL suggest increasing timeout if needed

### Requirement: System SHALL log agent installation progress
The system SHALL log agent installation steps for debugging and monitoring.

#### Scenario: Log installation start
- **WHEN** starting agent installation
- **THEN** system SHALL log container_id at INFO level
- **THEN** system SHALL log agent type (OpenCode) at INFO level
- **THEN** system SHALL log installation script at DEBUG level

#### Scenario: Log installation steps
- **WHEN** installation is in progress
- **THEN** system SHALL log "Installing Bun..." at INFO level
- **THEN** system SHALL log "Installing OpenCode..." at INFO level
- **THEN** system SHALL log "Verifying installation..." at INFO level

#### Scenario: Log installation completion
- **WHEN** installation completes successfully
- **THEN** system SHALL log "Agent installed successfully" at INFO level
- **THEN** system SHALL log installation duration at INFO level
- **THEN** system SHALL log Bun and OpenCode versions at INFO level

#### Scenario: Log installation failure
- **WHEN** installation fails
- **THEN** system SHALL log error at ERROR level
- **THEN** system SHALL include container_id in error
- **THEN** system SHALL include full error output
- **THEN** system SHALL include installation step that failed

### Requirement: System SHALL support configurable agent installation
The system SHALL support configuration for agent type and installation parameters.

#### Scenario: Use agent configuration from environment
- **WHEN** installing agent
- **THEN** system SHALL read AGENT_TYPE from configuration
- **THEN** system SHALL read AGENT_TIMEOUT_SECONDS from configuration
- **THEN** system SHALL use configured values for installation

#### Scenario: Support different agent types
- **WHEN** AGENT_TYPE is "opencode"
- **THEN** system SHALL install Bun + OpenCode
- **WHEN** AGENT_TYPE is "claude-code"
- **THEN** system SHALL install Node.js + Claude Code
- **WHEN** AGENT_TYPE is unsupported
- **THEN** system SHALL return configuration error

#### Scenario: Support custom installation scripts
- **WHEN** custom installation script is provided
- **THEN** system SHALL use custom script instead of default
- **THEN** system SHALL validate script before execution
- **THEN** system SHALL log that custom script is being used

### Requirement: System SHALL handle agent installation errors gracefully
The system SHALL provide clear error messages for common installation failures.

#### Scenario: Network error during installation
- **WHEN** network is unavailable during installation
- **THEN** error message SHALL indicate network issue
- **THEN** error message SHALL suggest checking internet connectivity
- **THEN** error message SHALL include curl/npm error details

#### Scenario: Insufficient disk space
- **WHEN** container has insufficient disk space
- **THEN** error message SHALL indicate disk space issue
- **THEN** error message SHALL show required vs available space
- **THEN** error message SHALL suggest increasing container disk limit

#### Scenario: Permission denied error
- **WHEN** installation fails due to permissions
- **THEN** error message SHALL indicate permission issue
- **THEN** error message SHALL suggest running as root
- **THEN** error message SHALL include file path that failed

#### Scenario: Incompatible architecture
- **WHEN** container architecture is unsupported
- **THEN** error message SHALL indicate architecture mismatch
- **THEN** error message SHALL show container architecture
- **THEN** error message SHALL list supported architectures

### Requirement: System SHALL clean up on installation failure
The system SHALL clean up resources when agent installation fails.

#### Scenario: Remove container on installation failure
- **WHEN** agent installation fails
- **THEN** system SHALL call devcontainer_service.remove_workspace()
- **THEN** system SHALL remove container to avoid orphaned containers
- **THEN** system SHALL log cleanup action at INFO level

#### Scenario: Preserve container for debugging
- **WHEN** DEBUG_PRESERVE_FAILED_CONTAINERS is true
- **THEN** system SHALL NOT remove container on failure
- **THEN** system SHALL log container_id for manual inspection
- **THEN** system SHALL add label indicating failed installation

## MODIFIED Requirements

### Requirement: Workspace creation SHALL include agent installation
The workspace creation flow SHALL be updated to include agent installation as a required step.

#### Scenario: Install agent after container creation
- **WHEN** WorkspaceService creates workspace
- **THEN** WorkspaceService SHALL call devcontainer_service.create_workspace()
- **THEN** WorkspaceService SHALL call devcontainer_service.install_agent()
- **THEN** WorkspaceService SHALL only mark workspace as ready after agent installation
- **THEN** WorkspaceService SHALL clean up container if agent installation fails

#### Scenario: Store agent installation status
- **WHEN** agent installation completes
- **THEN** WorkspaceService SHALL update workspace record
- **THEN** WorkspaceService SHALL store agent version in database
- **THEN** WorkspaceService SHALL store installation timestamp

## REMOVED Requirements

### Requirement: Agents SHALL NOT be pre-installed in Docker images
Agents SHALL be installed at runtime, not baked into Docker images.

#### Scenario: Remove agent from base images
- **WHEN** building Docker images
- **THEN** images SHALL NOT include Bun
- **THEN** images SHALL NOT include OpenCode
- **THEN** images SHALL only include base OS and essential tools

#### Scenario: Remove agent installation from Dockerfile
- **WHEN** updating Dockerfile
- **THEN** Dockerfile SHALL NOT have RUN commands for Bun installation
- **THEN** Dockerfile SHALL NOT have RUN commands for OpenCode installation
- **THEN** Dockerfile SHALL be minimal and generic
