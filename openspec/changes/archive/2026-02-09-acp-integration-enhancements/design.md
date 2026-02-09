# Design: ACP Integration Enhancements

## Context

This change addresses critical gaps and improvements identified during an ACP protocol audit of the initial `acp-integration` implementation. All enhancements have been implemented and tested on 2026-02-09.

## Key Decisions

### Decision 1: Integrate Existing Permission System

**Choice:** Use existing `PermissionPolicy` system rather than building new permission logic.

**Rationale:**
- Permission policy already exists and is well-tested
- Avoids code duplication
- Maintains consistency with existing security model

**Implementation:**
- Add `permission_policy` field to `VibeRepoClient`
- Map ACP tool calls to internal `PermissionRequest` types
- Return proper ACP protocol responses

### Decision 2: Two-Phase Shutdown Strategy

**Choice:** Attempt graceful shutdown before force kill.

**Rationale:**
- Gives agents opportunity to clean up resources
- Reduces risk of corrupted state
- Improves user experience
- Fallback ensures reliability

**Implementation:**
- Phase 1: Send `session/cancel` + `/exit` command, wait 500ms
- Phase 2: Force SIGKILL if still running
- Comprehensive logging for debugging

### Decision 3: File-Based MCP Configuration

**Choice:** Use JSON configuration files instead of database storage.

**Rationale:**
- Version control friendly (can commit to repository)
- Easy to edit and review
- No database migrations needed
- Supports repository-specific customization

**Configuration Priority:**
1. Repository level: `{workspace}/.vibe-repo/mcp-servers.json`
2. Global level: `./data/vibe-repo/config/mcp-servers.json`
3. Default: Empty (no MCP servers)

### Decision 4: Environment Variable Substitution

**Choice:** Support `${VAR}` syntax for sensitive values.

**Rationale:**
- Keeps secrets out of configuration files
- Follows 12-factor app principles
- Flexible deployment across environments

**Implementation:**
- Regex-based substitution: `\$\{([A-Z_][A-Z0-9_]*)\}`
- Error if variable not found
- Substitution happens at load time

### Decision 5: Complete Event Tracking

**Choice:** Handle all SessionUpdate types, not just core ones.

**Rationale:**
- Better observability and debugging
- Complete audit trail
- Future-proof for new update types

**Implementation:**
- Add handlers for all update types
- Store relevant events in database
- Use appropriate log levels (info/debug)

## Architecture Changes

### Before (Original ACP Integration)

```
AcpClient
├── request_permission() → method_not_found ❌
├── ClientCapabilities::default() ❌
├── shutdown() → immediate SIGKILL ❌
└── session_notification() → partial handlers ❌
```

### After (With Enhancements)

```
AcpClient
├── request_permission() → PermissionPolicy evaluation ✅
├── ClientCapabilities::new() with explicit values ✅
├── shutdown() → graceful + force fallback ✅
├── session_notification() → complete handlers ✅
└── load_mcp_servers() → MCP configuration ✅

McpConfigLoader (New)
├── load_for_workspace() → priority-based loading
├── substitute_env_vars() → ${VAR} replacement
└── validate_and_process() → validation
```

## Implementation Summary

### Files Modified
- `backend/src/services/acp/client.rs` - Core enhancements
- `backend/src/services/acp/events.rs` - Event parsing updates
- `backend/src/services/agent_manager.rs` - MCP integration

### Files Added
- `backend/src/config/mcp.rs` - MCP configuration module (400+ lines)
- `docs/api/mcp-integration.md` - Complete MCP guide
- `data/vibe-repo/config/mcp-servers.json.example` - Example config
- `docs/examples/mcp-servers.json` - Repository example

### Testing
- 10+ new unit tests for MCP configuration
- All existing tests pass (280+ tests)
- Build verification successful
- No clippy warnings

## Trade-offs

### Permission Request Handler
- ✅ Fixes critical security issue
- ✅ Proper protocol compliance
- ❌ Adds complexity to ACP client

### Graceful Shutdown
- ✅ Better resource cleanup
- ✅ Reduced forced terminations
- ❌ Adds 500ms delay to shutdown
- ❌ `/exit` is OpenCode-specific (not ACP standard)

### MCP File Configuration
- ✅ Version control friendly
- ✅ Easy to edit
- ✅ No database changes
- ❌ Requires file system access
- ❌ No UI for configuration (yet)

### Complete Event Tracking
- ✅ Better observability
- ✅ Complete audit trail
- ❌ Slightly more database storage
- ❌ More event processing overhead

## Success Metrics

All metrics achieved:

- ✅ Zero permission requests returning `method_not_found`
- ✅ 100% of permission requests logged
- ✅ Graceful shutdown implemented
- ✅ All SessionUpdate types handled
- ✅ MCP servers configurable
- ✅ All tests passing (280+ unit tests)
- ✅ Zero build warnings

## Future Considerations

### Potential Improvements
1. **UI for MCP Configuration** - Web interface for managing MCP servers
2. **MCP Server Marketplace** - Curated list of useful MCP servers
3. **Permission Request UI** - Interactive approval for sensitive operations
4. **Configurable Shutdown Timeout** - Allow customization of 500ms wait
5. **MCP Server Health Monitoring** - Track MCP server availability

### Not Planned
- Multi-agent orchestration (deferred to future)
- Real-time WebSocket streaming (deferred to future)
- Custom ACP protocol extensions (stick to standard)

## Completion Status

**Status:** ✅ **COMPLETED**  
**Date:** 2026-02-09  
**Total Tasks:** 100  
**All Tests:** ✅ Passing  
**Documentation:** ✅ Complete  
**Ready for:** Production deployment
