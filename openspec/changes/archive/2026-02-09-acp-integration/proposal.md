## Why

The current Docker + CLI execution approach lacks real-time progress tracking, structured communication, and permission control. By integrating the Agent Client Protocol (ACP) with OpenCode and Bun runtime, we can achieve 10x faster startup times, real-time task monitoring, and a standardized agent interface that supports future extensibility.

## What Changes

- Add ACP client implementation in Rust for JSON-RPC communication over stdin/stdout
- Integrate OpenCode as the primary AI agent with native ACP support
- Replace Node.js with Bun runtime for 10x faster agent startup
- Implement real-time event streaming for task progress, tool calls, and agent messages
- Add permission management system for controlling agent actions
- Refactor Task Executor to use ACP instead of docker exec
- Update Docker images to include Bun and OpenCode
- Add configuration for agent selection (OpenCode, Claude Code, etc.)

## Capabilities

### New Capabilities
- `acp-client`: Rust implementation of ACP client with JSON-RPC over stdin/stdout communication
- `agent-manager`: Service for spawning, managing, and monitoring ACP-compatible agents
- `opencode-integration`: OpenCode agent integration with Bun runtime
- `real-time-events`: Event streaming system for task progress, tool calls, and agent messages
- `permission-system`: Permission management for controlling agent file operations and commands

### Modified Capabilities
- `task-execution`: Task execution now uses ACP protocol instead of CLI output parsing

## Impact

**Affected Code:**
- `backend/src/services/task_executor_service.rs` - Complete refactor to use ACP
- `backend/src/services/agent_service.rs` - New service for agent management
- `backend/src/config.rs` - Add agent configuration options
- `backend/src/entities/tasks.rs` - Add fields for real-time events and plans

**Dependencies:**
- Add `tokio` with full features for async subprocess management
- Add `serde_json` for JSON-RPC serialization
- Add Bun to Docker images (~90MB, single binary)
- Add OpenCode via `bun install -g opencode-ai`

**Database Schema:**
- Add `plans` JSONB field to `tasks` table for storing agent plans
- Add `events` JSONB field to `tasks` table for storing real-time events

**Docker Images:**
- Update `docker/workspace/Dockerfile` to install Bun and OpenCode
- Estimated size increase: ~90MB (Bun) + ~50MB (OpenCode) = ~140MB

**APIs:**
- No breaking changes to external APIs
- Internal task execution flow changes significantly
