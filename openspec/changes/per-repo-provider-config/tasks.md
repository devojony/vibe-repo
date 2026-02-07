# Implementation Tasks

## 1. 数据库 Schema 变更

- [ ] 1.1 创建新的 migration 文件 `m20260207_per_repo_provider.rs`
- [ ] 1.2 在 migration 中删除 `webhook_configs` 表
- [ ] 1.3 在 migration 中删除 `repo_providers` 表
- [ ] 1.4 在 migration 中删除旧的 `repositories` 表
- [ ] 1.5 在 migration 中创建新的 `repositories` 表，包含字段：provider_type, provider_base_url, access_token, webhook_secret
- [ ] 1.6 在干净的数据库上测试 migration
- [ ] 1.7 验证所有表结构正确创建

## 2. 实体层更新

- [ ] 2.1 更新 `backend/src/entities/repository.rs` 添加 provider_type 字段
- [ ] 2.2 更新 `backend/src/entities/repository.rs` 添加 provider_base_url 字段
- [ ] 2.3 更新 `backend/src/entities/repository.rs` 添加 access_token 字段
- [ ] 2.4 更新 `backend/src/entities/repository.rs` 添加 webhook_secret 字段
- [ ] 2.5 更新 `backend/src/entities/repository.rs` 移除 provider_id 字段
- [ ] 2.6 更新 `backend/src/entities/repository.rs` 移除 RepoProvider 关系
- [ ] 2.7 删除文件 `backend/src/entities/repo_provider.rs`
- [ ] 2.8 删除文件 `backend/src/entities/webhook_config.rs`
- [ ] 2.9 更新 `backend/src/entities/mod.rs` 移除已删除的实体导出
- [ ] 2.10 运行 `cargo check` 找出所有编译错误

## 3. GitClientFactory 重构

- [ ] 3.1 在 `backend/src/git_provider/factory.rs` 中添加 `from_repository()` 方法
- [ ] 3.2 更新 `backend/src/services/repository_service.rs` 中的 GitClientFactory 调用（4 处）
- [ ] 3.3 更新 `backend/src/services/pr_creation_service.rs` 中的 GitClientFactory 调用（1 处）
- [ ] 3.4 更新 `backend/src/services/issue_closure_service.rs` 中的 GitClientFactory 调用（1 处）
- [ ] 3.5 更新 `backend/src/api/repositories/handlers.rs` 中的 GitClientFactory 调用（2 处）
- [ ] 3.6 更新 `backend/src/api/webhooks/handlers.rs` 中的 GitClientFactory 调用（1 处）
- [ ] 3.7 更新所有测试文件中的 GitClientFactory 调用（约 21 处）
- [ ] 3.8 删除或标记为废弃 `from_provider()` 方法
- [ ] 3.9 运行 `cargo check` 验证所有调用点已更新

## 4. RepositoryService 重构

- [ ] 4.1 删除 `sync_all_providers()` 方法
- [ ] 4.2 删除 `process_provider()` 方法
- [ ] 4.3 删除 `store_repository()` 方法
- [ ] 4.4 添加 `add_repository()` 方法实现 token 验证
- [ ] 4.5 在 `add_repository()` 中实现从 provider 获取仓库信息
- [ ] 4.6 在 `add_repository()` 中实现权限验证（branches, labels, PRs, issues, webhooks）
- [ ] 4.7 在 `add_repository()` 中实现生成随机 webhook_secret
- [ ] 4.8 在 `add_repository()` 中实现存储仓库记录
- [ ] 4.9 在 `add_repository()` 中实现创建 workspace 和 agent
- [ ] 4.10 在 `add_repository()` 中实现初始化分支和标签
- [ ] 4.11 在 `add_repository()` 中实现创建 webhook
- [ ] 4.12 更新 `initialize_repository()` 方法移除 provider 查询
- [ ] 4.13 更新 `create_webhook_for_repository()` 方法使用仓库字段
- [ ] 4.14 更新 `validate_repository()` 方法移除 provider 依赖
- [ ] 4.15 更新所有其他方法移除 provider 查询（约 4 处）

## 5. 其他服务更新

- [ ] 5.1 更新 `backend/src/services/pr_creation_service.rs` 移除 provider 查询
- [ ] 5.2 更新 `backend/src/services/pr_creation_service.rs` 使用 repository 字段创建 GitClient
- [ ] 5.3 更新 `backend/src/services/issue_closure_service.rs` 移除 provider 查询
- [ ] 5.4 更新 `backend/src/services/issue_closure_service.rs` 使用 repository 字段创建 GitClient

## 6. API Handler 更新

- [ ] 6.1 创建 `AddRepositoryRequest` 结构体（provider_type, provider_base_url, access_token, full_name, branch_name）
- [ ] 6.2 创建 `add_repository()` handler 实现
- [ ] 6.3 在 `add_repository()` handler 中调用 RepositoryService::add_repository()
- [ ] 6.4 在 `add_repository()` handler 中处理错误（400, 401, 403, 404）
- [ ] 6.5 在 `add_repository()` handler 中返回 201 Created 和完整仓库详情
- [ ] 6.6 删除 `batch_initialize_repositories()` handler
- [ ] 6.7 删除 `batch_archive_repositories()` handler（按 provider_id）
- [ ] 6.8 删除 `batch_delete_repositories()` handler（按 provider_id）
- [ ] 6.9 删除 `batch_refresh_repositories()` handler（按 provider_id）
- [ ] 6.10 删除 `batch_reinitialize_repositories()` handler（按 provider_id）
- [ ] 6.11 更新 `list_repositories()` handler 移除 provider_id 过滤参数
- [ ] 6.12 更新 `RepositoryResponse` 结构体添加 provider_type 和 provider_base_url 字段
- [ ] 6.13 更新 `RepositoryResponse` 结构体移除 provider_id 字段
- [ ] 6.14 确保 `RepositoryResponse` 不包含 access_token 和 webhook_secret（安全）
- [ ] 6.15 更新 `backend/src/api/repositories/routes.rs` 添加新的 POST / 路由
- [ ] 6.16 更新 `backend/src/api/repositories/routes.rs` 移除已删除的 handler 路由

## 7. Webhook Handler 更新

- [ ] 7.1 更新 `backend/src/api/webhooks/handlers.rs` 移除 webhook_configs 表查询
- [ ] 7.2 更新 webhook 验证逻辑使用 repository.webhook_secret
- [ ] 7.3 更新 webhook 验证逻辑从 repository.provider_type 确定签名算法
- [ ] 7.4 确保 webhook 验证只需要单次数据库查询（repository）
- [ ] 7.5 更新 webhook 处理逻辑移除 provider 查询

## 8. 测试工具更新

- [ ] 8.1 更新 `backend/src/test_utils/db.rs` 移除 `create_test_provider()` 函数
- [ ] 8.2 创建新的 `create_test_repository()` 函数，包含完整 provider 配置
- [ ] 8.3 更新 `create_test_repository()` 生成随机 webhook_secret
- [ ] 8.4 更新所有测试辅助函数使用新的 repository 模型

## 9. 单元测试更新

- [ ] 9.1 更新 `backend/src/services/repository_service.rs` 中的测试
- [ ] 9.2 更新 `backend/src/services/pr_creation_service.rs` 中的测试
- [ ] 9.3 更新 `backend/src/services/issue_closure_service.rs` 中的测试
- [ ] 9.4 更新 `backend/src/api/repositories/handlers.rs` 中的测试
- [ ] 9.5 更新 `backend/src/api/webhooks/handlers.rs` 中的测试
- [ ] 9.6 更新 `backend/src/git_provider/factory.rs` 中的测试
- [ ] 9.7 删除所有 provider sync 相关的测试
- [ ] 9.8 删除所有 webhook_configs 相关的测试

## 10. 集成测试更新

- [ ] 10.1 更新 `backend/tests/repositories/` 中的所有测试
- [ ] 10.2 更新 `backend/tests/repositories/repository_sync_property_tests.rs` 或删除（如果是 sync 测试）
- [ ] 10.3 添加新的集成测试：测试 POST /api/repositories 端点
- [ ] 10.4 添加新的集成测试：测试 token 验证失败场景
- [ ] 10.5 添加新的集成测试：测试仓库不存在场景
- [ ] 10.6 添加新的集成测试：测试权限不足场景
- [ ] 10.7 添加新的集成测试：测试原子操作（部分失败时回滚）
- [ ] 10.8 更新所有现有集成测试使用新的 repository 模型

## 11. OpenAPI 文档更新

- [ ] 11.1 更新 `AddRepositoryRequest` 的 OpenAPI schema 定义
- [ ] 11.2 更新 `RepositoryResponse` 的 OpenAPI schema 定义
- [ ] 11.3 添加 POST /api/repositories 的 OpenAPI 路径定义
- [ ] 11.4 移除已删除端点的 OpenAPI 定义
- [ ] 11.5 更新 OpenAPI 文档中的示例请求和响应
- [ ] 11.6 验证 Swagger UI 正确显示新的 API

## 12. 文档更新

- [ ] 12.1 更新 `docs/api/api-reference.md` 添加 POST /api/repositories 文档
- [ ] 12.2 更新 `docs/api/api-reference.md` 移除已删除的端点文档
- [ ] 12.3 更新 `docs/api/user-guide.md` 说明新的仓库添加流程
- [ ] 12.4 更新 `docs/database/schema.md` 反映新的数据库结构
- [ ] 12.5 更新 `AGENTS.md` 移除 provider 相关的说明
- [ ] 12.6 创建升级指南文档说明破坏性变更
- [ ] 12.7 更新 README.md（如果有 provider 相关内容）

## 13. 最终验证

- [ ] 13.1 运行完整测试套件：`cargo test`
- [ ] 13.2 运行 clippy 检查：`cargo clippy`
- [ ] 13.3 运行格式检查：`cargo fmt --check`
- [ ] 13.4 在干净的数据库上启动应用验证 migration
- [ ] 13.5 手动测试：通过 API 添加一个仓库
- [ ] 13.6 手动测试：验证 webhook 接收和处理
- [ ] 13.7 手动测试：验证 PR 创建功能
- [ ] 13.8 手动测试：验证 Issue 关闭功能
- [ ] 13.9 检查日志确保没有错误
- [ ] 13.10 验证 Swagger UI 文档正确显示

## 14. 清理和优化

- [ ] 14.1 删除所有未使用的 provider 相关代码
- [ ] 14.2 删除所有未使用的 webhook_config 相关代码
- [ ] 14.3 清理导入语句移除已删除实体的引用
- [ ] 14.4 更新错误消息移除 provider 相关引用
- [ ] 14.5 检查并更新日志语句
- [ ] 14.6 运行 `cargo clean` 和 `cargo build --release` 验证发布构建
