# Webhook功能开发 - 快速恢复指南

**用途**: 下次会话快速恢复上下文

---

## 🚀 快速启动

### 当前状态
- **分支**: main
- **进度**: 8/40 任务完成 (20%)
- **测试**: 178+ 通过，0失败
- **下一个任务**: Task 3.3 - 实现payload解析

### 立即运行
```bash
cd backend
cargo test  # 验证所有测试通过
cargo clippy  # 验证无警告
git log --oneline -10  # 查看最近提交
```

---

## 📋 已完成的功能

### 数据库层 ✅
- `webhook_configs` 表（10字段，3索引，2外键）
- `WebhookConfig` 实体（SeaORM）
- `repositories.webhook_status` 字段

### 抽象层 ✅
- `WebhookEvent` 枚举（4种事件）
- `CreateWebhookRequest` 和 `GitWebhook` 模型
- `GitProvider` trait扩展（3个webhook方法）

### 实现层 ✅
- Gitea webhook管理（create/delete/list）
- Webhook API端点（POST /api/webhooks/:provider_id）
- 签名验证（HMAC-SHA256）

---

## 🎯 下一步任务

### Task 3.3: 实现payload解析 (下一个)

**目标**: 解析来自不同平台的webhook payload

**需要实现**:
1. Gitea IssueCommentPayload 模型
2. GitHub IssueCommentEvent 模型（未来）
3. GitLab NoteEvent 模型（未来）
4. 统一的CommentInfo提取器

**文件**:
- `backend/src/api/webhooks/models.rs` - 添加payload模型
- `backend/src/api/webhooks/handlers.rs` - 集成payload解析
- `backend/tests/webhook_payload_tests.rs` - payload解析测试

**参考**:
- Gitea payload格式：见DeepWiki查询结果
- 已有的统一模型：`git_provider/models.rs`

### Task 3.4: 实现事件路由和处理

**目标**: 根据事件类型路由到不同的处理器

**需要实现**:
1. 事件类型检测（从header或payload）
2. @mention快速检测
3. 异步任务投递（tokio::spawn）
4. 事件日志记录

### Task 3.5: 错误处理和日志记录

**目标**: 完善错误处理和日志系统

**需要实现**:
1. 统一的错误响应格式
2. 结构化日志（tracing）
3. 错误分类（4xx vs 5xx）
4. 监控指标（可选）

---

## 🔧 开发环境

### 必需的环境变量
```bash
# 在 .env 文件中
DATABASE_URL=sqlite:./data/gitautodev/db/gitautodev.db?mode=rwc
DATABASE_MAX_CONNECTIONS=10
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug

# Webhook配置（待添加）
WEBHOOK_DOMAIN=https://gitautodev.example.com
WEBHOOK_SECRET_KEY=your-secret-key-for-signing
CONTEXT_RETENTION_DAYS=30
```

### 常用命令
```bash
# 测试
cargo test                          # 所有测试
cargo test webhook                  # webhook相关测试
cargo test -- --nocapture          # 显示输出

# 代码质量
cargo clippy                        # 检查警告
cargo fmt                           # 格式化代码

# 构建
cargo build                         # 开发构建
cargo build --release              # 生产构建

# 运行
cargo run                           # 启动服务器
```

---

## 📁 关键文件位置

### 数据库
- 迁移: `backend/src/migration/m20260117_*.rs`
- 实体: `backend/src/entities/webhook_config.rs`

### GitProvider
- Trait: `backend/src/git_provider/traits.rs`
- 模型: `backend/src/git_provider/models.rs`
- Gitea: `backend/src/git_provider/gitea/client.rs`

### Webhook API
- 模块: `backend/src/api/webhooks/`
- 路由: `backend/src/api/webhooks/routes.rs`
- 处理器: `backend/src/api/webhooks/handlers.rs`
- 验证: `backend/src/api/webhooks/verification.rs`

### 测试
- 迁移: `backend/tests/webhook_migration_tests.rs`
- 实体: `backend/tests/webhook_entity_tests.rs`
- API: `backend/tests/webhook_api_tests.rs`
- 验证: `backend/tests/webhook_verification_tests.rs`

---

## 💡 实现提示

### Payload解析（Task 3.3）
```rust
// 参考现有的Gitea模型转换
// 见 git_provider/gitea/models.rs 中的 From trait实现

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaIssueCommentPayload {
    pub action: String,  // "created", "edited", "deleted"
    pub issue: GiteaIssue,
    pub comment: GiteaComment,
    pub repository: GiteaRepository,
    pub sender: GiteaUser,
}

// 提取统一的评论信息
impl GiteaIssueCommentPayload {
    pub fn extract_comment_info(&self) -> CommentInfo {
        CommentInfo {
            comment_id: self.comment.id.to_string(),
            comment_body: self.comment.body.clone(),
            comment_author: self.comment.user.login.clone(),
            issue_number: self.issue.number,
            repository_full_name: self.repository.full_name.clone(),
            action: self.action.clone(),
        }
    }
}
```

### @Mention检测（Task 3.4预览）
```rust
pub fn detect_mention(comment_body: &str, username: &str) -> bool {
    let patterns = [
        format!("@{}", username),
        format!("@{} ", username),
        format!("@{}\n", username),
    ];
    patterns.iter().any(|p| comment_body.contains(p))
}
```

---

## 🐛 已知问题和解决方案

### 问题1: Crate名称混淆
**症状**: 导入路径使用`gitautodev_backend`失败  
**原因**: Crate名称是`gitautodev`，不是`gitautodev_backend`  
**解决**: 使用`use gitautodev::*`

### 问题2: 测试超时
**症状**: `cargo test`运行超过3分钟  
**原因**: 集成测试较多  
**解决**: 使用`cargo test --lib`只运行单元测试

### 问题3: 规范vs质量冲突
**症状**: 规范审查要求删除索引，质量审查要求添加索引  
**原因**: 初始规范过于严格  
**解决**: 优先采纳代码质量建议，更新规范

---

## 📞 联系信息

**项目**: GitAutoDev  
**仓库**: code-agent-platform-2  
**文档**: docs/plans/  
**问题跟踪**: 使用Git issues

---

**最后更新**: 2026-01-17  
**下次会话**: 从Task 3.3开始
