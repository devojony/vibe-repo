# Repository 管理功能设计文档索引

**创建日期**: 2026-01-17  
**版本**: v1.0  
**状态**: 设计阶段

## 文档概述

本目录包含 GitAutoDev Repository 管理功能的完整设计文档。该功能为 v0.1.20 版本添加了完整的仓库生命周期管理能力。

---

## 文档列表

### 1. [主设计文档](./2026-01-17-repository-management-design.md)

**内容**：完整的功能设计方案

**包含章节**：
- 概述和设计目标
- 背景和动机
- 核心设计决策（软删除、5种状态、归档过滤、批量操作）
- 状态机设计
- 软删除与同步策略
- 归档功能设计
- 批量操作设计
- 与 Workspace 的集成
- 风险和缓解措施
- 参考资料和附录

**适合读者**：产品经理、架构师、开发人员

---

### 2. [实施路线图](./2026-01-17-implementation-roadmap.md)

**内容**：详细的实施计划和时间线

**包含章节**：
- 时间线总览（3周，14-19天）
- Phase 1: 数据库层 (3-4 天)
- Phase 2: Service 层 (4-5 天)
- Phase 3: API 层 (3-4 天)
- Phase 4: 测试 (3-4 天)
- Phase 5: 文档和部署 (1-2 天)
- 里程碑和验收标准
- 风险管理

**适合读者**：项目经理、开发人员、测试人员

---

## 快速开始

### 阅读顺序建议

**对于产品经理/架构师**：
1. 主设计文档 → 了解整体设计
2. 实施路线图 → 了解时间和资源需求

**对于开发人员**：
1. 主设计文档 → 理解设计决策
2. 实施路线图 → 了解任务分解
3. 开始实施 Phase 1

**对于测试人员**：
1. 主设计文档（状态机部分）→ 理解业务逻辑
2. 实施路线图（Phase 4）→ 了解测试策略

---

## 核心设计要点

### 5种状态

| 状态 | 说明 |
|------|------|
| `uninitialized` | 未初始化 - 刚从 Provider 同步 |
| `idle` | 闲置 - 已初始化但未关联 Workspace |
| `active` | 活动 - 已关联 Workspace，处理 Issue |
| `unavailable` | 不可用 - 验证失败或权限不足 |
| `archived` | 归档 - 不再使用，只读 |

### 关键特性

✅ **软删除** - 使用 `deleted_at`，同步时自动恢复  
✅ **归档过滤** - 同步时跳过归档仓库  
✅ **批量操作** - 部分成功模式，返回详细结果  
✅ **完全只读** - 归档仓库不允许任何修改  
✅ **自动状态转换** - 系统自动管理状态变化  

---

## 新增 API 端点

### 单个操作
- `PATCH /api/repositories/:id` - 更新元数据
- `POST /api/repositories/:id/archive` - 归档
- `POST /api/repositories/:id/unarchive` - 取消归档
- `DELETE /api/repositories/:id` - 软删除
- `POST /api/repositories/:id/reinitialize` - 重新初始化

### 批量操作
- `POST /api/repositories/batch-archive` - 批量归档
- `POST /api/repositories/batch-delete` - 批量删除
- `POST /api/repositories/batch-refresh` - 批量刷新
- `POST /api/repositories/batch-reinitialize` - 批量重新初始化

### 查询增强
- `GET /api/repositories?status=idle&has_workspace=false&search=test`

---

## 数据库变更

### 新增字段
- `status` (TEXT, NOT NULL, DEFAULT 'uninitialized')
- `has_workspace` (BOOLEAN, NOT NULL, DEFAULT FALSE)
- `deleted_at` (TIMESTAMP, NULL)

### 新增索引
- `idx_repositories_status`
- `idx_repositories_deleted_at`
- `idx_repositories_has_workspace`

---

## 实施时间线

```
Week 1: 数据库层 + Service 层基础
Week 2: Service 层完善 + API 层
Week 3: 测试 + 文档
```

**总计**: 14-19 天

---

## 相关资源

### 项目文档
- [INIT-PRD.md](../INIT-PRD.md) - 项目需求文档
- [AGENTS.md](../../AGENTS.md) - 开发指南

### 参考实现
- [vibe-kanban Repository 管理](https://deepwiki.com/BloopAI/vibe-kanban)

### 技术栈
- SeaORM 0.12 - ORM 框架
- Axum 0.7 - Web 框架
- SQLite/PostgreSQL - 数据库
- utoipa 4.x - OpenAPI 文档

---

## 版本历史

| 版本 | 日期 | 变更说明 | 作者 |
|------|------|---------|------|
| v1.0 | 2026-01-17 | 初始版本 | AI Assistant |

---

## 联系方式

如有问题或建议，请：
- 创建 GitHub Issue
- 联系项目维护者

---

**文档索引结束**
