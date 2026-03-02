## 1. Dependencies and Configuration

- [ ] 1.1 Remove bollard dependency from backend/Cargo.toml
- [x] 1.2 Add DEVCONTAINER_CLI_PATH to backend/src/config.rs
- [x] 1.3 Add devcontainer_cli_path field to Config struct
- [x] 1.4 Add environment variable parsing for DEVCONTAINER_CLI_PATH
- [x] 1.5 Set default value to "devcontainer" if not specified
- [x] 1.6 Update .env.example with DEVCONTAINER_CLI_PATH documentation
- [ ] 1.7 Document @devcontainers/cli installation requirements

## 2. DevContainer Service Implementation

- [x] 2.1 Create backend/src/services/devcontainer_service.rs module
- [x] 2.2 Define DevContainerService struct with cli_path and workspace_base_dir
- [x] 2.3 Define WorkspaceInfo struct with container_id, remote_user, remote_workspace_folder
- [x] 2.4 Define DevContainerOutput struct for JSON parsing
- [x] 2.5 Implement DevContainerService::new() constructor
- [x] 2.6 Implement check_cli_available() method to verify CLI in PATH
- [x] 2.7 Implement check_devcontainer_exists() method to detect .devcontainer/devcontainer.json
- [x] 2.8 Implement create_workspace() method with devcontainer up command
- [x] 2.9 Implement JSON output parsing (extract last line, parse as JSON)
- [x] 2.10 Implement container_id extraction from DevContainerOutput
- [x] 2.11 Implement remove_workspace() method with docker rm -f command
- [x] 2.12 Implement error handling for CLI not found
- [x] 2.13 Implement error handling for invalid JSON output
- [x] 2.14 Implement error handling for container creation timeout
- [x] 2.15 Implement detailed logging (INFO for operations, DEBUG for commands, TRACE for output)
- [x] 2.16 Write unit tests for DevContainerService methods
- [x] 2.17 Write unit tests for JSON parsing logic
- [x] 2.18 Write unit tests for error handling scenarios

## 3. Runtime Agent Installation

- [x] 3.1 Add install_agent() method to DevContainerService
- [x] 3.2 Define AgentConfig struct with agent_type and timeout
- [x] 3.3 Implement generate_bun_install_script() helper function
- [x] 3.4 Implement generate_opencode_install_script() helper function
- [x] 3.5 Implement combined installation script generation
- [x] 3.6 Implement docker exec command execution for installation
- [x] 3.7 Implement installation timeout handling (default 300 seconds)
- [x] 3.8 Implement Bun installation verification (bun --version)
- [x] 3.9 Implement OpenCode installation verification (opencode --version)
- [x] 3.10 Implement ACP support verification (opencode acp --help)
- [x] 3.11 Implement error handling for network failures
- [x] 3.12 Implement error handling for disk space issues
- [x] 3.13 Implement error handling for permission errors
- [x] 3.14 Implement detailed logging for installation steps
- [x] 3.15 Write unit tests for installation script generation
- [ ] 3.16 Write integration tests for agent installation

## 4. Default DevContainer Configuration

- [x] 4.1 Create default devcontainer.json template in code
- [x] 4.2 Set default image to ubuntu:22.04
- [x] 4.3 Set default remoteUser to root
- [x] 4.4 Set default workspaceFolder to /workspace
- [x] 4.5 Set default overrideCommand to true
- [x] 4.6 Implement create_default_config() method
- [x] 4.7 Implement temporary config file creation for default config
- [x] 4.8 Implement temporary config file cleanup after container creation
- [x] 4.9 Write unit tests for default configuration

## 5. DevContainer Configuration Validation

- [x] 5.1 Implement validate_devcontainer_json() method
- [x] 5.2 Implement JSON syntax validation
- [x] 5.3 Implement required fields validation (image or build)
- [x] 5.4 Implement warning for unsupported properties
- [x] 5.5 Implement error messages with file path and line numbers
- [x] 5.6 Write unit tests for validation logic
- [x] 5.7 Write unit tests for error message formatting

## 6. Workspace Service Integration

- [x] 6.1 Update backend/src/services/workspace_service.rs imports
- [x] 6.2 Add DevContainerService field to WorkspaceService struct
- [x] 6.3 Update WorkspaceService::new() to accept DevContainerService
- [x] 6.4 Refactor create_workspace() to use devcontainer_service.create_workspace()
- [x] 6.5 Add devcontainer.json existence check before creation
- [x] 6.6 Add agent installation call after container creation
- [x] 6.7 Update error handling to use DevContainerService errors
- [x] 6.8 Remove docker_service and container_service dependencies
- [x] 6.9 Update delete_workspace() to use devcontainer_service.remove_workspace()
- [ ] 6.10 Update workspace status checks to use docker inspect
- [ ] 6.11 Write integration tests for workspace creation flow
- [ ] 6.12 Write integration tests for workspace deletion flow

## 7. State Management Integration

- [x] 7.1 Update backend/src/state.rs to include DevContainerService
- [x] 7.2 Initialize DevContainerService in AppState::new()
- [x] 7.3 Pass DevContainerService to WorkspaceService
- [x] 7.4 Remove DockerService and ContainerService from AppState
- [x] 7.5 Update all service initialization code
- [ ] 7.6 Write unit tests for state initialization

## 8. Code Removal

- [ ] 8.1 Remove backend/src/services/docker_service.rs (1,805 lines)
- [ ] 8.2 Remove backend/src/services/container_service.rs (1,096 lines)
- [ ] 8.3 Remove backend/src/services/timeout_watchdog.rs (275 lines)
- [ ] 8.4 Remove all imports of docker_service module
- [ ] 8.5 Remove all imports of container_service module
- [ ] 8.6 Remove all imports of timeout_watchdog module
- [ ] 8.7 Remove all references to DockerService struct
- [ ] 8.8 Remove all references to ContainerService struct
- [ ] 8.9 Remove all references to TimeoutWatchdog struct
- [ ] 8.10 Update backend/src/services/mod.rs to remove old modules
- [ ] 8.11 Verify no remaining references with grep
- [ ] 8.12 Run cargo check to verify compilation

## 9. Task Executor Verification

- [ ] 9.1 Verify task_executor_service.rs requires no changes
- [ ] 9.2 Verify ACP communication via docker exec still works
- [ ] 9.3 Test end-to-end task execution with new DevContainerService
- [ ] 9.4 Verify PR creation flow is unchanged
- [ ] 9.5 Write integration tests for complete Issue-to-PR workflow

## 10. Testing

- [ ] 10.1 Write unit tests for DevContainerService (target: 20+ tests)
- [ ] 10.2 Write unit tests for agent installation (target: 10+ tests)
- [ ] 10.3 Write integration tests for workspace creation (target: 5+ tests)
- [ ] 10.4 Write integration tests for agent installation (target: 5+ tests)
- [ ] 10.5 Test with repository containing devcontainer.json
- [ ] 10.6 Test with repository without devcontainer.json (default config)
- [ ] 10.7 Test with devcontainer.json containing Features
- [ ] 10.8 Test error scenarios (CLI not found, invalid config, timeout)
- [ ] 10.9 Test concurrent workspace creation
- [ ] 10.10 Performance benchmark: container creation time (target: < 30s)
- [ ] 10.11 Performance benchmark: agent installation time (target: < 60s)
- [ ] 10.12 Performance benchmark: end-to-end workspace creation (target: < 90s)
- [ ] 10.13 Run all existing tests to ensure no regressions
- [ ] 10.14 Achieve 100% test pass rate

## 11. Documentation

- [ ] 11.1 Create docs/devcontainer-integration.md guide
- [ ] 11.2 Document @devcontainers/cli installation for Linux
- [ ] 11.3 Document @devcontainers/cli installation for macOS
- [ ] 11.4 Document @devcontainers/cli installation for Windows
- [ ] 11.5 Create devcontainer.json examples (minimal, with Features, with Dockerfile)
- [ ] 11.6 Document supported devcontainer.json properties
- [ ] 11.7 Document unsupported properties and workarounds
- [ ] 11.8 Create migration guide from old Docker service
- [ ] 11.9 Document troubleshooting common issues
- [ ] 11.10 Update docs/api/user-guide.md with devcontainer.json section
- [ ] 11.11 Update docs/development/README.md with new architecture
- [ ] 11.12 Update AGENTS.md with DevContainerService information
- [ ] 11.13 Update API documentation (Swagger) if needed
- [ ] 11.14 Create FAQ section for devcontainer.json usage

## 12. Configuration Examples

- [ ] 12.1 Create example: minimal devcontainer.json
- [ ] 12.2 Create example: devcontainer.json with Node.js Feature
- [ ] 12.3 Create example: devcontainer.json with Python Feature
- [ ] 12.4 Create example: devcontainer.json with custom Dockerfile
- [ ] 12.5 Create example: devcontainer.json with lifecycle hooks
- [ ] 12.6 Create example: devcontainer.json with environment variables
- [ ] 12.7 Add examples to docs/examples/devcontainer/ directory
- [ ] 12.8 Document each example with use case and benefits

## 13. Error Handling and User Experience

- [ ] 13.1 Implement user-friendly error message for CLI not found
- [ ] 13.2 Implement user-friendly error message for invalid devcontainer.json
- [ ] 13.3 Implement user-friendly error message for Docker daemon not running
- [ ] 13.4 Implement user-friendly error message for image pull failure
- [ ] 13.5 Implement user-friendly error message for agent installation failure
- [ ] 13.6 Add suggestions to error messages (e.g., "Run: npm install -g @devcontainers/cli")
- [ ] 13.7 Add links to documentation in error messages
- [ ] 13.8 Test error messages with real failure scenarios

## 14. Deployment Preparation

- [ ] 14.1 Create deployment checklist
- [ ] 14.2 Document @devcontainers/cli installation on production servers
- [ ] 14.3 Create installation script for @devcontainers/cli
- [ ] 14.4 Update deployment documentation with new requirements
- [ ] 14.5 Create rollback plan documentation
- [ ] 14.6 Add feature flag USE_DEVCONTAINER for gradual rollout
- [ ] 14.7 Implement feature flag logic in WorkspaceService
- [ ] 14.8 Test feature flag switching between old and new systems
- [ ] 14.9 Document feature flag usage for operators

## 15. Performance Optimization

- [ ] 15.1 Implement CLI output streaming for progress feedback
- [ ] 15.2 Implement agent binary caching (optional, via AGENT_CACHE_DIR)
- [ ] 15.3 Add performance metrics logging (container creation time, agent install time)
- [ ] 15.4 Optimize JSON parsing (use serde_json streaming if needed)
- [ ] 15.5 Profile memory usage during container creation
- [ ] 15.6 Profile CPU usage during agent installation
- [ ] 15.7 Document performance characteristics and benchmarks

## 16. Staging Deployment

- [ ] 16.1 Deploy to staging environment
- [ ] 16.2 Install @devcontainers/cli on staging servers
- [ ] 16.3 Verify CLI is in PATH and executable
- [ ] 16.4 Run smoke tests (create workspace, install agent, execute task)
- [ ] 16.5 Monitor logs for errors and warnings
- [ ] 16.6 Test with real repositories from staging
- [ ] 16.7 Measure performance metrics (creation time, success rate)
- [ ] 16.8 Collect feedback from staging testing
- [ ] 16.9 Fix any issues found in staging
- [ ] 16.10 Verify rollback procedure works

## 17. Production Rollout

- [ ] 17.1 Create production deployment plan
- [ ] 17.2 Schedule deployment window
- [ ] 17.3 Notify users of upcoming changes
- [ ] 17.4 Deploy to production with feature flag disabled
- [ ] 17.5 Enable feature flag for 10% of workspaces
- [ ] 17.6 Monitor error rates and performance for 24 hours
- [ ] 17.7 Increase to 50% if no issues
- [ ] 17.8 Monitor for another 24 hours
- [ ] 17.9 Increase to 100% if no issues
- [ ] 17.10 Remove feature flag after 1 week of stable operation
- [ ] 17.11 Monitor long-term stability and performance

## 18. Post-Deployment

- [ ] 18.1 Collect user feedback on devcontainer.json support
- [ ] 18.2 Monitor support requests for common issues
- [ ] 18.3 Update documentation based on user feedback
- [ ] 18.4 Create additional examples based on user needs
- [ ] 18.5 Plan future enhancements (prebuild support, multi-container)
- [ ] 18.6 Update roadmap with lessons learned
- [ ] 18.7 Celebrate 81% code reduction achievement! 🎉

---

## Summary

**Total Tasks:** 180+  
**Estimated Duration:** 7-10 days  
**Code Reduction:** ~2,476 lines (81%)  
**Key Milestones:**
- Day 1-3: Implementation (Tasks 1-7)
- Day 4-5: Testing (Tasks 10-11)
- Day 6-7: Documentation and Cleanup (Tasks 8, 11-15)
- Day 8-10: Deployment (Tasks 16-18)

**Success Criteria:**
- ✅ All tests passing (100% pass rate)
- ✅ Container creation < 30s
- ✅ Agent installation < 60s
- ✅ End-to-end workspace creation < 90s
- ✅ Zero regressions in existing functionality
- ✅ Documentation complete and accurate
- ✅ Successful production deployment
