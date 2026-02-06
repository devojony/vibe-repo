# 简化 MVP 技术设计

## 🚨 关键说明：Git Provider 核心模块必须保留

**本次简化的核心原则：**
- ✅ 删除管理功能（API 端点、配置界面）
- ✅ 删除辅助功能（轮询、重试、清理）
- ❌ 不删除核心业务逻辑（PR 创建、Issue 关闭）

**特别注意：**
- `src/git_provider/` 核心模块（4,111 行）**必须保留**
- 只删除 `src/api/settings/providers/` API 层（727 行）
- Git Provider 是 PR 创建和 Issue 关闭的核心依赖
- 删除将导致系统失去核心价值

---

## Context

VibeRepo 当前实现包含 15,000+ 行代码，10 个数据库表，6 个后台服务，50+ 个 API 端点。经过评估，发现存在明显的过度设计：

**当前架构复杂度：**
- 双模式 Issue 检测（Webhook + Polling）
- 复杂的失败分析系统（9 种失败类型）
- 完整的执行历史追踪（独立表 + 文件存储）
- 多 Agent 管理（每个 workspace 可配置多个 agent）
- Init Scripts 功能（自定义初始化脚本）
- WebSocket 实时日志流
- 多个清理和重试服务

**核心价值：**
实际上，用户只需要一个简单的流程：Issue → Agent 执行 → PR 创建。当前 95% 的复杂度服务于 5% 的核心价值。

**约束条件：**
- 必须保持核心功能完整（Issue 到 PR 的自动化）
- 必须保持 Docker 容器隔离
- 必须支持 GitHub/Gitea webhook
- 代码必须保持可测试性

**利益相关者：**
- 开发团队：需要更快的迭代速度
- 早期用户：需要稳定可靠的核心功能
- 维护者：需要更简单的代码库

## Goals / Non-Goals

**Goals:**
- 将代码量从 15,000+ 行减少到 6,000 行左右（60% 减少）
- 将数据库表从 10 个减少到 4 个核心表
- 将后台服务从 6 个减少到 1 个
- 将 API 端点从 50+ 个减少到 8 个核心端点
- 保持核心功能完整：Webhook → Task → Docker 执行 → PR 创建
- 保持代码可测试性（至少 200 个核心测试）
- 简化部署和维护复杂度

**Non-Goals:**
- 不追求功能完整性（删除非核心功能是目标）
- 不保持向后兼容（这是 BREAKING 变更）
- 不优化性能（简化优先，性能次要）
- 不添加新功能（纯粹的简化重构）
- 不支持数据迁移（建议全新部署）

## Decisions

### Decision 1: 只保留 Webhook，删除 Polling

**选择：** 删除 Issue Polling Service，只保留 Webhook 作为 Issue 检测方式

**理由：**
- Webhook 是实时的，Polling 是备用方案
- MVP 阶段不需要备用方案
- 删除 Polling 可以减少 ~2000 行代码和 1 个后台服务
- 如果 Webhook 失败，用户可以手动创建 Task

**替代方案考虑：**
- 保留 Polling 删除 Webhook：Polling 延迟高（5 分钟），用户体验差
- 两者都保留：违背简化目标

**影响：**
- 如果 Git Provider 不支持 Webhook，用户需要手动创建任务
- 删除 `issue_polling_service.rs` 和相关配置

### Decision 2: 删除 WebSocket，使用 REST 轮询

**选择：** 删除 WebSocket 实时日志流，改用 REST API 轮询查询日志

**理由：**
- WebSocket 增加了约 600 行代码和复杂的订阅管理
- MVP 阶段，用户可以接受轮询（每 2-5 秒查询一次）
- 简化了客户端实现（不需要 WebSocket 库）
- 减少了服务器资源消耗（不需要维护长连接）

**替代方案考虑：**
- 保留 WebSocket：增加复杂度，MVP 不需要
- 使用 Server-Sent Events (SSE)：仍然需要长连接管理

**影响：**
- 删除 `task_log_broadcaster.rs` 和 WebSocket 路由
- 日志查询有 2-5 秒延迟（可接受）
- 客户端需要实现轮询逻辑

### Decision 3: 删除 Init Scripts，使用预构建镜像

**选择：** 删除 Init Scripts 功能，使用预构建的 Docker 镜像

**理由：**
- Init Scripts 增加了 ~800 行代码和 1 个数据库表
- 预构建镜像更快（不需要每次运行脚本）
- 预构建镜像更可靠（脚本可能失败）
- 用户可以自己构建自定义镜像

**替代方案考虑：**
- 保留 Init Scripts：增加复杂度和失败点
- 使用 Dockerfile：需要构建时间，不如预构建

**影响：**
- 删除 `init_scripts` 表和 `init_script_service.rs`
- 用户需要使用预构建镜像或自己构建镜像
- 启动速度更快（不需要运行初始化脚本）

### Decision 4: 删除失败分析，只记录错误信息

**选择：** 删除智能失败分析系统（9 种失败类型），只在任务中记录错误信息

**理由：**
- 失败分析增加了 ~1000 行代码
- MVP 阶段，简单的错误日志就够了
- 用户可以通过日志自己分析失败原因
- 减少了数据库查询和分析开销

**替代方案考虑：**
- 保留简化版（3-4 种类型）：仍然增加复杂度
- 使用外部日志分析工具：MVP 不需要

**影响：**
- 删除 `task_failure_analyzer.rs`
- 任务失败时只记录错误消息（字符串）
- 用户需要自己分析日志

### Decision 5: 删除执行历史表，只保留最近日志

**选择：** 删除 `task_executions` 表，在 `tasks` 表中直接存储最近的日志

**理由：**
- 执行历史表增加了复杂度（独立表 + 文件存储）
- MVP 阶段，只需要看最近的执行日志
- 减少了数据库表和查询复杂度

**替代方案考虑：**
- 保留历史表：增加复杂度，MVP 不需要
- 使用外部日志系统：增加依赖

**影响：**
- 删除 `task_executions` 表
- 在 `tasks` 表中添加 `last_log` 字段（TEXT）
- 只保留最近一次执行的日志（最多 10MB）

### Decision 6: 单 Agent 模式

**选择：** 每个 workspace 只能配置一个 agent

**理由：**
- 多 Agent 支持增加了约 300 行代码
- MVP 阶段，一个 workspace 一个 agent 就够了
- 简化了 agent 选择逻辑

**替代方案考虑：**
- 保留多 Agent：增加复杂度，MVP 不需要

**影响：**
- 简化 `agents` 表（添加 UNIQUE 约束：workspace_id）
- 删除 agent 选择逻辑
- 创建 agent 时检查是否已存在

### Decision 7: 简化任务状态机

**选择：** 移除 `Assigned` 状态，简化为 Pending → Running → Completed/Failed/Cancelled

**理由：**
- `Assigned` 状态在 MVP 中不必要（创建任务时直接指定 agent）
- 减少状态转换复杂度
- 简化状态机验证逻辑

**替代方案考虑：**
- 保留 Assigned：增加一个中间状态，MVP 不需要

**影响：**
- 修改 `task-state-machine` spec
- 更新状态转换逻辑
- 简化 API（创建任务时必须指定 agent_id）

### Decision 8: 删除所有清理和重试服务

**选择：** 删除 Webhook Retry、Webhook Cleanup、Log Cleanup 服务

**理由：**
- 这些服务增加了约 1500 行代码和 3 个后台服务
- MVP 阶段，手动清理就够了
- 减少了后台任务和数据库查询

**替代方案考虑：**
- 保留清理服务：增加复杂度，MVP 可以手动清理

**影响：**
- 删除 3 个服务文件
- Webhook 失败不会自动重试（用户需要手动重试）
- 旧日志不会自动清理（用户需要手动清理或重启）

### Decision 9: 最小化 API 端点

**选择：** 只保留 8 个核心 API 端点

**保留的端点：**
1. `POST /repositories` - 添加仓库配置
2. `POST /webhooks/github` - 接收 GitHub webhook
3. `GET /tasks` - 查询任务列表
4. `POST /tasks` - 手动创建任务
5. `POST /tasks/:id/execute` - 手动执行任务
6. `GET /tasks/:id/logs` - 查看任务日志
7. `GET /tasks/:id/status` - 查询任务状态
8. `DELETE /tasks/:id` - 删除任务

**删除的端点：**
- 所有 Provider 管理端点（改用环境变量配置）
- 所有 Workspace 管理端点（自动创建）
- 所有 Agent 管理端点（改用环境变量配置）
- 所有 Webhook 配置端点（自动配置）
- 所有统计和监控端点

**🚨 重要说明：保留 Git Provider 核心模块**

虽然删除 Provider 管理 API 端点，但**必须保留** `src/git_provider/` 核心模块：

- **保留模块**: `src/git_provider/` (4,111 行代码)
  - `git_provider/traits.rs` - GitProvider trait 定义
  - `git_provider/factory.rs` - GitClientFactory
  - `git_provider/gitea/` - Gitea 客户端实现
  - `git_provider/models.rs` - 数据模型
  - `git_provider/error.rs` - 错误类型

- **删除模块**: `src/api/settings/providers/` (727 行代码)
  - `providers/handlers.rs` - REST API handlers
  - `providers/routes.rs` - 路由定义
  - `providers/models.rs` - API 请求/响应模型

**保留原因：**
1. PR 创建服务 (`pr_creation_service.rs`) 依赖 `GitClientFactory::from_provider()`
2. Issue 关闭服务 (`issue_closure_service.rs`) 依赖 `GitProvider::update_issue()`
3. 这些是核心业务逻辑，不是管理功能
4. 删除将导致 Issue → PR 自动化完全失效

**配置方式变更：**
- 旧方式：通过 REST API 创建和管理 Provider 配置
- 新方式：通过环境变量配置（`GITHUB_TOKEN`, `GITHUB_BASE_URL` 等）
- Provider 配置在启动时加载，不再支持运行时修改

**理由：**
- MVP 阶段，核心流程只需要这 8 个端点
- Provider 配置改用环境变量（更简单、更安全）
- 减少了约 40+ 个管理端点和相关代码
- 保留核心业务功能（PR 创建、Issue 关闭）

**影响：**
- 删除 Provider 管理 API 代码（727 行）
- 保留 Git Provider 核心功能（4,111 行）
- 用户需要通过环境变量配置 Provider
- 简化了 API 文档

## Risks / Trade-offs

### Risk 1: 功能缺失导致用户流失

**风险：** 删除的功能可能是某些用户需要的

**缓解措施：**
- 保留完整版代码在 `main` 分支
- 在 `mvp-simplified` 分支进行简化
- 在文档中明确说明这是简化版
- 收集用户反馈，按需恢复功能

### Risk 2: 数据迁移困难

**风险：** 现有用户无法迁移到简化版

**缓解措施：**
- 不提供自动迁移（建议全新部署）
- 提供手动迁移脚本（如果需要）
- 在文档中说明这是 BREAKING 变更

### Risk 3: Webhook 失败无备用方案

**风险：** 如果 Webhook 失败，Issue 无法被检测

**缓解措施：**
- 提供手动创建任务的 API
- 在文档中说明如何手动创建任务
- 如果用户强烈需要，可以后续添加 Polling

### Risk 4: 日志轮询增加服务器负载

**风险：** 多个客户端轮询日志可能增加服务器负载

**缓解措施：**
- 建议客户端使用 5 秒轮询间隔
- 添加 rate limiting（如果需要）
- 日志查询使用索引优化

### Risk 5: 单 Agent 限制灵活性

**风险：** 用户可能需要为不同任务使用不同 Agent

**缓解措施：**
- 用户可以创建多个 workspace（每个 workspace 一个 agent）
- 如果用户强烈需要，可以后续恢复多 Agent 支持

### Trade-off 1: 简化 vs 功能完整性

**选择：** 优先简化，牺牲功能完整性

**理由：** MVP 阶段需要快速验证核心价值，功能完整性可以后续添加

### Trade-off 2: 实时性 vs 复杂度

**选择：** 牺牲实时性（WebSocket），降低复杂度

**理由：** 2-5 秒的日志延迟在 MVP 阶段可以接受

### Trade-off 3: 自动化 vs 手动配置

**选择：** 部分配置改为手动（Provider、Agent）

**理由：** 减少 API 端点和配置管理复杂度

## Migration Plan

### Phase 1: 创建简化分支（Day 1）

```bash
git checkout -b mvp-simplified
```

### Phase 2: 删除服务和代码（Day 1-2）

**删除文件：**
```bash
# 删除服务
rm src/services/issue_polling_service.rs
rm src/services/webhook_retry_service.rs
rm src/services/webhook_cleanup_service.rs
rm src/services/log_cleanup_service.rs
rm src/services/init_script_service.rs
rm src/services/task_failure_analyzer.rs
rm src/services/health_check_service.rs
rm src/services/image_management_service.rs
rm src/services/task_log_broadcaster.rs

# 删除 WebSocket 路由
# 编辑 src/api/mod.rs，删除 WebSocket 相关路由
```

**更新 mod.rs：**
- 从 `src/services/mod.rs` 中移除已删除服务的导出
- 从 `src/main.rs` 中移除服务注册代码

### Phase 3: 简化数据库（Day 2-3）

**创建新的 migration：**
```bash
sea-orm-cli migrate generate simplify_mvp
```

**Migration 内容：**
```sql
-- 删除表
DROP TABLE IF EXISTS webhook_configs;
DROP TABLE IF EXISTS init_scripts;
DROP TABLE IF EXISTS task_executions;
DROP TABLE IF EXISTS webhook_delivery_logs;

-- 简化 tasks 表
ALTER TABLE tasks DROP COLUMN retry_count;
ALTER TABLE tasks DROP COLUMN max_retries;
ALTER TABLE tasks ADD COLUMN last_log TEXT;

-- 简化 agents 表
ALTER TABLE agents DROP COLUMN enabled;
ALTER TABLE agents ADD CONSTRAINT unique_workspace_agent UNIQUE(workspace_id);

-- 删除 workspaces 表（workspace 信息合并到 repositories）
DROP TABLE IF EXISTS workspaces;
ALTER TABLE repositories ADD COLUMN docker_image VARCHAR(255) DEFAULT 'ubuntu:22.04';
ALTER TABLE repositories ADD COLUMN agent_command TEXT;
ALTER TABLE repositories ADD COLUMN agent_timeout INTEGER DEFAULT 600;
```

### Phase 4: 简化 API（Day 3-4）

**删除 API 模块：**
```bash
rm -rf src/api/providers/
rm -rf src/api/workspaces/
rm -rf src/api/agents/
rm -rf src/api/webhooks/
# 保留 src/api/tasks/ 和 src/api/repositories/
```

**更新路由：**
- 只保留 8 个核心端点
- 更新 OpenAPI 文档

### Phase 5: 更新测试（Day 4-5）

**删除测试：**
```bash
# 删除对应被删除功能的测试
rm tests/*_polling_*.rs
rm tests/*_webhook_retry_*.rs
rm tests/*_init_script_*.rs
# 等等
```

**更新核心测试：**
- 更新状态机测试（移除 Assigned 状态）
- 更新 API 测试（只测试 8 个端点）
- 更新 E2E 测试（简化场景）

### Phase 6: 更新文档（Day 5）

**更新文档：**
- README.md - 说明这是简化版
- docs/api/user-guide.md - 只保留核心功能文档
- docs/api/api-reference.md - 只保留 8 个端点
- AGENTS.md - 更新开发指南

### Phase 7: 测试和验证（Day 6-7）

```bash
# 运行所有测试
cargo test

# 运行 E2E 测试
./scripts/run_e2e_tests.sh

# 手动测试核心流程
```

### Rollback Strategy

如果简化版出现问题：

```bash
# 回到 main 分支
git checkout main

# 或者恢复特定功能
git checkout main -- src/services/issue_polling_service.rs
```

## Open Questions

1. **配置管理：** Provider 和 Agent 配置应该用配置文件还是环境变量？
   - 建议：使用环境变量（更简单）

2. **日志存储：** 最近日志应该存储多少？10MB 够吗？
   - 建议：10MB，如果不够可以调整

3. **Webhook 签名验证：** 是否保留 Webhook 签名验证？
   - 建议：保留（安全性重要）

4. **Docker 镜像：** 应该提供哪些预构建镜像？
   - 建议：ubuntu:22.04 + 常用工具（git, curl, opencode）

5. **API 认证：** 是否需要添加 API Key 认证？
   - 建议：是，添加简单的 API Key 认证（不在本次变更中）
