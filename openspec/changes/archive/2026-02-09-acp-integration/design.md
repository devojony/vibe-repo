## Context

VibeRepo currently executes tasks using `docker exec` to run CLI commands inside workspace containers. This approach has several limitations:

**Current State:**
- Task execution is a black box - no visibility into agent progress
- Output parsing is fragile and error-prone
- No structured communication with agents
- No permission control over agent actions
- No way to cancel or pause running tasks
- CLI startup overhead (~100-200ms with Node.js)

**Constraints:**
- Must maintain Docker workspace isolation
- Must support multiple AI agents (OpenCode, Claude Code, etc.)
- Must work in headless server environment
- Must be production-ready for automated workflows
- Database schema changes allowed (pre-1.0)

**Stakeholders:**
- Backend developers implementing task execution
- DevOps managing Docker images and deployments
- Users expecting reliable Issue-to-PR automation

## Goals / Non-Goals

**Goals:**
- Implement ACP client in Rust for structured agent communication
- Achieve real-time visibility into agent progress (plans, tool calls, messages)
- Enable permission control for agent file operations and commands
- Support multiple ACP-compatible agents (OpenCode, Claude Code)
- Reduce agent startup time by 10x (Node.js → Bun)
- Maintain Docker workspace isolation per repository
- Provide foundation for future multi-agent support

**Non-Goals:**
- Multi-agent orchestration (single agent per task for MVP)
- Interactive user approval for permissions (auto-approve based on policy)
- WebSocket streaming to frontend (store events in DB for now)
- Support for non-ACP agents (focus on ACP standard)
- Backward compatibility with old CLI approach (breaking change acceptable)

## Decisions

### Decision 1: Use ACP Protocol

**Choice:** Adopt Agent Client Protocol (ACP) as the standard interface for agent communication.

**Rationale:**
- Industry standard protocol (supported by Zed, JetBrains, Neovim, Emacs)
- Structured JSON-RPC communication over stdin/stdout
- Real-time streaming of events (plans, tool calls, messages)
- Built-in permission system
- Future-proof (growing ecosystem)

**Alternatives Considered:**
- **Custom protocol:** More control but reinventing the wheel, no ecosystem
- **Direct API calls:** Tighter coupling, no standard interface
- **MCP only:** Model Context Protocol is for tools, not agent orchestration

**Trade-offs:**
- ✅ Standardization and ecosystem
- ✅ Real-time structured communication
- ❌ Learning curve for new protocol
- ❌ Dependency on protocol stability

### Decision 2: Use Bun Instead of Node.js

**Choice:** Use Bun as the JavaScript runtime for OpenCode and other JS-based agents.

**Rationale:**
- 10x faster startup (~10-20ms vs ~100-200ms)
- 40% less memory usage (~30-50MB vs ~50-100MB)
- Drop-in replacement for Node.js (npm compatibility)
- Single binary installation (~90MB)
- Better performance for JSON parsing and I/O

**Alternatives Considered:**
- **Node.js:** Standard choice but slower startup
- **Deno:** Good performance but less npm compatibility
- **Native Rust agent:** Best performance but requires building custom agent

**Trade-offs:**
- ✅ Significantly faster startup
- ✅ Lower memory footprint
- ✅ npm compatibility
- ❌ Slightly larger binary (~90MB vs ~50MB for Node.js)
- ❌ Less mature than Node.js (but stable enough)

### Decision 3: OpenCode as Primary Agent

**Choice:** Use OpenCode as the default AI agent with native ACP support.

**Rationale:**
- Native ACP support (`opencode acp` command)
- Supports multiple LLM providers (OpenAI, Anthropic, Gemini, etc.)
- Open source and actively maintained (99.5k stars)
- Optimized for code execution tasks
- Strong community and documentation

**Alternatives Considered:**
- **Claude Code via adapter:** Official Anthropic agent but requires adapter layer
- **Custom agent:** Full control but significant development effort
- **Aider:** Popular but no ACP support yet

**Trade-offs:**
- ✅ Native ACP support (no adapter needed)
- ✅ Multi-provider flexibility
- ✅ Active development
- ❌ Different feature set than Claude Code
- ❌ Requires API keys for LLM providers

### Decision 4: Subprocess Communication Pattern

**Choice:** Spawn agents as subprocesses and communicate via JSON-RPC over stdin/stdout.

**Rationale:**
- Standard ACP transport mechanism
- Process isolation (agent crashes don't affect VibeRepo)
- Simple and reliable (no network overhead)
- Easy to implement with Tokio
- Proven pattern (used by Zed, Emacs, Neovim)

**Alternatives Considered:**
- **HTTP API:** More overhead, requires port management
- **WebSocket:** Overkill for local communication
- **Shared memory:** Complex, platform-specific

**Trade-offs:**
- ✅ Simple and reliable
- ✅ Process isolation
- ✅ Standard pattern
- ❌ No built-in reconnection (need to respawn on crash)
- ❌ Buffering considerations for large messages

### Decision 5: Event Storage Strategy

**Choice:** Store agent events (plans, tool calls) as JSONB in the tasks table.

**Rationale:**
- Simple schema (no new tables needed)
- PostgreSQL JSONB is efficient and queryable
- Easy to add more event types later
- Sufficient for MVP (no real-time streaming to frontend yet)

**Alternatives Considered:**
- **Separate events table:** More normalized but overkill for MVP
- **Time-series database:** Better for analytics but adds complexity
- **In-memory only:** Fast but loses history on restart

**Trade-offs:**
- ✅ Simple implementation
- ✅ Queryable with PostgreSQL JSON operators
- ✅ No new tables
- ❌ Large JSONB fields if many events
- ❌ Not optimized for real-time streaming (future work)

### Decision 6: Permission Management

**Choice:** Auto-approve permissions based on configurable policy, log all actions.

**Rationale:**
- Headless environment (no user to prompt)
- Automated workflows require non-interactive execution
- Policy-based control (e.g., allow read, allow write to workspace, deny delete)
- Audit trail via logging

**Alternatives Considered:**
- **Always allow:** Simpler but less secure
- **Always deny:** Breaks agent functionality
- **User approval:** Not feasible in automated environment

**Trade-offs:**
- ✅ Non-interactive automation
- ✅ Policy-based control
- ✅ Audit trail
- ❌ Less granular than interactive approval
- ❌ Requires careful policy configuration

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────┐
│         VibeRepo Backend (Rust)                     │
│                                                     │
│  ┌───────────────────────────────────────────────┐ │
│  │   Task Executor Service                       │ │
│  │   • Orchestrates task execution               │ │
│  │   • Manages agent lifecycle                   │ │
│  └───────────────┬───────────────────────────────┘ │
│                  │                                  │
│                  ▼                                  │
│  ┌───────────────────────────────────────────────┐ │
│  │   Agent Manager Service (NEW)                 │ │
│  │   • Spawns agent subprocesses                 │ │
│  │   • Manages JSON-RPC communication            │ │
│  │   • Handles permission requests               │ │
│  │   • Streams events to database                │ │
│  └───────────────┬───────────────────────────────┘ │
│                  │                                  │
│                  ▼                                  │
│  ┌───────────────────────────────────────────────┐ │
│  │   ACP Client (NEW)                            │ │
│  │   • JSON-RPC over stdin/stdout                │ │
│  │   • Initialize, create session, send prompts  │ │
│  │   • Handle streaming responses                │ │
│  └───────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
                      │
                      │ stdin/stdout (NDJSON)
                      │
┌─────────────────────▼─────────────────────────────┐
│      Agent Process (Bun + OpenCode)               │
│      • opencode acp                               │
│      • Runs in workspace container                │
│      • Executes tools (filesystem, shell, etc.)   │
└───────────────────────────────────────────────────┘
```

### Data Flow

```
1. Webhook triggers task creation
   │
   ▼
2. Task Executor picks up task
   │
   ▼
3. Agent Manager spawns agent subprocess
   │  bun opencode acp
   │
   ▼
4. ACP Client initializes session
   │  → initialize (capabilities)
   │  ← initialize response
   │  → newSession (working_dir)
   │  ← session_id
   │
   ▼
5. ACP Client sends prompt
   │  → prompt (session_id, issue description)
   │
   ▼
6. Agent streams events
   │  ← sessionUpdate (plan)
   │  ← sessionUpdate (tool_call: read_file)
   │  ← requestPermission (write_file)
   │  → permissionResponse (allow)
   │  ← sessionUpdate (tool_call: write_file)
   │  ← sessionUpdate (message: "Fixed the bug")
   │  ← sessionUpdate (completed)
   │
   ▼
7. Agent Manager stores events in DB
   │  UPDATE tasks SET plans = ..., events = ...
   │
   ▼
8. Task Executor creates PR
   │  git push origin feature/issue-123
   │  GitHub API: create PR
   │
   ▼
9. Task marked as completed
```

### Database Schema Changes

```sql
-- Add new fields to tasks table
ALTER TABLE tasks
ADD COLUMN plans JSONB,
ADD COLUMN events JSONB;

-- Example data structure
{
  "plans": [
    {
      "step": "Analyze issue",
      "status": "completed",
      "timestamp": "2026-02-07T10:00:00Z"
    },
    {
      "step": "Fix bug in login.rs",
      "status": "in_progress",
      "timestamp": "2026-02-07T10:01:00Z"
    }
  ],
  "events": [
    {
      "type": "tool_call",
      "tool": "read_file",
      "args": {"path": "src/login.rs"},
      "timestamp": "2026-02-07T10:00:30Z"
    },
    {
      "type": "tool_call",
      "tool": "write_file",
      "args": {"path": "src/login.rs", "content": "..."},
      "timestamp": "2026-02-07T10:01:15Z"
    }
  ]
}
```

## Risks / Trade-offs

### Risk 1: ACP Protocol Stability
**Risk:** ACP is relatively new (2025), protocol changes could break integration.

**Mitigation:**
- Pin to specific ACP version in documentation
- Monitor ACP GitHub for breaking changes
- Abstract ACP client behind trait for easier updates
- Test with multiple ACP-compatible agents

### Risk 2: Bun Maturity
**Risk:** Bun is less mature than Node.js, potential bugs or compatibility issues.

**Mitigation:**
- Bun 1.0+ is stable for production use
- OpenCode officially supports Bun
- Fallback to Node.js if critical issues found
- Monitor Bun releases and community feedback

### Risk 3: Agent Process Crashes
**Risk:** Agent subprocess crashes could leave tasks in inconsistent state.

**Mitigation:**
- Implement process health monitoring
- Auto-restart on crash with exponential backoff
- Store partial progress in database
- Set task to failed state if max retries exceeded

### Risk 4: Large Event Data
**Risk:** JSONB fields could grow large with many events, impacting database performance.

**Mitigation:**
- Limit event storage to last N events per task
- Implement event compaction/summarization
- Consider separate events table if becomes issue
- Monitor database performance metrics

### Risk 5: Permission Policy Misconfiguration
**Risk:** Overly permissive policy could allow destructive operations.

**Mitigation:**
- Default to restrictive policy (read-only + workspace writes)
- Explicitly deny dangerous operations (rm -rf, etc.)
- Log all permission requests and decisions
- Regular audit of permission logs

### Risk 6: Startup Time Regression
**Risk:** Subprocess spawning overhead could negate Bun's startup benefits.

**Mitigation:**
- Measure end-to-end startup time in tests
- Consider process pooling if needed
- Optimize subprocess creation (reuse containers)
- Profile and optimize hot paths

## Migration Plan

### Phase 1: Development (Week 1-2)
1. Implement ACP client module with JSON-RPC communication
2. Create Agent Manager service with subprocess management
3. Add OpenCode integration with Bun runtime
4. Update database schema (add plans and events fields)
5. Write comprehensive unit tests

### Phase 2: Integration (Week 2-3)
1. Refactor Task Executor to use Agent Manager
2. Implement permission policy system
3. Add event storage and retrieval
4. Update Docker images with Bun and OpenCode
5. Write integration tests

### Phase 3: Testing (Week 3-4)
1. End-to-end testing with real repositories
2. Performance benchmarking (startup time, memory usage)
3. Error handling and edge case testing
4. Load testing with concurrent tasks
5. Security audit of permission system

### Phase 4: Deployment (Week 4)
1. Deploy to staging environment
2. Monitor metrics (startup time, success rate, errors)
3. Gradual rollout to production
4. Monitor and iterate based on feedback

### Rollback Strategy
- Keep old CLI execution code in separate module (deprecated)
- Feature flag to switch between old and new execution
- Database schema changes are additive (no data loss)
- Can rollback Docker images to previous version
- Rollback plan: disable feature flag, redeploy old version

## Open Questions

1. **Event Retention Policy:** How long should we keep event history? Should we implement automatic cleanup?
   - **Proposal:** Keep last 100 events per task, implement cleanup job

2. **Multi-Agent Support:** Should we design for multiple agents per task now or defer to future?
   - **Proposal:** Single agent for MVP, design interfaces to support multiple agents later

3. **Real-time Streaming:** Should we add WebSocket support for real-time event streaming to frontend?
   - **Proposal:** Defer to post-MVP, store in DB for now

4. **Agent Selection:** Should users be able to choose which agent to use per repository?
   - **Proposal:** Yes, add agent_type field to repositories table

5. **Cost Monitoring:** Should we track LLM API costs per task?
   - **Proposal:** Add cost tracking in future iteration, not MVP

6. **Timeout Handling:** What should happen when agent exceeds timeout?
   - **Proposal:** Kill process, mark task as failed, store partial progress
