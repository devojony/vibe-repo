# Repository 管理功能实施路线图

**版本**: v1.0  
**日期**: 2026-01-17  
**预计工期**: 14-19 天  
**状态**: 待开始

## 概述

本文档描述了 Repository 管理功能的详细实施计划，包括时间线、里程碑、任务分解和验收标准。

---

## 时间线总览

```
Week 1: 数据库层 + Service 层基础
├─ Day 1-2: 数据库迁移 + Entity 更新
├─ Day 3-4: Service 层核心方法
└─ Day 5: Service 层单元测试

Week 2: Service 层完善 + API 层
├─ Day 1: 批量操作实现
├─ Day 2-3: API Handlers（单个操作）
├─ Day 4: API Handlers（批量操作）
└─ Day 5: 路由更新 + OpenAPI

Week 3: 测试 + 文档
├─ Day 1-2: 集成测试
├─ Day 3: Property 测试 + E2E 测试
├─ Day 4: 文档更新
└─ Day 5: 代码审查 + 优化
```

---

## Phase 1: 数据库层 (3-4 天)

### 任务 1.1: 创建数据库迁移 (1 天)

**文件**: `backend/src/migration/m20250117_000001_add_repository_status_and_soft_delete.rs`

**任务清单**:
- [ ] 创建迁移文件
- [ ] 添加 `status` 字段 (TEXT, NOT NULL, DEFAULT 'uninitialized')
- [ ] 添加 `has_workspace` 字段 (BOOLEAN, NOT NULL, DEFAULT FALSE)
- [ ] 添加 `deleted_at` 字段 (TIMESTAMP, NULL)
- [ ] 创建索引 (`idx_repositories_status`, `idx_repositories_deleted_at`, `idx_repositories_has_workspace`)
- [ ] 实现数据迁移逻辑（设置现有记录的初始状态）
- [ ] 实现 `down()` 回滚方法
- [ ] 在 `mod.rs` 中注册迁移

**验收标准**:
- ✅ 迁移成功执行（`cargo run`）
- ✅ 所有字段和索引创建成功
- ✅ 现有数据正确迁移
- ✅ 回滚功能正常工作

### 任务 1.2: 更新 Entity 定义 (0.5 天)

**文件**: `backend/src/entities/repository.rs`

**任务清单**:
- [ ] 添加 `RepositoryStatus` 枚举
- [ ] 在 `Model` 中添加新字段
- [ ] 实现辅助方法：`is_deleted()`, `can_delete()`, `can_archive()`
- [ ] 更新文档注释

**验收标准**:
- ✅ 编译通过（`cargo check`）
- ✅ 所有辅助方法测试通过

### 任务 1.3: 编写迁移测试 (0.5 天)

**文件**: `backend/tests/migration_validation_tests.rs`

**任务清单**:
- [ ] 测试迁移 up 成功
- [ ] 测试迁移 down 成功
- [ ] 测试数据迁移正确性
- [ ] 测试索引创建

**验收标准**:
- ✅ 所有测试通过（`cargo test --test migration_validation_tests`）

---

## Phase 2: Service 层 (4-5 天)

### 任务 2.1: 实现核心 Service 方法 (2 天)

**文件**: `backend/src/services/repository_service.rs`

**新增方法**:
- [ ] `archive_repository(repo_id)` - 归档仓库
- [ ] `unarchive_repository(repo_id)` - 取消归档
- [ ] `delete_repository(repo_id)` - 软删除
- [ ] `restore_repository(repo_id)` - 恢复软删除
- [ ] `update_repository_metadata(repo_id, name)` - 更新元数据

**修改现有方法**:
- [ ] `store_repository()` - 支持软删除恢复和归档过滤
- [ ] `initialize_repository()` - 更新状态为 Idle/Unavailable

**验收标准**:
- ✅ 所有方法实现完成
- ✅ 编译通过
- ✅ 业务逻辑正确

### 任务 2.2: 实现批量操作 (1 天)

**文件**: `backend/src/services/repository_service.rs`

**新增方法**:
- [ ] `batch_archive(repo_ids)` - 批量归档
- [ ] `batch_delete(repo_ids)` - 批量删除
- [ ] `batch_refresh(repo_ids)` - 批量刷新
- [ ] `batch_reinitialize(repo_ids, branch_name)` - 批量重新初始化

**验收标准**:
- ✅ 返回 `BatchOperationResponse`
- ✅ 部分成功模式正确实现
- ✅ 错误处理完善

### 任务 2.3: Service 层单元测试 (1 天)

**文件**: `backend/src/services/repository_service.rs` (tests 模块)

**测试用例**:
- [ ] 归档操作测试 (5个用例)
- [ ] 取消归档测试 (3个用例)
- [ ] 软删除测试 (3个用例)
- [ ] 同步逻辑测试 (3个用例)
- [ ] 批量操作测试 (4个用例)

**验收标准**:
- ✅ 至少 18 个单元测试
- ✅ 所有测试通过
- ✅ 覆盖率 > 80%

### 任务 2.4: 添加响应模型 (0.5 天)

**文件**: `backend/src/api/repositories/models.rs`

**新增模型**:
- [ ] `BatchOperationResponse`
- [ ] `BatchOperationResult`
- [ ] `UpdateRepositoryRequest`
- [ ] `BatchOperationRequest`

**验收标准**:
- ✅ 所有模型实现 `Serialize`, `Deserialize`, `ToSchema`
- ✅ 编译通过

---

## Phase 3: API 层 (3-4 天)

### 任务 3.1: 实现单个操作 Handlers (2 天)

**文件**: `backend/src/api/repositories/handlers.rs`

**新增 handlers**:
- [ ] `update_repository()` - PATCH /api/repositories/:id
- [ ] `archive_repository()` - POST /api/repositories/:id/archive
- [ ] `unarchive_repository()` - POST /api/repositories/:id/unarchive
- [ ] `delete_repository()` - DELETE /api/repositories/:id
- [ ] `reinitialize_repository()` - POST /api/repositories/:id/reinitialize

**修改现有 handlers**:
- [ ] `list_repositories()` - 支持新的过滤条件

**验收标准**:
- ✅ 所有 handlers 实现完成
- ✅ OpenAPI 文档完整
- ✅ 错误处理正确

### 任务 3.2: 实现批量操作 Handlers (1 天)

**文件**: `backend/src/api/repositories/handlers.rs`

**新增 handlers**:
- [ ] `batch_archive_repositories()` - POST /api/repositories/batch-archive
- [ ] `batch_delete_repositories()` - POST /api/repositories/batch-delete
- [ ] `batch_refresh_repositories()` - POST /api/repositories/batch-refresh
- [ ] `batch_reinitialize_repositories()` - POST /api/repositories/batch-reinitialize

**验收标准**:
- ✅ 所有批量操作实现完成
- ✅ 返回详细的操作结果

### 任务 3.3: 更新路由 (0.5 天)

**文件**: `backend/src/api/repositories/routes.rs`

**任务清单**:
- [ ] 注册所有新的单个操作路由
- [ ] 注册所有批量操作路由
- [ ] 更新 OpenAPI 文档

**验收标准**:
- ✅ 所有路由正确注册
- ✅ Swagger UI 显示所有端点

---

## Phase 4: 测试 (3-4 天)

### 任务 4.1: 集成测试 (2 天)

**文件**: `backend/tests/repository_management_integration_tests.rs`

**测试分类**:
- [ ] 归档操作测试 (4个用例)
- [ ] 取消归档测试 (2个用例)
- [ ] 删除操作测试 (3个用例)
- [ ] 更新操作测试 (2个用例)
- [ ] 重新初始化测试 (2个用例)
- [ ] 列表查询测试 (6个用例)
- [ ] 批量操作测试 (4个用例)
- [ ] 同步场景测试 (3个用例)

**验收标准**:
- ✅ 至少 26 个集成测试
- ✅ 所有测试通过
- ✅ 覆盖所有 API 端点

### 任务 4.2: Property 测试 (0.5 天)

**文件**: `backend/tests/repository_management_property_tests.rs`

**测试用例**:
- [ ] 幂等性测试 (2个)
- [ ] 状态转换测试 (1个)
- [ ] 查询过滤测试 (1个)
- [ ] 批量操作测试 (1个)

**验收标准**:
- ✅ 至少 5 个 property 测试
- ✅ 所有测试通过

### 任务 4.3: E2E 测试 (0.5 天)

**文件**: `backend/tests/repository_lifecycle_e2e_tests.rs`

**测试场景**:
- [ ] 完整生命周期测试
- [ ] 同步场景测试
- [ ] 批量操作场景测试

**验收标准**:
- ✅ 至少 3 个 E2E 测试
- ✅ 所有测试通过

---

## Phase 5: 文档和部署 (1-2 天)

### 任务 5.1: 更新 OpenAPI 文档 (0.5 天)

**文件**: `backend/src/api/mod.rs`

**任务清单**:
- [ ] 注册所有新的 handlers 到 OpenAPI
- [ ] 更新 RepositoryResponse schema
- [ ] 添加新的请求/响应模型

**验收标准**:
- ✅ Swagger UI 完整显示所有端点
- ✅ 所有模型正确显示

### 任务 5.2: 更新项目文档 (0.5 天)

**文件**: `AGENTS.md`, `README.md`

**任务清单**:
- [ ] 更新 API 端点列表
- [ ] 添加 Repository 状态机说明
- [ ] 添加软删除和归档的说明

**验收标准**:
- ✅ 文档清晰完整
- ✅ 示例代码正确

### 任务 5.3: 代码审查和优化 (0.5 天)

**任务清单**:
- [ ] 运行 `cargo clippy`
- [ ] 运行 `cargo fmt`
- [ ] 检查测试覆盖率
- [ ] 性能测试

**验收标准**:
- ✅ 无 clippy 警告
- ✅ 代码格式正确
- ✅ 测试覆盖率 > 70%

---

## 里程碑

| 里程碑 | 完成标准 | 预计时间 |
|--------|---------|---------|
| **M1: 数据库就绪** | 迁移成功，Entity 更新，测试通过 | Day 2 |
| **M2: Service 层完成** | 所有方法实现，单元测试通过 | Day 7 |
| **M3: API 层完成** | 所有端点实现，路由注册 | Day 12 |
| **M4: 测试完成** | 所有测试通过，覆盖率达标 | Day 17 |
| **M5: 生产就绪** | 文档完善，代码审查通过 | Day 19 |

---

## 风险管理

### 高风险项

1. **数据迁移**
   - 风险：现有数据迁移可能失败
   - 缓解：充分测试，准备回滚方案

2. **状态不一致**
   - 风险：has_workspace 字段可能不同步
   - 缓解：严格的更新逻辑，一致性检查工具

### 中风险项

1. **性能影响**
   - 风险：批量操作可能影响性能
   - 缓解：限制批量数量，考虑异步处理

2. **同步冲突**
   - 风险：软删除恢复可能困惑用户
   - 缓解：清晰的日志和 UI 提示

---

## 验收标准

### 功能验收

- ✅ 所有 API 端点正常工作
- ✅ 状态转换逻辑正确
- ✅ 软删除和恢复功能正常
- ✅ 归档和取消归档功能正常
- ✅ 批量操作返回正确结果
- ✅ 同步逻辑正确处理归档和删除

### 质量验收

- ✅ 所有测试通过（单元 + 集成 + Property + E2E）
- ✅ 测试覆盖率 > 70%
- ✅ 无 clippy 警告
- ✅ 代码格式正确
- ✅ 文档完整

### 性能验收

- ✅ 单个操作响应时间 < 100ms
- ✅ 批量操作（10个）响应时间 < 1s
- ✅ 列表查询响应时间 < 200ms

---

## 下一步行动

1. **创建 Git 分支**
   ```bash
   git checkout -b feature/repository-management
   ```

2. **开始 Phase 1**
   - 创建数据库迁移文件
   - 更新 Entity 定义
   - 编写迁移测试

3. **每日站会**
   - 回顾昨天完成的任务
   - 计划今天的任务
   - 识别阻塞问题

---

**文档结束**
