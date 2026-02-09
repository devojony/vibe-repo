## 1. Dependencies and Configuration

- [x] 1.1 Add tokio with full features to Cargo.toml
- [x] 1.2 Add serde_json for JSON-RPC serialization to Cargo.toml
- [x] 1.3 Add async-trait for trait definitions to Cargo.toml
- [x] 1.4 Update backend/src/config.rs to add AgentSettings struct
- [x] 1.5 Add agent configuration fields (agent_type, api_key, default_model, timeout)
- [x] 1.6 Add environment variable parsing for agent configuration

## 2. Database Schema Changes

- [x] 2.1 Create migration to add plans JSONB field to tasks table
- [x] 2.2 Create migration to add events JSONB field to tasks table
- [x] 2.3 Update backend/src/entities/tasks.rs to add plans field
- [x] 2.4 Update backend/src/entities/tasks.rs to add events field
- [x] 2.5 Test migrations with both SQLite and PostgreSQL

## 3. ACP Client Implementation

- [x] 3.1 Create backend/src/services/acp/mod.rs module
- [x] 3.2 Create backend/src/services/acp/client.rs with AcpClient struct
- [x] 3.3 Implement JSON-RPC request/response serialization
- [x] 3.4 Implement NDJSON parsing for stdin/stdout communication
- [x] 3.5 Implement initialize method with capability negotiation
- [x] 3.6 Implement newSession method with working directory
- [x] 3.7 Implement prompt method with streaming response handling
- [x] 3.8 Implement cancel method for task cancellation
- [x] 3.9 Add error handling for JSON parse errors and protocol violations
- [x] 3.10 Add timeout handling with configurable duration
- [x] 3.11 Write unit tests for ACP client methods

## 4. Agent Manager Service

- [x] 4.1 Create backend/src/services/agent_manager.rs
- [x] 4.2 Define AgentType enum (OpenCode, ClaudeCode)
- [x] 4.3 Define AgentConfig struct with agent settings
- [x] 4.4 Implement spawn_agent method for subprocess creation
- [x] 4.5 Implement spawn_opencode method with Bun runtime
- [x] 4.6 Implement spawn_claude_code method with adapter
- [x] 4.7 Implement process health monitoring
- [x] 4.8 Implement graceful shutdown with timeout
- [x] 4.9 Implement force kill on unresponsive agents
- [x] 4.10 Add concurrent task limit enforcement
- [x] 4.11 Add resource usage tracking (CPU, memory)
- [x] 4.12 Write unit tests for agent lifecycle management

## 5. Permission System

- [x] 5.1 Create backend/src/services/acp/permissions.rs
- [x] 5.2 Define PermissionPolicy struct with rules
- [x] 5.3 Implement default permission policy (allow read, workspace writes, safe commands)
- [x] 5.4 Implement evaluate_permission method for policy evaluation
- [x] 5.5 Implement path validation (within workspace check)
- [x] 5.6 Implement command allowlist (git, cargo, npm, etc.)
- [x] 5.7 Implement command denylist (rm -rf, dd, mkfs, etc.)
- [x] 5.8 Implement permission logging to tasks.events
- [x] 5.9 Add repository-specific policy loading
- [x] 5.10 Write unit tests for permission evaluation

## 6. Event Streaming and Storage

- [x] 6.1 Create backend/src/services/acp/events.rs
- [x] 6.2 Define AgentEvent enum (Plan, ToolCall, Message, Completed)
- [x] 6.3 Implement event parsing from ACP sessionUpdate notifications
- [x] 6.4 Implement plan extraction and storage
- [x] 6.5 Implement tool call extraction and storage
- [x] 6.6 Implement message extraction and storage
- [x] 6.7 Implement event compaction (keep last 100 events)
- [x] 6.8 Implement progress calculation from plans
- [x] 6.9 Add event query methods for API
- [x] 6.10 Write unit tests for event processing

## 7. Task Executor Refactoring

- [x] 7.1 Backup current task_executor_service.rs implementation
- [x] 7.2 Refactor execute_task method to use Agent Manager
- [x] 7.3 Remove docker exec CLI execution code
- [x] 7.4 Implement ACP-based task execution flow
- [x] 7.5 Implement real-time event streaming to database
- [x] 7.6 Implement permission request handling
- [x] 7.7 Implement task cancellation via ACP
- [x] 7.8 Preserve PR creation logic with git operation extraction
- [x] 7.9 Add error handling for agent crashes
- [x] 7.10 Add retry logic for transient failures
- [x] 7.11 Write integration tests for task execution

## 8. Docker Image Updates

- [x] 8.1 Update docker/workspace/Dockerfile to install Bun
- [x] 8.2 Add Bun installation from official source
- [x] 8.3 Install OpenCode via "bun install -g opencode-ai"
- [x] 8.4 Verify OpenCode ACP support with "opencode acp --version"
- [x] 8.5 Optionally install Claude Code adapter
- [x] 8.6 Test Docker image build locally
- [x] 8.7 Verify agent startup time improvements
- [x] 8.8 Update Docker image documentation

## 9. API Updates

- [x] 9.1 Add GET /tasks/:id/plans endpoint for plan retrieval
- [x] 9.2 Add GET /tasks/:id/events endpoint for event retrieval
- [x] 9.3 Add GET /tasks/:id/progress endpoint for progress tracking
- [x] 9.4 Update GET /tasks/:id/status to include progress percentage
- [x] 9.5 Add query parameters for event filtering (type, time range)
- [x] 9.6 Update OpenAPI documentation for new endpoints
- [x] 9.7 Write integration tests for new API endpoints

## 10. Testing

- [x] 10.1 Write unit tests for ACP client (JSON-RPC, NDJSON parsing)
- [x] 10.2 Write unit tests for Agent Manager (subprocess lifecycle)
- [x] 10.3 Write unit tests for permission system (policy evaluation)
- [x] 10.4 Write unit tests for event processing (parsing, storage)
- [x] 10.5 Write integration tests for task execution with OpenCode
- [x] 10.6 Write integration tests for permission handling
- [x] 10.7 Write integration tests for event streaming
- [x] 10.8 Write E2E tests for complete Issue-to-PR workflow
- [x] 10.9 Test with real GitHub repositories
- [x] 10.10 Test error scenarios (agent crash, timeout, permission denial)

## 11. Performance Testing

- [x] 11.1 Benchmark agent startup time (Bun vs Node.js)
- [x] 11.2 Benchmark task execution time (ACP vs CLI)
- [x] 11.3 Benchmark memory usage per agent
- [x] 11.4 Test concurrent task execution (multiple agents)
- [x] 11.5 Test event storage performance with large event counts
- [x] 11.6 Profile and optimize hot paths
- [x] 11.7 Document performance improvements

## 12. Documentation

- [x] 12.1 Update AGENTS.md with ACP integration details
- [x] 12.2 Document agent configuration options
- [x] 12.3 Document permission policy configuration
- [x] 12.4 Document event structure and query API
- [x] 12.5 Add troubleshooting guide for common issues
- [x] 12.6 Update API documentation with new endpoints
- [x] 12.7 Add examples for using different agents (OpenCode, Claude Code)
- [x] 12.8 Document migration from old CLI approach

## 13. Deployment

- [x] 13.1 Create deployment checklist
- [x] 13.2 Test in staging environment
- [x] 13.3 Monitor metrics (startup time, success rate, errors)
- [x] 13.4 Gradual rollout to production
- [x] 13.5 Monitor production metrics
- [x] 13.6 Document rollback procedure
- [x] 13.7 Create runbook for common operational tasks
