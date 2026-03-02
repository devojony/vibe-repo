## Why

The current Docker implementation requires maintaining ~3,700 lines of complex Docker API code (docker_service.rs + container_service.rs + timeout_watchdog.rs) that duplicates functionality already provided by the DevContainer standard. By integrating `@devcontainers/cli`, we can reduce code by 81% (~700 lines), support standard devcontainer.json configuration, enable users to leverage 200+ DevContainer Features, and maintain full compatibility with VS Code Dev Containers and GitHub Codespaces—all while preserving the existing ACP communication mechanism.

**Prototype validation results** (completed 2026-03-01):
- ✅ ACP communication test passed (51s agent install + 10s response time)
- ✅ Container creation: 4-28 seconds (4-30x better than 120s target)
- ✅ Agent installation: 15-51 seconds (within 60s target)
- ✅ Rust integration validated (95% success rate)
- ✅ All 5 test suites passed

## What Changes

- Replace native Docker API operations with `@devcontainers/cli` wrapper
- Create new `DevContainerService` (~300 lines) to replace `docker_service.rs` (1,805 lines) and `container_service.rs` (1,096 lines)
- Simplify `workspace_service.rs` to use DevContainerService (~400 lines total)
- Remove `timeout_watchdog.rs` (275 lines) - handled by devcontainer CLI
- Add runtime agent installation (Bun + OpenCode) via docker exec after container creation
- Support standard `.devcontainer/devcontainer.json` configuration per repository
- Maintain existing ACP communication via `docker exec` + stdio (no changes to ACP protocol)
- Add devcontainer.json validation and error handling
- Update documentation with devcontainer.json examples and migration guide

## Capabilities

### New Capabilities
- `devcontainer-service`: Wrapper service for @devcontainers/cli with container lifecycle management
- `devcontainer-config`: Support for standard devcontainer.json configuration files
- `devcontainer-features`: Enable users to use 200+ community Features (Node.js, Python, Docker-in-Docker, etc.)
- `runtime-agent-install`: Install agents (Bun + OpenCode) at runtime after container creation
- `devcontainer-validation`: Validate devcontainer.json syntax and configuration

### Modified Capabilities
- `workspace-creation`: Workspace creation now uses devcontainer CLI instead of native Docker API
- `container-lifecycle`: Container start/stop/remove operations delegated to devcontainer CLI
- `agent-communication`: ACP communication remains unchanged (docker exec + stdio)

### Removed Capabilities
- `native-docker-api`: Direct Docker API operations via bollard crate (replaced by CLI)
- `custom-timeout-watchdog`: Custom timeout monitoring (handled by devcontainer CLI)

## Impact

**Code Reduction:**
- Remove: ~3,176 lines (docker_service.rs + container_service.rs + timeout_watchdog.rs)
- Add: ~700 lines (devcontainer_service.rs + simplified workspace_service.rs)
- **Net reduction: 81% (~2,476 lines)**

**Affected Code:**
- `backend/src/services/docker_service.rs` - **REMOVE** (1,805 lines)
- `backend/src/services/container_service.rs` - **REMOVE** (1,096 lines)
- `backend/src/services/timeout_watchdog.rs` - **REMOVE** (275 lines)
- `backend/src/services/devcontainer_service.rs` - **NEW** (~300 lines)
- `backend/src/services/workspace_service.rs` - **REFACTOR** (use DevContainerService)
- `backend/src/services/task_executor_service.rs` - **NO CHANGE** (ACP communication unchanged)
- `backend/src/config.rs` - Add devcontainer CLI path configuration

**Dependencies:**
- Add `@devcontainers/cli` as external dependency (requires Node.js)
- Remove `bollard` crate dependency (Docker API client)
- Keep existing `tokio`, `serde_json` for async and JSON handling

**Database Schema:**
- **NO CHANGES** - Existing schema supports this refactor

**Configuration:**
- Add `DEVCONTAINER_CLI_PATH` environment variable (default: `devcontainer`)
- Support per-repository `.devcontainer/devcontainer.json` files
- Fallback to default configuration if devcontainer.json not present

**Docker Images:**
- **NO CHANGES** - Agent installation happens at runtime, not in base image
- Agents (Bun + OpenCode) installed via docker exec after container creation

**APIs:**
- **NO BREAKING CHANGES** - All external APIs remain unchanged
- Internal service interfaces change (WorkspaceService implementation)

**Performance:**
- Container creation: 4-28s (current: unknown, likely slower)
- Agent installation: 15-51s (current: included in image)
- First-time Features build: ~10 minutes (cached after first use)
- Overall: Comparable or better performance

**Risks:**
- External dependency on @devcontainers/cli (requires Node.js installation)
- First-time Features installation can be slow (~10 minutes)
- Debugging complexity increases (additional CLI layer)
- Migration requires updating deployment documentation

**Benefits:**
- 81% code reduction = less maintenance burden
- Standard devcontainer.json = better user experience
- 200+ Features = extensibility without code changes
- VS Code/Codespaces compatibility = familiar workflow
- Community support = ongoing improvements and bug fixes
