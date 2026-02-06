# 简化 MVP 提案

## 🚨 重要说明：保留核心业务逻辑

本次简化的目标是删除**管理功能**和**辅助功能**，但**必须保留核心业务逻辑**：

- ✅ 保留：PR 创建功能（`src/git_provider/` + `pr_creation_service.rs`）
- ✅ 保留：Issue 关闭功能（`src/git_provider/` + `issue_closure_service.rs`）
- ✅ 保留：Webhook 接收和处理
- ✅ 保留：Docker 容器执行
- ❌ 删除：Provider 管理 API（改用环境变量）
- ❌ 删除：轮询、重试、清理等辅助服务

**关键模块保留说明：**
- `src/git_provider/` (4,111 行) - **必须保留**，这是 PR 创建的核心
- `src/api/settings/providers/` (727 行) - 删除，改用环境变量配置

---

## Why

VibeRepo 当前实现存在过度设计问题，代码量达到 15,000+ 行，包含 10 个数据库表、6 个后台服务和 50+ 个 API 端点。这种复杂度阻碍了快速迭代和用户验证。我们需要简化到核心 MVP 功能（Issue → Agent → PR 自动化），将代码量减少 60%，以便快速验证产品价值并获取用户反馈。

## What Changes

- **BREAKING**: 删除 Issue Polling Service（保留 Webhook 作为唯一的 Issue 检测方式）
- **BREAKING**: 删除 Webhook Retry Service 和 Webhook Cleanup Service
- **BREAKING**: 删除 Init Scripts 功能（使用预构建的 Docker 镜像）
- **BREAKING**: 删除 WebSocket 实时日志流（改用 REST API 轮询）
- **BREAKING**: 删除智能失败分析系统（简化为基础错误日志）
- **BREAKING**: 删除 Task Execution History 表（只保留最近日志）
- **BREAKING**: 删除 Log Cleanup Service
- **BREAKING**: 删除 Health Check Service 和 Image Management Service
- **BREAKING**: 删除多 Agent 支持（每个 workspace 只能有一个 agent）
- **BREAKING**: 简化 Repository Service（移除定期同步功能）
- 简化数据库模型：从 10 个表减少到 4 个核心表（repositories, agents, tasks, task_logs）
- 简化 API 端点：从 50+ 个减少到 8 个核心端点
- 简化后台服务：从 6 个减少到 1 个（TaskSchedulerService）
- 保留核心功能：Webhook 接收、Task 创建和执行、Docker 容器运行、PR 创建

## Capabilities

### New Capabilities

- `simplified-task-execution`: 简化的任务执行流程，移除复杂的历史追踪和失败分析，只保留基础的任务状态管理和日志记录
- `minimal-api-surface`: 最小化的 API 接口，只包含核心的 8 个端点（仓库管理、webhook、任务 CRUD、日志查询）
- `single-agent-workspace`: 简化的 workspace-agent 关系，每个 workspace 只能配置一个 agent

### Modified Capabilities

- `task-state-machine`: 简化任务状态机，移除 Assigned 状态，只保留 Pending → Running → Completed/Failed/Cancelled 的基本流程

## Impact

**代码影响：**
- 删除约 9,000 行代码（60% 代码量）
- 删除 8 个服务文件：`issue_polling_service.rs`, `webhook_retry_service.rs`, `webhook_cleanup_service.rs`, `log_cleanup_service.rs`, `init_script_service.rs`, `task_failure_analyzer.rs`, `health_check_service.rs`, `image_management_service.rs`
- 删除 WebSocket 相关代码：`task_log_broadcaster.rs` 和 WebSocket 路由
- 删除 Provider 管理 API：`src/api/settings/providers/` (727 行)
- **保留** Git Provider 核心模块：`src/git_provider/` (4,111 行) - PR 创建和 Issue 关闭的核心依赖
- 简化 `task_executor_service.rs` 和 `task_scheduler_service.rs`

**数据库影响：**
- 删除 6 个表：`webhook_configs`, `init_scripts`, `task_executions`, `webhook_delivery_logs` 等
- 简化 `tasks` 表结构（移除 retry 相关字段）
- 简化 `agents` 表（移除 enabled 字段，通过存在性判断）

**API 影响：**
- 删除 40+ 个 API 端点
- 保留 8 个核心端点：
  - `POST /repositories` - 添加仓库
  - `POST /webhooks/github` - 接收 webhook
  - `GET /tasks` - 查询任务列表
  - `POST /tasks` - 创建任务
  - `POST /tasks/:id/execute` - 执行任务
  - `GET /tasks/:id/logs` - 查看日志
  - `GET /tasks/:id/status` - 查询状态
  - `DELETE /tasks/:id` - 删除任务

**依赖影响：**
- 可能移除 `futures-util` 依赖（WebSocket 相关）
- 保留核心依赖：`axum`, `tokio`, `sea-orm`, `bollard`

**测试影响：**
- 删除约 400 个测试用例（对应被删除的功能）
- 保留约 200 个核心测试用例
- 简化 E2E 测试场景

**文档影响：**
- 更新所有 API 文档，标注已删除的功能
- 更新用户指南，聚焦核心流程
- 更新架构文档，反映简化后的设计
