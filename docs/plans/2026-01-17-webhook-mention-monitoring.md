# Webhook @Mention监控功能实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 实现跨Git平台（Gitea/GitHub/GitLab）的webhook接收和@mention监控功能，当仓库开发者在评论中@提及provider对应的用户时，自动创建Task工作流进行响应。

**架构:** 
- Webhook接收层：统一的HTTP端点接收来自不同Git平台的webhook事件
- 业务逻辑层：@mention检测、上下文收集、Task创建
- 数据持久层：混合存储策略（数据库存储元数据，文件系统存储大文本上下文）
- 跨平台抽象：统一的GitProvider trait扩展，隐藏平台差异

**技术栈:** 
- Rust (Axum, SeaORM, Tokio)
- HMAC-SHA256签名验证
- JSON文件存储（分层目录结构）
- 后台异步任务处理

**环境配置:**
```bash
# 在 .env 文件中添加
WEBHOOK_DOMAIN=https://gitautodev.example.com
WEBHOOK_SECRET_KEY=your-secret-key-for-signing
CONTEXT_RETENTION_DAYS=30
```

---

## 实现概览

本计划分为8个阶段，共约40个任务：

1. **数据库Schema** (3个任务) - webhook_configs表、实体模型、repository字段
2. **GitProvider扩展** (3个任务) - Webhook模型、trait扩展、Gitea实现
3. **Webhook接收端点** (5个任务) - API模块、签名验证、payload解析
4. **仓库初始化集成** (3个任务) - 自动创建webhook、错误处理、清理机制
5. **@Mention检测** (4个任务) - 检测器、上下文收集、文件存储
6. **Task工作流集成** (4个任务) - Workspace实体、Task创建、上下文管理
7. **后台服务** (3个任务) - 上下文清理服务、健康检查
8. **测试和文档** (5个任务) - 集成测试、属性测试、API文档

预计总工作量：2-3天

---

## 阶段1：数据库Schema和实体模型

### Task 1.1: 创建webhook_configs表迁移

**目标:** 创建webhook配置表用于存储webhook元数据

**文件:**
- Create: `backend/src/migration/m20260117_000002_create_webhook_configs.rs`
- Modify: `backend/src/migration/mod.rs`
- Create: `backend/tests/webhook_migration_tests.rs`

**依赖:** 无

**Step 1: 编写迁移测试**

创建测试文件验证表结构：

```rust
// backend/tests/webhook_migration_tests.rs
use gitautodev_backend::test_utils::db::setup_test_db;

#[tokio::test]
async fn test_webhook_configs_table_exists() {
    let db = setup_test_db().await.expect("Failed to setup test db");
    
    // 验证表存在且可以查询
    let result = sqlx::query("SELECT * FROM webhook_configs LIMIT 1")
        .fetch_optional(db.get_connection())
        .await;
    
    assert!(result.is_ok(), "webhook_configs table should exist");
}

#[tokio::test]
async fn test_webhook_configs_has_required_columns() {
    let db = setup_test_db().await.expect("Failed to setup test db");
    
    // 验证所有必需列存在
    let result = sqlx::query(
        "SELECT id, provider_id, repository_id, webhook_id, webhook_secret, 
         webhook_url, events, enabled, created_at FROM webhook_configs LIMIT 1"
    )
    .fetch_optional(db.get_connection())
    .await;
    
    assert!(result.is_ok(), "All required columns should exist");
}
```

**Step 2: 运行测试验证失败**

```bash
cd backend
cargo test webhook_migration_tests -- --nocapture
```

预期输出: 
```
FAILED - no such table: webhook_configs
```

**Step 3: 创建迁移文件**

```rust
// backend/src/migration/m20260117_000002_create_webhook_configs.rs
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebhookConfigs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WebhookConfigs::ProviderId).integer().not_null())
                    .col(ColumnDef::new(WebhookConfigs::RepositoryId).integer().not_null())
                    .col(ColumnDef::new(WebhookConfigs::WebhookId).string().not_null())
                    .col(ColumnDef::new(WebhookConfigs::WebhookSecret).string().not_null())
                    .col(ColumnDef::new(WebhookConfigs::WebhookUrl).string().not_null())
                    .col(ColumnDef::new(WebhookConfigs::Events).string().not_null())
                    .col(
                        ColumnDef::new(WebhookConfigs::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(WebhookConfigs::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_provider")
                            .from(WebhookConfigs::Table, WebhookConfigs::ProviderId)
                            .to(RepoProviders::Table, RepoProviders::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_repository")
                            .from(WebhookConfigs::Table, WebhookConfigs::RepositoryId)
                            .to(Repositories::Table, Repositories::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WebhookConfigs::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum WebhookConfigs {
    Table,
    Id,
    ProviderId,
    RepositoryId,
    WebhookId,
    WebhookSecret,
    WebhookUrl,
    Events,
    Enabled,
    CreatedAt,
}

#[derive(Iden)]
enum RepoProviders {
    Table,
    Id,
}

#[derive(Iden)]
enum Repositories {
    Table,
    Id,
}
```

**Step 4: 注册迁移**

```rust
// 在 backend/src/migration/mod.rs 顶部添加
pub use m20260117_000002_create_webhook_configs::Migration as M20260117000002;

// 在 impl MigratorTrait for Migrator 的 migrations() 方法中添加
Box::new(M20260117000002),
```

**Step 5: 运行测试验证通过**

```bash
cargo test webhook_migration_tests -- --nocapture
```

预期输出:
```
test test_webhook_configs_table_exists ... ok
test test_webhook_configs_has_required_columns ... ok
```

**Step 6: 提交**

```bash
git add backend/src/migration/m20260117_000002_create_webhook_configs.rs
git add backend/src/migration/mod.rs
git add backend/tests/webhook_migration_tests.rs
git commit -m "feat(db): add webhook_configs table migration

- Create webhook_configs table with foreign keys
- Add cascade delete for provider and repository
- Include tests for table structure"
```

---

### Task 1.2: 创建webhook_config实体

**目标:** 创建SeaORM实体模型用于操作webhook_configs表

**文件:**
- Create: `backend/src/entities/webhook_config.rs`
- Modify: `backend/src/entities/mod.rs`
- Modify: `backend/src/entities/prelude.rs`
- Create: `backend/tests/webhook_entity_tests.rs`

**依赖:** Task 1.1

**Step 1: 编写实体测试**

```rust
// backend/tests/webhook_entity_tests.rs
use chrono::Utc;
use gitautodev_backend::{
    entities::{prelude::*, webhook_config},
    test_utils::db::setup_test_db,
};
use sea_orm::{ActiveModelTrait, Set};

#[tokio::test]
async fn test_create_webhook_config() {
    let db = setup_test_db().await.expect("Failed to setup test db");
    
    // 创建测试provider和repository
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    
    let webhook = webhook_config::ActiveModel {
        provider_id: Set(provider.id),
        repository_id: Set(repo.id),
        webhook_id: Set("123".to_string()),
        webhook_secret: Set("secret123".to_string()),
        webhook_url: Set("https://example.com/webhook/1".to_string()),
        events: Set(r#"["issue_comment","pull_request_comment"]"#.to_string()),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    
    let result = webhook.insert(db.get_connection()).await;
    assert!(result.is_ok());
    
    let saved = result.unwrap();
    assert_eq!(saved.provider_id, provider.id);
    assert_eq!(saved.webhook_id, "123");
}

#[tokio::test]
async fn test_webhook_config_cascade_delete_with_provider() {
    let db = setup_test_db().await.expect("Failed to setup test db");
    
    let provider = create_test_provider(&db).await;
    let repo = create_test_repository(&db, provider.id).await;
    let webhook = create_test_webhook(&db, provider.id, repo.id).await;
    
    // 删除provider应该级联删除webhook
    provider.delete(db.get_connection()).await.unwrap();
    
    let found = WebhookConfig::find_by_id(webhook.id)
        .one(db.get_connection())
        .await
        .unwrap();
    
    assert!(found.is_none(), "Webhook should be deleted when provider is deleted");
}

// 辅助函数
async fn create_test_provider(db: &DatabasePool) -> repo_provider::Model {
    // 实现创建测试provider的逻辑
}

async fn create_test_repository(db: &DatabasePool, provider_id: i32) -> repository::Model {
    // 实现创建测试repository的逻辑
}

async fn create_test_webhook(
    db: &DatabasePool,
    provider_id: i32,
    repo_id: i32,
) -> webhook_config::Model {
    // 实现创建测试webhook的逻辑
}
```

**Step 2: 运行测试验证失败**

```bash
cargo test webhook_entity_tests -- --nocapture
```

预期输出:
```
FAILED - webhook_config module not found
```

**Step 3: 创建实体文件**

```rust
// backend/src/entities/webhook_config.rs
//! WebhookConfig entity
//!
//! Represents webhook configuration for repository event monitoring.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub provider_id: i32,
    pub repository_id: i32,
    pub webhook_id: String,
    pub webhook_secret: String,
    pub webhook_url: String,
    pub events: String,  // JSON array of event types
    pub enabled: bool,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::repo_provider::Entity",
        from = "Column::ProviderId",
        to = "super::repo_provider::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    RepoProvider,
    #[sea_orm(
        belongs_to = "super::repository::Entity",
        from = "Column::RepositoryId",
        to = "super::repository::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Repository,
}

impl Related<super::repo_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RepoProvider.def()
    }
}

impl Related<super::repository::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Repository.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

**Step 4: 更新模块导出**

```rust
// 在 backend/src/entities/mod.rs 添加
pub mod webhook_config;

// 在 backend/src/entities/prelude.rs 添加
pub use super::webhook_config::{Entity as WebhookConfig, Model as WebhookConfigModel};
```

**Step 5: 运行测试验证通过**

```bash
cargo test webhook_entity_tests -- --nocapture
```

预期输出:
```
test test_create_webhook_config ... ok
test test_webhook_config_cascade_delete_with_provider ... ok
```

**Step 6: 提交**

```bash
git add backend/src/entities/webhook_config.rs
git add backend/src/entities/mod.rs
git add backend/src/entities/prelude.rs
git add backend/tests/webhook_entity_tests.rs
git commit -m "feat(entities): add webhook_config entity

- Create WebhookConfig entity with relations
- Add cascade delete support
- Include entity tests"
```

---

由于完整计划非常长（约40个任务），我将计划文档保存到文件中。现在让我完成保存：

