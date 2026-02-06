# 简化 MVP 实施任务清单

## 🚨 关键警告：Git Provider 核心模块必须保留

**删除范围说明：**

| 模块 | 路径 | 代码量 | 操作 | 原因 |
|------|------|--------|------|------|
| Provider 管理 API | `src/api/settings/providers/` | 727 行 | ✅ 删除 | 改用环境变量配置 |
| Git Provider 核心 | `src/git_provider/` | 4,111 行 | ❌ 保留 | PR 创建和 Issue 关闭的核心依赖 |

**为什么必须保留 `src/git_provider/`：**
1. **PR 创建服务** (`pr_creation_service.rs`) 100% 依赖此模块
2. **Issue 关闭服务** (`issue_closure_service.rs`) 100% 依赖此模块
3. 删除将导致 Issue → PR 自动化流程完全失效
4. 这是系统的核心价值，不是可选功能

**实施时请注意：**
- Task 7.1 只删除 API 端点，不删除核心模块
- 所有涉及 "Provider" 的任务都指 API 层，不是核心模块
- 如有疑问，请先确认再操作

---

## 0. 设置 Git Worktree（必须）

**重要：** 所有实施工作必须在专属的 git worktree 中进行，以避免影响主工作目录。

- [ ] 0.1 创建 .worktrees 目录（如果不存在）：`mkdir -p .worktrees`
- [ ] 0.2 创建 worktree：`git worktree add .worktrees/simplify-mvp mvp-simplified`
- [ ] 0.3 切换到 worktree 目录：`cd .worktrees/simplify-mvp`
- [ ] 0.4 验证当前在 mvp-simplified 分支：`git branch --show-current`
- [ ] 0.5 确认 worktree 独立性：`git worktree list`

## 1. 创建简化分支

- [ ] 1.1 从 main 分支创建 mvp-simplified 分支（如果不存在）
- [ ] 1.2 确认所有测试在当前分支通过
- [ ] 1.3 创建备份标签 pre-simplification

## 2. 删除后台服务（每个服务删除前需评估）

**重要：** 每个服务删除前必须评估影响，确保没有遗漏的依赖。

### 2.1 评估并删除 Issue Polling Service

- [ ] 2.1.1 评估：搜索 IssuePollingService 的所有引用：`grep -r "IssuePollingService\|issue_polling" backend/src/`
- [ ] 2.1.2 评估：检查 main.rs 中的服务注册
- [ ] 2.1.3 评估：检查配置文件中的 polling 相关配置
- [ ] 2.1.4 删除：删除 src/services/issue_polling_service.rs
- [ ] 2.1.5 删除：从 src/services/mod.rs 移除导出
- [ ] 2.1.6 删除：从 src/main.rs 移除服务注册
- [ ] 2.1.7 删除：从 src/config.rs 移除 IssuePollingConfig（如果存在）
- [ ] 2.1.8 验证：运行 `cargo check` 确认无编译错误

### 2.2 评估并删除 Webhook Retry Service

- [ ] 2.2.1 评估：搜索 WebhookRetryService 的所有引用：`grep -r "WebhookRetryService\|webhook_retry" backend/src/`
- [ ] 2.2.2 评估：检查是否有 webhook_delivery_logs 表依赖
- [ ] 2.2.3 评估：检查配置中的 retry 相关设置
- [ ] 2.2.4 删除：删除 src/services/webhook_retry_service.rs
- [ ] 2.2.5 删除：从 src/services/mod.rs 移除导出
- [ ] 2.2.6 删除：从 src/main.rs 移除服务注册
- [ ] 2.2.7 验证：运行 `cargo check` 确认无编译错误

### 2.3 评估并删除 Webhook Cleanup Service

- [ ] 2.3.1 评估：搜索 WebhookCleanupService 的所有引用：`grep -r "WebhookCleanupService\|webhook_cleanup" backend/src/`
- [ ] 2.3.2 评估：检查是否有定时清理任务依赖
- [ ] 2.3.3 删除：删除 src/services/webhook_cleanup_service.rs
- [ ] 2.3.4 删除：从 src/services/mod.rs 移除导出
- [ ] 2.3.5 删除：从 src/main.rs 移除服务注册
- [ ] 2.3.6 验证：运行 `cargo check` 确认无编译错误

### 2.4 评估并删除 Log Cleanup Service

- [ ] 2.4.1 评估：搜索 LogCleanupService 的所有引用：`grep -r "LogCleanupService\|log_cleanup" backend/src/`
- [ ] 2.4.2 评估：检查是否有日志文件清理逻辑依赖
- [ ] 2.4.3 删除：删除 src/services/log_cleanup_service.rs
- [ ] 2.4.4 删除：从 src/services/mod.rs 移除导出
- [ ] 2.4.5 删除：从 src/main.rs 移除服务注册
- [ ] 2.4.6 验证：运行 `cargo check` 确认无编译错误

### 2.5 评估并删除 Init Script Service

- [ ] 2.5.1 评估：搜索 InitScriptService 的所有引用：`grep -r "InitScriptService\|init_script" backend/src/`
- [ ] 2.5.2 评估：检查 init_scripts 表的使用情况
- [ ] 2.5.3 评估：检查 workspace 创建流程中的 init script 调用
- [ ] 2.5.4 删除：删除 src/services/init_script_service.rs
- [ ] 2.5.5 删除：从 src/services/mod.rs 移除导出
- [ ] 2.5.6 删除：从 workspace 相关代码移除 init script 调用
- [ ] 2.5.7 验证：运行 `cargo check` 确认无编译错误

### 2.6 评估并删除 Task Failure Analyzer

- [ ] 2.6.1 评估：搜索 TaskFailureAnalyzer 的所有引用：`grep -r "TaskFailureAnalyzer\|failure_analyzer" backend/src/`
- [ ] 2.6.2 评估：检查 task_executor 中的失败分析调用
- [ ] 2.6.3 评估：检查 API 中的失败分析端点
- [ ] 2.6.4 删除：删除 src/services/task_failure_analyzer.rs
- [ ] 2.6.5 删除：从 src/services/mod.rs 移除导出
- [ ] 2.6.6 删除：从 task_executor 移除失败分析调用
- [ ] 2.6.7 删除：删除 API 中的失败分析端点（如果存在）
- [ ] 2.6.8 验证：运行 `cargo check` 确认无编译错误

### 2.7 评估并删除 Health Check Service

- [ ] 2.7.1 评估：搜索 HealthCheckService 的所有引用：`grep -r "HealthCheckService\|health_check" backend/src/`
- [ ] 2.7.2 评估：检查是否有健康检查 API 端点
- [ ] 2.7.3 评估：检查容器健康检查逻辑
- [ ] 2.7.4 删除：删除 src/services/health_check_service.rs
- [ ] 2.7.5 删除：从 src/services/mod.rs 移除导出
- [ ] 2.7.6 删除：从 src/main.rs 移除服务注册（如果有）
- [ ] 2.7.7 删除：删除健康检查 API 端点（如果存在）
- [ ] 2.7.8 验证：运行 `cargo check` 确认无编译错误

### 2.8 评估并删除 Image Management Service

- [ ] 2.8.1 评估：搜索 ImageManagementService 的所有引用：`grep -r "ImageManagementService\|image_management" backend/src/`
- [ ] 2.8.2 评估：检查 Docker 镜像构建和管理逻辑
- [ ] 2.8.3 评估：检查 API 中的镜像管理端点
- [ ] 2.8.4 删除：删除 src/services/image_management_service.rs
- [ ] 2.8.5 删除：从 src/services/mod.rs 移除导出
- [ ] 2.8.6 删除：删除镜像管理 API 端点（如果存在）
- [ ] 2.8.7 验证：运行 `cargo check` 确认无编译错误

### 2.9 最终验证

- [ ] 2.9.1 运行 `cargo clippy` 检查未使用的导入
- [ ] 2.9.2 运行 `cargo test` 确认测试通过
- [ ] 2.9.3 检查 src/services/mod.rs 确认所有删除的服务已移除
- [ ] 2.9.4 检查 src/main.rs 确认所有服务注册已移除
- [ ] 2.9.5 搜索残留引用：`grep -r "PollingService\|RetryService\|CleanupService\|FailureAnalyzer\|HealthCheck\|ImageManagement" backend/src/`

## 3. 删除 WebSocket 功能（需全面评估）

**重要：** 删除 WebSocket 前必须全面评估影响，确保没有遗留代码或依赖。

**影响范围总结：**
- 删除文件：3 个（task_log_broadcaster.rs, websocket.rs, 相关测试）
- 修改文件：9 个（config.rs, state.rs, main.rs, 2个service, 3个api文件, mod.rs）
- 删除代码：约 600 行
- 保留依赖：futures（issue_polling 和 docker_service 使用）
- 移除依赖：axum["ws"] feature, 可能移除 futures-util

### 3.1 Phase 1: 删除路由和 API 层

- [ ] 3.1.1 从 src/api/tasks/routes.rs 移除 WebSocket 路由（第 43-46 行）
- [ ] 3.1.2 从 src/api/tasks/mod.rs 移除 websocket 模块导入（第 4 行）
- [ ] 3.1.3 从 src/api/tasks/mod.rs 移除 websocket 导出（第 9 行）
- [ ] 3.1.4 删除 src/api/tasks/websocket.rs 文件（205 行）
- [ ] 3.1.5 运行 `cargo check` 验证编译（预期：log_broadcaster 相关错误）

### 3.2 Phase 2: 删除服务层使用

- [ ] 3.2.1 从 src/services/task_executor_service.rs 删除两处 broadcast 调用（第 440, 458 行）
- [ ] 3.2.2 从 src/services/task_executor_service.rs 删除 log_broadcaster 字段（第 60 行）
- [ ] 3.2.3 从 src/services/task_executor_service.rs 删除构造函数参数（第 67 行）
- [ ] 3.2.4 从 src/services/task_scheduler_service.rs 删除传递给 TaskExecutorService 的 log_broadcaster（第 167, 268 行）
- [ ] 3.2.5 从 src/services/task_scheduler_service.rs 删除 log_broadcaster 字段（第 42 行）
- [ ] 3.2.6 从 src/services/task_scheduler_service.rs 删除构造函数参数（第 51 行）
- [ ] 3.2.7 从 src/api/tasks/handlers.rs 删除传递的 log_broadcaster（第 420 行）
- [ ] 3.2.8 运行 `cargo check` 验证编译（预期：state.log_broadcaster 相关错误）

### 3.3 Phase 3: 删除状态层

- [ ] 3.3.1 从 src/state.rs 删除 log_broadcaster 字段（第 23 行）
- [ ] 3.3.2 从 src/state.rs 删除 get_or_create_log_channel() 方法
- [ ] 3.3.3 从 src/state.rs 删除 broadcast_log() 方法
- [ ] 3.3.4 从 src/state.rs 删除 cleanup_log_channel() 方法
- [ ] 3.3.5 从 src/state.rs 删除 TaskLogBroadcaster::new() 初始化
- [ ] 3.3.6 从 src/main.rs 删除传递给 TaskSchedulerService 的 log_broadcaster（第 96 行）
- [ ] 3.3.7 运行 `cargo check` 验证编译（预期：TaskLogBroadcaster 未定义错误）

### 3.4 Phase 4: 删除核心实现

- [ ] 3.4.1 删除 src/services/task_log_broadcaster.rs 文件（150 行）
- [ ] 3.4.2 从 src/services/mod.rs 删除 task_log_broadcaster 模块导出（第 22 行）
- [ ] 3.4.3 从 src/services/mod.rs 删除 TaskLogBroadcaster 导出（第 49 行）
- [ ] 3.4.4 运行 `cargo check` 验证编译（预期：WebSocketConfig 相关错误）

### 3.5 Phase 5: 清理配置和依赖

- [ ] 3.5.1 从 src/config.rs 删除 WebSocketConfig 结构体（第 207-220 行）
- [ ] 3.5.2 从 src/config.rs 的 AppConfig 删除 websocket 字段
- [ ] 3.5.3 更新 src/config.rs 中所有测试的配置初始化（删除 websocket 字段）
- [ ] 3.5.4 搜索并更新其他测试中的 WebSocketConfig 引用：`grep -r "WebSocketConfig\|websocket:" backend/src/`
- [ ] 3.5.5 从 Cargo.toml 的 axum 依赖移除 "ws" feature
- [ ] 3.5.6 检查 futures 使用：`grep -r "use futures" backend/src/` （确认 issue_polling 和 docker_service 使用）
- [ ] 3.5.7 评估是否移除 futures-util：`grep -r "futures_util" backend/src/` （如果只在 websocket.rs 使用则移除）
- [ ] 3.5.8 运行 `cargo build` 确认编译成功

### 3.6 Phase 6: 更新文档和注释

- [ ] 3.6.1 从 docs/api/api-reference.md 删除 WebSocket 端点文档
- [ ] 3.6.2 从 docs/api/user-guide.md 删除实时日志流章节（WebSocket 部分）
- [ ] 3.6.3 更新 README.md 移除 WebSocket 相关说明
- [ ] 3.6.4 搜索代码注释中的 WebSocket 引用：`grep -r "WebSocket\|websocket" backend/src/ --include="*.rs"`
- [ ] 3.6.5 更新找到的注释，移除 WebSocket 相关说明

### 3.7 Phase 7: 验证清理完整性

- [ ] 3.7.1 运行完整性检查脚本（见下方）
- [ ] 3.7.2 运行 `cargo clippy` 检查警告
- [ ] 3.7.3 运行 `cargo test` 确认所有测试通过
- [ ] 3.7.4 运行 `cargo build --release` 确认发布版本编译
- [ ] 3.7.5 手动检查 Cargo.toml 确认依赖正确

### 3.8 创建验证脚本

- [ ] 3.8.1 创建 scripts/verify_websocket_removal.sh
- [ ] 3.8.2 添加检查残留引用的命令
- [ ] 3.8.3 添加编译和测试验证
- [ ] 3.8.4 添加依赖检查
- [ ] 3.8.5 运行验证脚本确认清理完整

**验证脚本内容：**
```bash
#!/bin/bash
echo "=== WebSocket 删除验证 ==="
echo "1. 检查残留引用..."
grep -r "websocket\|WebSocket" backend/src/ && echo "❌ 发现残留" || echo "✓ 无残留"
grep -r "log_broadcaster" backend/src/ && echo "❌ 发现残留" || echo "✓ 无残留"
echo "2. 检查编译..."
cd backend && cargo check && echo "✓ 编译成功" || echo "❌ 编译失败"
echo "3. 运行测试..."
cargo test && echo "✓ 测试通过" || echo "❌ 测试失败"
echo "4. 检查依赖..."
grep 'features.*"ws"' Cargo.toml && echo "⚠️ 仍有 ws feature" || echo "✓ 已移除 ws"
```

## 4. 简化数据库模型（每个表删除前需评估）

**重要：** 每个表删除前必须评估影响，确保没有外键约束或代码依赖。

### 4.1 创建 Migration

- [ ] 4.1.1 创建新的 migration：`sea-orm-cli migrate generate simplify_mvp_schema`
- [ ] 4.1.2 在 migration 文件中添加注释说明变更目的

### 4.2 评估并删除 webhook_configs 表

- [ ] 4.2.1 评估：搜索 webhook_configs 的所有引用：`grep -r "webhook_config" backend/src/`
- [ ] 4.2.2 评估：检查是否有外键约束
- [ ] 4.2.3 评估：检查 entity 文件使用情况
- [ ] 4.2.4 删除：在 migration 中添加 `DROP TABLE IF EXISTS webhook_configs;`
- [ ] 4.2.5 删除：删除 src/entities/webhook_config.rs（如果存在）
- [ ] 4.2.6 删除：从 src/entities/mod.rs 移除导出

### 4.3 评估并删除 init_scripts 表

- [ ] 4.3.1 评估：搜索 init_scripts 的所有引用：`grep -r "init_script" backend/src/entities/`
- [ ] 4.3.2 评估：检查与 workspaces 表的关系
- [ ] 4.3.3 评估：检查是否有日志文件存储路径依赖
- [ ] 4.3.4 删除：在 migration 中添加 `DROP TABLE IF EXISTS init_scripts;`
- [ ] 4.3.5 删除：删除 src/entities/init_script.rs
- [ ] 4.3.6 删除：从 src/entities/mod.rs 移除导出

### 4.4 评估并删除 task_executions 表

- [ ] 4.4.1 评估：搜索 task_executions 的所有引用：`grep -r "task_execution" backend/src/`
- [ ] 4.4.2 评估：检查执行历史查询逻辑
- [ ] 4.4.3 评估：检查 API 端点是否依赖此表
- [ ] 4.4.4 删除：在 migration 中添加 `DROP TABLE IF EXISTS task_executions;`
- [ ] 4.4.5 删除：删除 src/entities/task_execution.rs
- [ ] 4.4.6 删除：从 src/entities/mod.rs 移除导出
- [ ] 4.4.7 删除：删除相关的 service 方法

### 4.5 评估并删除 workspaces 表

- [ ] 4.5.1 评估：搜索 workspaces 的所有引用：`grep -r "workspace" backend/src/entities/`
- [ ] 4.5.2 评估：检查与 tasks 和 agents 表的外键关系
- [ ] 4.5.3 评估：确认 workspace 信息将合并到 repositories 表
- [ ] 4.5.4 删除：在 migration 中添加 `DROP TABLE IF EXISTS workspaces;`
- [ ] 4.5.5 删除：删除 src/entities/workspace.rs
- [ ] 4.5.6 删除：从 src/entities/mod.rs 移除导出
- [ ] 4.5.7 更新：修改 tasks 和 agents 表的外键（如果需要）

### 4.6 修改 tasks 表

- [ ] 4.6.1 评估：检查 tasks 表当前结构：`grep -A 20 "struct Task" backend/src/entities/task.rs`
- [ ] 4.6.2 添加：在 migration 中添加 `ALTER TABLE tasks ADD COLUMN last_log TEXT;`
- [ ] 4.6.3 删除：在 migration 中添加 `ALTER TABLE tasks DROP COLUMN retry_count;`
- [ ] 4.6.4 删除：在 migration 中添加 `ALTER TABLE tasks DROP COLUMN max_retries;`
- [ ] 4.6.5 更新：更新 src/entities/task.rs 添加 last_log 字段
- [ ] 4.6.6 更新：从 src/entities/task.rs 移除 retry 相关字段

### 4.7 修改 repositories 表

- [ ] 4.7.1 评估：检查 repositories 表当前结构
- [ ] 4.7.2 添加：在 migration 中添加 `ALTER TABLE repositories ADD COLUMN agent_command TEXT;`
- [ ] 4.7.3 添加：在 migration 中添加 `ALTER TABLE repositories ADD COLUMN agent_timeout INTEGER DEFAULT 600;`
- [ ] 4.7.4 添加：在 migration 中添加 `ALTER TABLE repositories ADD COLUMN agent_env_vars JSON;`
- [ ] 4.7.5 添加：在 migration 中添加 `ALTER TABLE repositories ADD COLUMN docker_image VARCHAR(255) DEFAULT 'ubuntu:22.04';`
- [ ] 4.7.6 更新：更新 src/entities/repository.rs 添加新字段

### 4.8 修改 agents 表

- [ ] 4.8.1 评估：检查 agents 表当前结构和使用情况
- [ ] 4.8.2 删除：在 migration 中添加 `ALTER TABLE agents DROP COLUMN enabled;`
- [ ] 4.8.3 添加：在 migration 中添加 `ALTER TABLE agents ADD CONSTRAINT unique_workspace_agent UNIQUE(workspace_id);`
- [ ] 4.8.4 更新：从 src/entities/agent.rs 移除 enabled 字段
- [ ] 4.8.5 更新：更新所有检查 enabled 状态的代码

### 4.9 运行 Migration

- [ ] 4.9.1 验证：检查 migration SQL 语法正确性
- [ ] 4.9.2 备份：创建数据库备份（如果有数据）
- [ ] 4.9.3 运行：执行 migration：`sea-orm-cli migrate up`
- [ ] 4.9.4 验证：检查数据库结构：`sqlite3 db.db ".schema"`
- [ ] 4.9.5 验证：运行 `cargo check` 确认 entity 更新正确

### 4.10 最终验证

- [ ] 4.10.1 运行 `cargo test` 确认所有测试通过
- [ ] 4.10.2 检查是否有残留的 entity 文件
- [ ] 4.10.3 验证数据库表数量：应该从 10 个减少到 4 个
- [ ] 4.10.4 搜索残留引用：`grep -r "webhook_config\|init_script\|task_execution\|workspace" backend/src/entities/`

## 5. 简化任务状态机

- [ ] 5.1 更新 TaskStatus enum，移除 Assigned 变体
- [ ] 5.2 更新状态转换验证逻辑，移除 Assigned 相关转换
- [ ] 5.3 更新 Pending 状态允许直接转换到 Running
- [ ] 5.4 更新 Failed 状态为终态，移除重试转换
- [ ] 5.5 更新 is_terminal() 方法包含 Failed 状态
- [ ] 5.6 移除 assign_agent() 方法
- [ ] 5.7 移除 retry_task() 方法
- [ ] 5.8 更新状态机测试以反映简化的状态转换
- [ ] 5.9 在 migration 中添加数据转换：Assigned → Pending

## 6. 简化任务执行

- [ ] 6.1 更新 TaskExecutorService 移除执行历史记录功能
- [ ] 6.2 更新任务执行将日志存储到 tasks.last_log 字段
- [ ] 6.3 实现日志大小限制（10MB）和截断逻辑
- [ ] 6.4 移除失败分析调用
- [ ] 6.5 简化错误处理，只记录错误消息字符串
- [ ] 6.6 移除 retry 相关逻辑
- [ ] 6.7 保留 PR 信息提取和创建功能
- [ ] 6.8 更新 TaskSchedulerService 移除复杂的调度逻辑

## 7. 删除 API 端点（每个模块删除前需评估）

**重要：** 每个 API 模块删除前必须评估影响，确保没有客户端依赖。

### 7.1 评估并删除 Providers API

**🚨 重要警告：只删除 API 层，必须保留核心 git_provider 模块！**

**删除范围：**
- ✅ 删除：`src/api/settings/providers/` - Provider 管理 API（727 行）
- ❌ 保留：`src/git_provider/` - Git Provider 核心模块（4,111 行）

**保留原因：**
- PR 创建功能完全依赖 `git_provider` 模块（`pr_creation_service.rs`）
- Issue 关闭功能完全依赖 `git_provider` 模块（`issue_closure_service.rs`）
- 删除核心模块将导致 Issue → PR 自动化流程完全失效
- 这是系统的核心价值，不能删除

**实施步骤：**

- [ ] 7.1.1 评估：搜索 providers API 的所有引用：`grep -r "api/settings/providers\|api/providers" backend/src/`
- [ ] 7.1.2 评估：检查是否有前端或脚本调用这些端点
- [ ] 7.1.3 评估：确认 provider 配置将改用环境变量
- [ ] 7.1.4 删除：删除 `src/api/settings/providers/` 目录（仅 API 层）
- [ ] 7.1.5 删除：从 `src/api/settings/mod.rs` 移除 providers 路由注册
- [ ] 7.1.6 **确认：`src/git_provider/` 模块完全保留，不做任何修改**
- [ ] 7.1.7 验证：运行 `cargo check` 确认无编译错误
- [ ] 7.1.8 验证：确认 `pr_creation_service.rs` 和 `issue_closure_service.rs` 仍可正常编译

### 7.2 评估并删除 Workspaces API

- [ ] 7.2.1 评估：搜索 workspaces API 的所有引用：`grep -r "api/workspaces\|/workspaces" backend/src/`
- [ ] 7.2.2 评估：检查 workspace 创建流程的调用方
- [ ] 7.2.3 评估：确认 workspace 将自动创建
- [ ] 7.2.4 删除：删除 src/api/workspaces/ 目录
- [ ] 7.2.5 删除：从 src/api/mod.rs 移除 workspaces 路由注册
- [ ] 7.2.6 验证：运行 `cargo check` 确认无编译错误

### 7.3 评估并删除 Agents API

- [ ] 7.3.1 评估：搜索 agents API 的所有引用：`grep -r "api/agents\|/agents" backend/src/`
- [ ] 7.3.2 评估：检查 agent 管理的调用方
- [ ] 7.3.3 评估：确认 agent 配置将改用环境变量
- [ ] 7.3.4 删除：删除 src/api/agents/ 目录
- [ ] 7.3.5 删除：从 src/api/mod.rs 移除 agents 路由注册
- [ ] 7.3.6 验证：运行 `cargo check` 确认无编译错误

### 7.4 评估并删除 Webhook Config API

- [ ] 7.4.1 评估：搜索 webhook config API 的所有引用：`grep -r "webhooks/config" backend/src/`
- [ ] 7.4.2 评估：确认保留 webhooks/handlers.rs（接收 webhook）
- [ ] 7.4.3 评估：确认 webhook 配置将改用环境变量
- [ ] 7.4.4 删除：删除 src/api/webhooks/config.rs
- [ ] 7.4.5 删除：从 src/api/webhooks/mod.rs 移除 config 导出
- [ ] 7.4.6 验证：运行 `cargo check` 确认无编译错误

### 7.5 评估并删除 Stats API

- [ ] 7.5.1 评估：搜索 stats API 的所有引用：`grep -r "api/stats\|/stats" backend/src/`
- [ ] 7.5.2 评估：检查是否有监控系统依赖这些端点
- [ ] 7.5.3 删除：删除 src/api/stats/ 目录（如果存在）
- [ ] 7.5.4 删除：从 src/api/mod.rs 移除 stats 路由注册
- [ ] 7.5.5 验证：运行 `cargo check` 确认无编译错误

### 7.6 评估并删除 Health API

- [ ] 7.6.1 评估：搜索 health API 的所有引用：`grep -r "api/health\|/health" backend/src/`
- [ ] 7.6.2 评估：检查是否有负载均衡器或监控依赖
- [ ] 7.6.3 评估：确认是否需要保留基础的 /health 端点
- [ ] 7.6.4 删除：删除 src/api/health/ 目录（如果存在）
- [ ] 7.6.5 删除：从 src/api/mod.rs 移除 health 路由注册（如果完全删除）
- [ ] 7.6.6 验证：运行 `cargo check` 确认无编译错误

### 7.7 更新路由注册

- [ ] 7.7.1 检查 src/api/mod.rs 确认所有删除的模块已移除
- [ ] 7.7.2 验证只保留以下路由：repositories, webhooks, tasks
- [ ] 7.7.3 运行 `cargo check` 确认编译成功
- [ ] 7.7.4 运行 `cargo test` 确认测试通过

### 7.8 最终验证

- [ ] 7.8.1 列出所有剩余的 API 端点：`grep -r "Router::new\|\.route" backend/src/api/`
- [ ] 7.8.2 确认只有 8 个核心端点
- [ ] 7.8.3 搜索残留引用：`grep -r "/providers\|/workspaces\|/agents\|/stats" backend/src/`
- [ ] 7.8.4 更新 OpenAPI 文档移除已删除的端点

## 8. 实现最小化 API

- [ ] 8.1 确认 POST /repositories 端点存在并正常工作
- [ ] 8.2 确认 POST /webhooks/github 端点存在并正常工作
- [ ] 8.3 确认 GET /tasks 端点存在并支持基本过滤
- [ ] 8.4 确认 POST /tasks 端点存在并正常工作
- [ ] 8.5 确认 POST /tasks/:id/execute 端点存在并正常工作
- [ ] 8.6 实现 GET /tasks/:id/logs 端点返回 last_log
- [ ] 8.7 实现 GET /tasks/:id/status 端点返回状态和时间戳
- [ ] 8.8 确认 DELETE /tasks/:id 端点存在并正常工作
- [ ] 8.9 更新所有端点的 OpenAPI 文档

## 9. 实现单 Agent 模式

- [ ] 9.1 更新 Repository 创建逻辑接受 agent 配置参数
- [ ] 9.2 实现 workspace 创建时自动使用 repository 的 agent 配置
- [ ] 9.3 添加 UNIQUE 约束验证（每个 workspace 只能有一个 agent）
- [ ] 9.4 更新任务创建逻辑自动分配 workspace 的 agent
- [ ] 9.5 移除任务创建 API 中的 agent_id 参数
- [ ] 9.6 移除 agent enabled/disabled 状态检查

## 10. 更新配置管理

- [ ] 10.1 添加环境变量支持：GITHUB_TOKEN
- [ ] 10.2 添加环境变量支持：GITHUB_BASE_URL
- [ ] 10.3 添加环境变量支持：WEBHOOK_SECRET
- [ ] 10.4 添加环境变量支持：DEFAULT_AGENT_COMMAND
- [ ] 10.5 添加环境变量支持：DEFAULT_AGENT_TIMEOUT
- [ ] 10.6 添加环境变量支持：DEFAULT_DOCKER_IMAGE
- [ ] 10.7 更新 AppConfig 结构以反映新的配置方式
- [ ] 10.8 移除不再需要的配置选项

## 11. 删除测试

- [ ] 11.1 删除 issue_polling 相关测试
- [ ] 11.2 删除 webhook_retry 相关测试
- [ ] 11.3 删除 init_script 相关测试
- [ ] 11.4 删除 WebSocket 相关测试
- [ ] 11.5 删除 failure_analyzer 相关测试
- [ ] 11.6 删除 task_executions 相关测试
- [ ] 11.7 删除 health_check 相关测试
- [ ] 11.8 删除 image_management 相关测试
- [ ] 11.9 删除 providers API 测试
- [ ] 11.10 删除 workspaces API 测试
- [ ] 11.11 删除 agents API 测试
- [ ] 11.12 删除 webhook config API 测试

## 12. 更新核心测试

- [ ] 12.1 更新状态机测试移除 Assigned 状态
- [ ] 12.2 更新状态机测试移除 retry 功能
- [ ] 12.3 更新任务执行测试使用 last_log 字段
- [ ] 12.4 更新 API 测试只测试 8 个核心端点
- [ ] 12.5 添加日志大小限制测试
- [ ] 12.6 添加单 agent 约束测试
- [ ] 12.7 更新 E2E 测试简化场景
- [ ] 12.8 确认至少 200 个核心测试通过

## 13. 更新文档

- [ ] 13.1 更新 README.md 说明这是简化版 MVP
- [ ] 13.2 更新 docs/api/user-guide.md 移除已删除功能
- [ ] 13.3 更新 docs/api/api-reference.md 只保留 8 个端点
- [ ] 13.4 更新 AGENTS.md 反映简化后的架构
- [ ] 13.5 更新 docs/database/schema.md 反映新的数据库结构
- [ ] 13.6 创建 MIGRATION.md 说明从完整版迁移的注意事项
- [ ] 13.7 更新 docs/roadmap/README.md 标注简化版状态
- [ ] 13.8 更新 OpenAPI 文档（Swagger UI）

## 14. 验证和测试

- [ ] 14.1 运行所有单元测试：cargo test --lib
- [ ] 14.2 运行所有集成测试：cargo test --test '*'
- [ ] 14.3 运行 E2E 测试：./scripts/run_e2e_tests.sh
- [ ] 14.4 手动测试：创建 repository
- [ ] 14.5 手动测试：接收 webhook 并创建任务
- [ ] 14.6 手动测试：执行任务并查看日志
- [ ] 14.7 手动测试：验证 PR 创建
- [ ] 14.8 手动测试：验证 Issue 关闭
- [ ] 14.9 检查代码行数是否减少到 6000 左右
- [ ] 14.10 检查数据库表是否减少到 4 个

## 15. 性能和清理

- [ ] 15.1 运行 cargo clippy 检查警告
- [ ] 15.2 运行 cargo fmt 格式化代码
- [ ] 15.3 移除未使用的依赖（cargo-udeps）
- [ ] 15.4 更新 Cargo.toml 版本号为 0.4.0-mvp
- [ ] 15.5 检查并移除未使用的导入
- [ ] 15.6 检查并移除未使用的函数和结构体
- [ ] 15.7 优化编译时间（如果需要）

## 16. 部署准备

- [ ] 16.1 创建简化版的 .env.example
- [ ] 16.2 更新 docker-compose.yml（如果存在）
- [ ] 16.3 创建 Dockerfile（如果需要更新）
- [ ] 16.4 测试 Docker 构建
- [ ] 16.5 创建部署文档 docs/deployment/simplified-mvp.md
- [ ] 16.6 准备发布说明 CHANGELOG.md

## 17. 最终验证

- [ ] 17.1 在干净的环境中从头部署
- [ ] 17.2 验证所有 8 个 API 端点正常工作
- [ ] 17.3 验证完整的 Issue → PR 流程
- [ ] 17.4 验证日志查询功能
- [ ] 17.5 验证错误处理
- [ ] 17.6 检查内存使用情况
- [ ] 17.7 检查启动时间
- [ ] 17.8 代码审查

## 18. 发布

- [ ] 18.1 合并到 mvp-simplified 分支
- [ ] 18.2 创建 Git 标签 v0.4.0-mvp
- [ ] 18.3 推送到远程仓库
- [ ] 18.4 创建 GitHub Release
- [ ] 18.5 更新项目 README 添加简化版说明
- [ ] 18.6 通知团队和用户
