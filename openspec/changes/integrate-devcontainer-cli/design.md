## Context

VibeRepo currently manages Docker containers using native Docker API operations via the `bollard` crate. This requires maintaining ~3,700 lines of complex code across three services (docker_service.rs, container_service.rs, timeout_watchdog.rs) that essentially duplicate functionality already provided by the DevContainer standard.

**Current State:**
- Direct Docker API operations via bollard crate (1,805 lines in docker_service.rs)
- Custom container lifecycle management (1,096 lines in container_service.rs)
- Custom timeout monitoring (275 lines in timeout_watchdog.rs)
- No support for standard devcontainer.json configuration
- Users cannot leverage DevContainer Features ecosystem
- High maintenance burden for Docker-related code
- No compatibility with VS Code Dev Containers or GitHub Codespaces

**Prototype Validation (2026-03-01):**
- ✅ All 5 test suites passed (100% success rate)
- ✅ ACP communication validated (51s agent install + 10s response)
- ✅ Container creation: 4-28s (4-30x better than target)
- ✅ Rust integration: 95% success rate
- ✅ Features installation: ~10 minutes (cached after first use)

**Constraints:**
- Must maintain ACP communication mechanism (docker exec + stdio)
- Must support existing workspace isolation per repository
- Must work in headless server environment
- Must be production-ready for automated Issue-to-PR workflows
- Database schema changes allowed (pre-1.0)
- Breaking changes to internal services acceptable (pre-1.0)

**Stakeholders:**
- Backend developers maintaining Docker integration code
- DevOps managing container deployments and configurations
- Users expecting reliable automated workflows
- Future users wanting to customize development environments

## Goals / Non-Goals

**Goals:**
- Replace native Docker API operations with devcontainer CLI wrapper
- Reduce Docker-related code by 81% (~3,700 lines → ~700 lines)
- Support standard devcontainer.json configuration per repository
- Enable users to leverage 200+ DevContainer Features
- Maintain full ACP communication compatibility (no changes to protocol)
- Achieve comparable or better container creation performance
- Provide VS Code Dev Containers and GitHub Codespaces compatibility
- Simplify maintenance burden for Docker operations

**Non-Goals:**
- Modify ACP communication protocol or implementation
- Support non-Docker container runtimes (Podman, etc.) in MVP
- Implement prebuild optimization in initial release
- Add WebSocket streaming for container logs
- Support interactive devcontainer.json configuration UI
- Backward compatibility with old Docker service API (breaking change acceptable)
- Multi-container devcontainer.json support (single container for MVP)

## Decisions

### Decision 1: Use @devcontainers/cli Instead of Native Docker API

**Choice:** Wrap `@devcontainers/cli` instead of using bollard crate for direct Docker API operations.

**Rationale:**
- Industry standard CLI maintained by Microsoft and community
- Supports full devcontainer.json specification
- Handles complex Docker operations (build, features, lifecycle hooks)
- Active development and bug fixes
- 200+ community Features available
- Reduces code from ~3,700 lines to ~700 lines (81% reduction)
- Proven reliability (used by VS Code, GitHub Codespaces, DevPod)

**Alternatives Considered:**
- **Keep bollard:** Full control but high maintenance burden, no standard support
- **DevPod SDK:** More features but heavier dependency, less mature
- **Custom Docker wrapper:** Reinventing the wheel, no ecosystem benefits

**Trade-offs:**
- ✅ 81% code reduction
- ✅ Standard devcontainer.json support
- ✅ 200+ Features ecosystem
- ✅ Active maintenance and community
- ❌ External dependency (requires Node.js)
- ❌ Additional CLI layer (debugging complexity)
- ❌ First-time Features build can be slow (~10 minutes)

### Decision 2: Runtime Agent Installation

**Choice:** Install agents (Bun + OpenCode) at runtime via docker exec after container creation, not in base image.

**Rationale:**
- Keeps base images small and generic
- Allows per-repository agent configuration
- Supports different agent versions per workspace
- No need to rebuild images when agent updates
- Prototype validated: 15-51s installation time (acceptable)
- Aligns with devcontainer.json lifecycle hooks pattern

**Alternatives Considered:**
- **Bake into image:** Faster startup but inflexible, larger images
- **DevContainer Feature:** Cleaner but requires maintaining custom Feature
- **Volume mount:** Complex, platform-specific issues

**Trade-offs:**
- ✅ Flexible agent configuration
- ✅ Smaller base images
- ✅ No image rebuilds for agent updates
- ✅ Per-repository customization
- ❌ 15-51s installation overhead per workspace
- ❌ Network dependency for agent downloads

### Decision 3: Preserve ACP Communication via docker exec

**Choice:** Keep existing ACP communication mechanism (docker exec + stdio) completely unchanged.

**Rationale:**
- Prototype validation confirmed compatibility
- No need to modify task_executor_service.rs
- Proven pattern already working in production
- Reduces integration risk significantly
- Clear separation of concerns (container management vs agent communication)

**Alternatives Considered:**
- **Modify ACP transport:** Unnecessary complexity, no benefits
- **Use devcontainer exec:** Same as docker exec, no advantage

**Trade-offs:**
- ✅ Zero changes to ACP protocol
- ✅ Zero changes to task executor
- ✅ Proven compatibility
- ✅ Reduced integration risk
- ❌ None (this is the optimal choice)

### Decision 4: JSON Output Parsing Strategy

**Choice:** Use `--log-format json` and parse the last line of output for result data.

**Rationale:**
- Prototype validated this approach works reliably
- Structured output easier to parse than text
- Last line contains the result JSON (containerId, etc.)
- Progress logs can be ignored or stored separately
- Standard pattern used by devcontainer CLI

**Alternatives Considered:**
- **Text parsing:** Fragile, error-prone, hard to maintain
- **Parse all lines:** Unnecessary complexity, performance overhead
- **Use exit code only:** Insufficient information

**Trade-offs:**
- ✅ Reliable structured parsing
- ✅ Easy to extract container ID and metadata
- ✅ Validated in prototype
- ❌ Requires handling multi-line output
- ❌ JSON parsing overhead (minimal)

### Decision 5: Configuration File Location

**Choice:** Support `.devcontainer/devcontainer.json` in repository root, fallback to default configuration.

**Rationale:**
- Standard location expected by VS Code and other tools
- Per-repository customization
- Graceful fallback for repositories without config
- Simple to document and explain to users
- Prototype validated this works correctly

**Alternatives Considered:**
- **Custom location:** Non-standard, confusing for users
- **Database storage:** Complex, loses VS Code compatibility
- **Require config:** Breaks existing repositories

**Trade-offs:**
- ✅ Standard location
- ✅ VS Code compatibility
- ✅ Graceful fallback
- ✅ Per-repository customization
- ❌ Requires file system access to check existence

### Decision 6: Error Handling Strategy

**Choice:** Capture stderr, parse JSON errors when available, provide user-friendly error messages.

**Rationale:**
- devcontainer CLI provides structured error output
- Users need actionable error messages
- Debugging requires detailed error information
- Prototype showed errors are well-formatted

**Alternatives Considered:**
- **Minimal error handling:** Poor user experience
- **Retry on all errors:** Wastes time on permanent failures
- **Silent failures:** Unacceptable for production

**Trade-offs:**
- ✅ User-friendly error messages
- ✅ Detailed debugging information
- ✅ Structured error parsing
- ❌ Additional error handling code
- ❌ Need to maintain error message mappings

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│              VibeRepo Backend (Rust)                        │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │   Workspace Service                                   │ │
│  │   • Orchestrates workspace lifecycle                 │ │
│  │   • Delegates to DevContainerService                 │ │
│  └───────────────┬───────────────────────────────────────┘ │
│                  │                                          │
│                  ▼                                          │
│  ┌───────────────────────────────────────────────────────┐ │
│  │   DevContainer Service (NEW)                          │ │
│  │   • Wraps @devcontainers/cli                          │ │
│  │   • create_workspace()                                │ │
│  │   • install_agent()                                   │ │
│  │   • remove_workspace()                                │ │
│  │   • check_devcontainer_exists()                       │ │
│  └───────────────┬───────────────────────────────────────┘ │
│                  │                                          │
│                  ▼                                          │
│  ┌───────────────────────────────────────────────────────┐ │
│  │   Task Executor Service (UNCHANGED)                   │ │
│  │   • Uses docker exec for ACP communication            │ │
│  │   • No changes required                               │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                      │
                      │ Shell commands
                      │
┌─────────────────────▼─────────────────────────────────────┐
│         @devcontainers/cli (Node.js)                      │
│         • devcontainer up                                 │
│         • devcontainer exec                               │
│         • Manages Docker containers                       │
└───────────────────────────────────────────────────────────┘
                      │
                      │ Docker API
                      │
┌─────────────────────▼─────────────────────────────────────┐
│              Docker Engine                                │
│              • Container runtime                          │
│              • Image management                           │
└───────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. Webhook triggers task creation
   │
   ▼
2. Workspace Service creates workspace
   │  workspace_service.create_workspace()
   │
   ▼
3. DevContainer Service checks for config
   │  devcontainer_service.check_devcontainer_exists()
   │  → Check .devcontainer/devcontainer.json
   │  → Fallback to default config if not found
   │
   ▼
4. DevContainer Service creates container
   │  devcontainer_service.create_workspace()
   │  → npx @devcontainers/cli up --workspace-folder ...
   │  → Parse JSON output (last line)
   │  → Extract container_id
   │
   ▼
5. DevContainer Service installs agent
   │  devcontainer_service.install_agent()
   │  → docker exec <container_id> bash -c "install script"
   │  → Install Bun (~15-30s)
   │  → Install OpenCode (~10-20s)
   │  → Verify installation
   │
   ▼
6. Task Executor executes task (UNCHANGED)
   │  task_executor_service.execute_task()
   │  → docker exec <container_id> opencode acp
   │  → ACP communication via stdin/stdout
   │  → Agent performs work
   │  → Create PR
   │
   ▼
7. Workspace Service cleans up
   │  workspace_service.delete_workspace()
   │  → devcontainer_service.remove_workspace()
   │  → docker rm -f <container_id>
```

### DevContainer Service API

```rust
// backend/src/services/devcontainer_service.rs

pub struct DevContainerService {
    cli_path: String,
    workspace_base_dir: PathBuf,
}

impl DevContainerService {
    /// Create a new workspace using devcontainer CLI
    pub async fn create_workspace(
        &self,
        workspace_id: &str,
        repo_path: &Path,
    ) -> Result<WorkspaceInfo> {
        // 1. Check for .devcontainer/devcontainer.json
        // 2. Run: npx @devcontainers/cli up --workspace-folder ...
        // 3. Parse JSON output (last line)
        // 4. Return WorkspaceInfo { container_id, ... }
    }
    
    /// Install agent (Bun + OpenCode) in container
    pub async fn install_agent(
        &self,
        container_id: &str,
        agent_config: &AgentConfig,
    ) -> Result<()> {
        // 1. Generate installation script
        // 2. Run: docker exec <container_id> bash -c "script"
        // 3. Verify installation
    }
    
    /// Remove workspace container
    pub async fn remove_workspace(
        &self,
        container_id: &str,
    ) -> Result<()> {
        // 1. Run: docker rm -f <container_id>
        // 2. Clean up any temporary files
    }
    
    /// Check if devcontainer.json exists in repository
    pub fn check_devcontainer_exists(
        &self,
        repo_path: &Path,
    ) -> bool {
        // Check .devcontainer/devcontainer.json
    }
}

#[derive(Debug)]
pub struct WorkspaceInfo {
    pub container_id: String,
    pub remote_user: Option<String>,
    pub remote_workspace_folder: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DevContainerOutput {
    #[serde(rename = "containerId")]
    container_id: String,
    #[serde(rename = "remoteUser")]
    remote_user: Option<String>,
    #[serde(rename = "remoteWorkspaceFolder")]
    remote_workspace_folder: Option<String>,
}
```

### Database Schema Changes

**No database schema changes required.** The existing schema fully supports this refactor:

```sql
-- Existing tables remain unchanged
-- repositories: Already has all needed fields
-- workspaces: Already has container_id field
-- agents: Already has configuration fields
-- tasks: Already has all needed fields for ACP communication
```

### Configuration Changes

```rust
// backend/src/config.rs

#[derive(Debug, Clone)]
pub struct Config {
    // ... existing fields ...
    
    /// Path to devcontainer CLI (default: "devcontainer")
    pub devcontainer_cli_path: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            // ... existing fields ...
            devcontainer_cli_path: env::var("DEVCONTAINER_CLI_PATH")
                .unwrap_or_else(|_| "devcontainer".to_string()),
        })
    }
}
```

### Default devcontainer.json

```json
{
  "name": "VibeRepo Workspace",
  "image": "ubuntu:22.04",
  "overrideCommand": true,
  "remoteUser": "root",
  "workspaceFolder": "/workspace",
  "customizations": {
    "vscode": {
      "extensions": []
    }
  }
}
```

## Risks / Trade-offs

### Risk 1: External Dependency on @devcontainers/cli

**Risk:** Requires Node.js and @devcontainers/cli to be installed on host system.

**Mitigation:**
- Document installation requirements clearly
- Provide installation script for common platforms
- Check for CLI availability at startup
- Fail fast with helpful error message if not found
- Consider bundling CLI in Docker image for self-contained deployment

**Severity:** Medium  
**Likelihood:** Low (easy to install)

### Risk 2: First-time Features Build Performance

**Risk:** First-time devcontainer.json with Features can take ~10 minutes to build.

**Mitigation:**
- Document expected build times
- Implement prebuild support in future iteration
- Cache built images aggressively
- Provide progress feedback to users
- Recommend minimal Features for faster builds

**Severity:** Medium  
**Likelihood:** High (affects all first-time users)

### Risk 3: Debugging Complexity

**Risk:** Additional CLI layer makes debugging harder when issues occur.

**Mitigation:**
- Capture and log all CLI output (stdout + stderr)
- Provide detailed error messages with context
- Document common issues and solutions
- Add verbose logging mode for troubleshooting
- Keep old Docker service code for comparison during transition

**Severity:** Low  
**Likelihood:** Medium

### Risk 4: CLI Breaking Changes

**Risk:** @devcontainers/cli updates could introduce breaking changes.

**Mitigation:**
- Pin to specific CLI version in documentation
- Test with multiple CLI versions
- Monitor CLI releases and changelogs
- Abstract CLI interface behind trait for easier updates
- Maintain compatibility layer if needed

**Severity:** Low  
**Likelihood:** Low (stable API)

### Risk 5: JSON Parsing Failures

**Risk:** CLI output format changes could break parsing logic.

**Mitigation:**
- Use structured JSON output (--log-format json)
- Validate JSON schema before parsing
- Provide fallback parsing strategies
- Log raw output for debugging
- Test with multiple CLI versions

**Severity:** Low  
**Likelihood:** Low (JSON format is stable)

### Risk 6: Agent Installation Failures

**Risk:** Network issues or script errors could fail agent installation.

**Mitigation:**
- Implement retry logic with exponential backoff
- Validate installation after completion
- Provide detailed error messages
- Support offline installation (pre-downloaded binaries)
- Cache agent binaries for faster reinstallation

**Severity:** Medium  
**Likelihood:** Medium (network-dependent)

## Migration Plan

### Phase 1: Implementation (Days 1-3)

**Day 1: DevContainer Service Foundation**
- Create `devcontainer_service.rs` module
- Implement `create_workspace()` with basic CLI wrapping
- Implement `check_devcontainer_exists()`
- Add configuration for CLI path
- Write unit tests for CLI command generation

**Day 2: Agent Installation**
- Implement `install_agent()` method
- Create agent installation scripts (Bun + OpenCode)
- Add verification logic
- Write unit tests for installation flow

**Day 3: Integration**
- Update `workspace_service.rs` to use DevContainerService
- Remove dependencies on docker_service.rs
- Update error handling
- Write integration tests

### Phase 2: Testing (Days 4-5)

**Day 4: Comprehensive Testing**
- Test with various devcontainer.json configurations
- Test with and without Features
- Test agent installation and verification
- Test error scenarios (missing CLI, invalid config, etc.)
- Performance benchmarking

**Day 5: End-to-End Validation**
- Test complete Issue-to-PR workflow
- Test with real repositories
- Test ACP communication (should be unchanged)
- Load testing with concurrent workspaces
- Document test results

### Phase 3: Cleanup (Days 6-7)

**Day 6: Code Removal**
- Remove `docker_service.rs` (1,805 lines)
- Remove `container_service.rs` (1,096 lines)
- Remove `timeout_watchdog.rs` (275 lines)
- Remove bollard dependency from Cargo.toml
- Update imports and references

**Day 7: Documentation**
- Update API documentation
- Create devcontainer.json guide for users
- Document migration from old system
- Add troubleshooting guide
- Update deployment documentation

### Phase 4: Deployment (Days 8-10)

**Day 8: Staging Deployment**
- Deploy to staging environment
- Install @devcontainers/cli on staging servers
- Run smoke tests
- Monitor metrics (startup time, success rate)

**Day 9: Production Rollout**
- Deploy to production with feature flag
- Gradual rollout (10% → 50% → 100%)
- Monitor error rates and performance
- Collect user feedback

**Day 10: Stabilization**
- Address any issues found in production
- Fine-tune configuration
- Update documentation based on feedback
- Remove feature flag once stable

### Rollback Strategy

**If critical issues are found:**

1. **Immediate Rollback (< 1 hour)**
   - Revert to previous deployment
   - No data loss (database unchanged)
   - Existing workspaces continue working

2. **Code Rollback (< 4 hours)**
   - Restore docker_service.rs, container_service.rs, timeout_watchdog.rs from git
   - Restore bollard dependency
   - Rebuild and redeploy

3. **Partial Rollback (< 1 day)**
   - Keep DevContainerService for new workspaces
   - Use old services for existing workspaces
   - Gradual migration over time

**Rollback Triggers:**
- Success rate drops below 95%
- Container creation time exceeds 120s consistently
- Critical bugs affecting production workflows
- ACP communication failures

## Open Questions

### 1. CLI Installation Strategy

**Question:** Should we bundle @devcontainers/cli in Docker image or require host installation?

**Options:**
- A) Require host installation (current plan)
- B) Bundle in Docker image (self-contained)
- C) Support both modes

**Recommendation:** Start with A (host installation) for simplicity, add B (bundled) if deployment complexity becomes an issue.

---

### 2. Features Build Optimization

**Question:** Should we implement prebuild support in initial release or defer to future?

**Options:**
- A) Implement prebuild in MVP (more work, better UX)
- B) Defer to post-MVP (faster release, slower first-time builds)
- C) Provide prebuild as optional feature

**Recommendation:** B (defer to post-MVP). Document expected build times and recommend minimal Features for MVP.

---

### 3. Configuration Validation

**Question:** Should we validate devcontainer.json syntax before running CLI?

**Options:**
- A) Validate using JSON schema (catches errors early)
- B) Let CLI validate (simpler, but errors come later)
- C) Validate only critical fields

**Recommendation:** C (validate critical fields). Check for required fields and common mistakes, let CLI handle full validation.

---

### 4. Agent Installation Caching

**Question:** Should we cache agent binaries to speed up installation?

**Options:**
- A) Cache on host filesystem (faster, requires disk space)
- B) No caching (simpler, slower)
- C) Optional caching via configuration

**Recommendation:** C (optional caching). Add `AGENT_CACHE_DIR` environment variable, default to no caching for MVP.

---

### 5. Multi-container Support

**Question:** Should we support docker-compose.yml in devcontainer.json?

**Options:**
- A) Support in MVP (more complex, more flexible)
- B) Defer to post-MVP (simpler, single container only)
- C) Never support (keep it simple)

**Recommendation:** B (defer to post-MVP). Single container is sufficient for most use cases, add multi-container support based on user demand.

---

### 6. Error Recovery

**Question:** How should we handle partial failures (container created but agent installation failed)?

**Options:**
- A) Clean up container and fail (clean state)
- B) Keep container for debugging (helpful but clutters)
- C) Retry installation automatically (may succeed on retry)

**Recommendation:** A (clean up and fail) for MVP. Add C (retry) in future iteration with exponential backoff.

---

### 7. Performance Monitoring

**Question:** Should we add metrics for container creation and agent installation times?

**Options:**
- A) Add detailed metrics (better visibility, more code)
- B) Log only (simpler, less visibility)
- C) Add metrics in post-MVP

**Recommendation:** B (log only) for MVP. Add structured logging with timing information, implement metrics in post-MVP.

---

### 8. Backward Compatibility

**Question:** Should we support both old and new systems during transition?

**Options:**
- A) Hard cutover (simpler, riskier)
- B) Feature flag (safer, more complex)
- C) Gradual migration (safest, most complex)

**Recommendation:** B (feature flag). Add `USE_DEVCONTAINER=true/false` environment variable for safe rollback.
