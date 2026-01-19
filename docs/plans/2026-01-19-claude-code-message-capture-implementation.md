# Claude Code 消息捕获实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 Claude Code 的完整消息历史捕获功能，包括实时流式传输和数据库持久化

**Architecture:**
- 使用 MsgStore 作为内存中的消息队列（100MB 循环缓冲区 + Tokio broadcast channel）
- ExecutionService 管理 Docker 容器和 Claude Code 进程
- 通过 stdout 捕获 JSON 流，规范化后存储到 MsgStore
- 双路径持久化：原始日志到数据库，规范化日志通过 WebSocket 推送

**Tech Stack:**
- Rust (Edition 2021)
- Tokio (异步运行时)
- Bollard (Docker SDK)
- SeaORM (数据库 ORM)
- serde_json (JSON 解析)
- Axum (Web 框架，用于 WebSocket)

---

## 前置条件

**环境要求:**
- Rust 1.70+
- Docker
- PostgreSQL 或 SQLite
- Node.js (用于运行 Claude Code)

**已完成的工作:**
- ✅ 设计文档 (docs/plans/2026-01-19-message-history-tracking-design.md)
- ✅ 演示代码验证 (examples/capture_claude_messages.py)
- ✅ 数据模型设计

**参考文档:**
- 设计文档: `docs/plans/2026-01-19-message-history-tracking-design.md`
- vibe-kanban 研究: `docs/plans/2026-01-19-agentfs-mcp-server-toolcall-tracking.md`
- 演示代码: `examples/capture_claude_messages.py`

---
## Task 1: 实现 MsgStore（消息存储核心）

**目标:** 创建内存中的消息队列，支持多订阅者和历史回放

**Files:**
- Create: `backend/crates/utils/src/msg_store.rs`
- Modify: `backend/crates/utils/src/lib.rs`
- Test: `backend/crates/utils/tests/msg_store_test.rs`

**依赖:**
```toml
# backend/Cargo.toml
[dependencies]
tokio = { version = "1.0", features = ["sync", "time"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

### Step 1: 写失败的测试

**File:** `backend/crates/utils/tests/msg_store_test.rs`

```rust
use utils::msg_store::{MsgStore, LogMsg};

#[tokio::test]
async fn test_msg_store_push_and_subscribe() {
    // 创建 MsgStore
    let store = MsgStore::new();
    
    // 订阅消息
    let mut receiver = store.subscribe();
    
    // 推送消息
    store.push(LogMsg::Stdout("test message".to_string()));
    
    // 接收消息
    let msg = receiver.recv().await.unwrap();
    
    // 验证
    match msg {
        LogMsg::Stdout(content) => assert_eq!(content, "test message"),
        _ => panic!("Expected Stdout message"),
    }
}

#[tokio::test]
async fn test_msg_store_history() {
    let store = MsgStore::new();
    
    // 推送多条消息
    store.push(LogMsg::Stdout("msg1".to_string()));
    store.push(LogMsg::Stdout("msg2".to_string()));
    store.push(LogMsg::Stdout("msg3".to_string()));
    
    // 获取历史
    let history = store.get_history();
    
    // 验证
    assert_eq!(history.len(), 3);
}

#[tokio::test]
async fn test_msg_store_circular_buffer() {
    let store = MsgStore::with_capacity(100); // 100 bytes
    
    // 推送超过容量的消息
    for i in 0..20 {
        store.push(LogMsg::Stdout(format!("message {}", i)));
    }
    
    // 验证不会无限增长
    let history = store.get_history();
    assert!(history.len() < 20); // 应该有一些被移除了
}
```

### Step 2: 运行测试验证失败

**Command:**
```bash
cd backend
cargo test --package utils --test msg_store_test
```

**Expected Output:**
```
error[E0433]: failed to resolve: could not find `msg_store` in `utils`
```

### Step 3: 实现 LogMsg 枚举

**File:** `backend/crates/utils/src/msg_store.rs`

```rust
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::collections::VecDeque;
use tokio::sync::broadcast;

/// 日志消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LogMsg {
    /// 标准输出
    Stdout(String),
    /// 标准错误
    Stderr(String),
    /// JSON Patch (RFC 6902)
    JsonPatch(serde_json::Value),
    /// 会话 ID
    SessionId(String),
    /// 工具调用信息
    ToolCall {
        tool_name: String,
        input: serde_json::Value,
    },
    /// 执行完成信号
    Finished,
}

/// 内部存储结构
struct Inner {
    /// 循环缓冲区
    buffer: VecDeque<u8>,
    /// 容量限制（字节）
    capacity: usize,
    /// 消息偏移量（用于快速查找）
    message_offsets: Vec<usize>,
}

/// 消息存储
pub struct MsgStore {
    inner: Arc<RwLock<Inner>>,
    sender: broadcast::Sender<LogMsg>,
}

impl MsgStore {
    /// 创建新的 MsgStore，默认容量 100MB
    pub fn new() -> Self {
        Self::with_capacity(100 * 1024 * 1024)
    }
    
    /// 创建指定容量的 MsgStore
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(1000);
        
        Self {
            inner: Arc::new(RwLock::new(Inner {
                buffer: VecDeque::with_capacity(capacity),
                capacity,
                message_offsets: Vec::new(),
            })),
            sender,
        }
    }
    
    /// 推送消息
    pub fn push(&self, msg: LogMsg) {
        let mut inner = self.inner.write().unwrap();
        
        // 序列化消息
        let bytes = serde_json::to_vec(&msg).unwrap();
        let msg_len = bytes.len();
        
        // 如果超过容量，移除最旧的消息
        while inner.buffer.len() + msg_len > inner.capacity && !inner.message_offsets.is_empty() {
            let first_offset = inner.message_offsets.remove(0);
            // 移除第一条消息
            inner.buffer.drain(0..first_offset);
            // 调整所有偏移量
            for offset in &mut inner.message_offsets {
                *offset -= first_offset;
            }
        }
        
        // 添加新消息
        inner.message_offsets.push(inner.buffer.len());
        inner.buffer.extend(&bytes);
        
        // 广播给所有订阅者
        let _ = self.sender.send(msg);
    }
    
    /// 订阅消息流
    pub fn subscribe(&self) -> broadcast::Receiver<LogMsg> {
        self.sender.subscribe()
    }
    
    /// 获取历史消息
    pub fn get_history(&self) -> Vec<LogMsg> {
        let inner = self.inner.read().unwrap();
        let mut messages = Vec::new();
        
        let mut current_pos = 0;
        for &offset in &inner.message_offsets {
            let msg_bytes: Vec<u8> = inner.buffer
                .range(current_pos..offset)
                .copied()
                .collect();
            
            if let Ok(msg) = serde_json::from_slice(&msg_bytes) {
                messages.push(msg);
            }
            
            current_pos = offset;
        }
        
        messages
    }
}

impl Default for MsgStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_msg_serialization() {
        let msg = LogMsg::Stdout("test".to_string());
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Stdout"));
    }
}
```

### Step 4: 导出模块

**File:** `backend/crates/utils/src/lib.rs`

```rust
pub mod msg_store;

pub use msg_store::{LogMsg, MsgStore};
```

### Step 5: 运行测试验证通过

**Command:**
```bash
cd backend
cargo test --package utils --test msg_store_test -- --nocapture
```

**Expected Output:**
```
running 3 tests
test test_msg_store_push_and_subscribe ... ok
test test_msg_store_history ... ok
test test_msg_store_circular_buffer ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Step 6: 提交

```bash
cd backend
git add crates/utils/src/msg_store.rs
git add crates/utils/src/lib.rs
git add crates/utils/tests/msg_store_test.rs
git add Cargo.toml
git commit -m "feat(utils): implement MsgStore with circular buffer and broadcast channel

- Add LogMsg enum for different message types
- Implement circular buffer with 100MB default capacity
- Add broadcast channel for multi-subscriber support
- Add history retrieval functionality
- Include comprehensive tests

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: 数据库迁移和模型

**目标:** 创建 execution_processes 和 execution_logs 表及对应的 SeaORM 模型

**Files:**
- Create: `backend/crates/migration/src/m20260119_000001_create_execution_tables.rs`
- Modify: `backend/crates/migration/src/lib.rs`
- Create: `backend/crates/entities/src/execution_process.rs`
- Create: `backend/crates/entities/src/execution_log.rs`
- Modify: `backend/crates/entities/src/lib.rs`

---

### Step 1: 写迁移测试

**File:** `backend/crates/migration/tests/execution_tables_test.rs`

```rust
use sea_orm::{Database, DbBackend, Schema};
use migration::{Migrator, MigratorTrait};

#[tokio::test]
async fn test_execution_tables_migration() {
    // 使用内存 SQLite
    let db = Database::connect("sqlite::memory:").await.unwrap();
    
    // 运行迁移
    Migrator::up(&db, None).await.unwrap();
    
    // 验证表存在
    let tables = db.query_all(
        "SELECT name FROM sqlite_master WHERE type='table'"
    ).await.unwrap();
    
    let table_names: Vec<String> = tables.iter()
        .map(|row| row.try_get("", "name").unwrap())
        .collect();
    
    assert!(table_names.contains(&"execution_processes".to_string()));
    assert!(table_names.contains(&"execution_logs".to_string()));
}
```

### Step 2: 运行测试验证失败

**Command:**
```bash
cd backend
cargo test --package migration --test execution_tables_test
```

**Expected:** 编译错误或测试失败

### Step 3: 创建迁移文件

**File:** `backend/crates/migration/src/m20260119_000001_create_execution_tables.rs`

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 execution_processes 表
        manager
            .create_table(
                Table::create()
                    .table(ExecutionProcess::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExecutionProcess::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ExecutionProcess::WorkspaceId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExecutionProcess::TaskId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExecutionProcess::AgentType)
                            .string()
                            .string_len(50)
                            .not_null()
                            .default("claude_code"),
                    )
                    .col(
                        ColumnDef::new(ExecutionProcess::Status)
                            .string()
                            .string_len(20)
                            .not_null()
                            .default("running"),
                    )
                    .col(ColumnDef::new(ExecutionProcess::ExitCode).integer())
                    .col(
                        ColumnDef::new(ExecutionProcess::StartedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(ExecutionProcess::CompletedAt).timestamp())
                    .col(
                        ColumnDef::new(ExecutionProcess::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ExecutionProcess::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_execution_processes_workspace_id")
                    .table(ExecutionProcess::Table)
                    .col(ExecutionProcess::WorkspaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_execution_processes_status")
                    .table(ExecutionProcess::Table)
                    .col(ExecutionProcess::Status)
                    .to_owned(),
            )
            .await?;

        // 创建 execution_logs 表
        manager
            .create_table(
                Table::create()
                    .table(ExecutionLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExecutionLog::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ExecutionLog::ExecutionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExecutionLog::LogLine)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExecutionLog::SequenceNumber)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExecutionLog::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_execution_logs_execution_id")
                            .from(ExecutionLog::Table, ExecutionLog::ExecutionId)
                            .to(ExecutionProcess::Table, ExecutionProcess::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_execution_logs_execution_id")
                    .table(ExecutionLog::Table)
                    .col(ExecutionLog::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_execution_logs_sequence")
                    .table(ExecutionLog::Table)
                    .col(ExecutionLog::ExecutionId)
                    .col(ExecutionLog::SequenceNumber)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExecutionLog::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ExecutionProcess::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum ExecutionProcess {
    Table,
    Id,
    WorkspaceId,
    TaskId,
    AgentType,
    Status,
    ExitCode,
    StartedAt,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ExecutionLog {
    Table,
    Id,
    ExecutionId,
    LogLine,
    SequenceNumber,
    CreatedAt,
}
```

### Step 4: 注册迁移

**File:** `backend/crates/migration/src/lib.rs`

```rust
pub use sea_orm_migration::prelude::*;

mod m20260119_000001_create_execution_tables;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // 添加新迁移
            Box::new(m20260119_000001_create_execution_tables::Migration),
        ]
    }
}
```

### Step 5: 运行迁移测试

**Command:**
```bash
cd backend
cargo test --package migration --test execution_tables_test
```

**Expected:** 测试通过

### Step 6: 提交迁移

```bash
cd backend
git add crates/migration/src/m20260119_000001_create_execution_tables.rs
git add crates/migration/src/lib.rs
git add crates/migration/tests/execution_tables_test.rs
git commit -m "feat(migration): add execution_processes and execution_logs tables

- Create execution_processes table with agent_type field
- Create execution_logs table with foreign key cascade
- Add indexes for performance
- Include migration tests

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: 创建 SeaORM 实体模型

**目标:** 为 execution_processes 和 execution_logs 创建 Rust 模型

**Files:**
- Create: `backend/crates/entities/src/execution_process.rs`
- Create: `backend/crates/entities/src/execution_log.rs`
- Modify: `backend/crates/entities/src/lib.rs`

---

### Step 1: 生成实体模型

**Command:**
```bash
cd backend
sea-orm-cli generate entity \
    -u sqlite:./data/vibe-repo.db \
    -o crates/entities/src \
    --with-serde both
```

**Expected:** 生成 execution_process.rs 和 execution_log.rs

### Step 2: 手动调整模型（如果需要）

**File:** `backend/crates/entities/src/execution_process.rs`

确保包含以下内容：

```rust
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "execution_processes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub task_id: Uuid,
    pub agent_type: String,
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub started_at: DateTime,
    pub completed_at: Option<DateTime>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum ExecutionStatus {
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "killed")]
    Killed,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::execution_log::Entity")]
    ExecutionLogs,
}

impl Related<super::execution_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExecutionLogs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

### Step 3: 导出模型

**File:** `backend/crates/entities/src/lib.rs`

```rust
pub mod execution_process;
pub mod execution_log;

pub use execution_process::{Entity as ExecutionProcess, Model as ExecutionProcessModel};
pub use execution_log::{Entity as ExecutionLog, Model as ExecutionLogModel};
```

### Step 4: 编译验证

**Command:**
```bash
cd backend
cargo build --package entities
```

**Expected:** 编译成功

### Step 5: 提交

```bash
cd backend
git add crates/entities/src/execution_process.rs
git add crates/entities/src/execution_log.rs
git add crates/entities/src/lib.rs
git commit -m "feat(entities): add ExecutionProcess and ExecutionLog models

- Add ExecutionProcess model with ExecutionStatus enum
- Add ExecutionLog model with foreign key relation
- Export models in lib.rs

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: 实现 ClaudeCodeExecutor

**目标:** 创建 Claude Code 的执行器，负责启动进程和捕获输出

**Files:**
- Create: `backend/crates/executors/src/executor.rs` (trait 定义)
- Create: `backend/crates/executors/src/claude_code.rs`
- Create: `backend/crates/executors/src/lib.rs`
- Test: `backend/crates/executors/tests/claude_code_test.rs`

---

### Step 1: 定义 Executor trait

**File:** `backend/crates/executors/src/executor.rs`

```rust
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentType {
    ClaudeCode,
    GeminiCli,
    OpenCode,
}

pub struct ExecutionEnv {
    pub api_keys: HashMap<String, String>,
    pub working_dir: PathBuf,
    pub workspace_id: Uuid,
    pub task_prompt: String,
}

pub struct AgentConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env_vars: Vec<(String, String)>,
    pub requires_api_key: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone)]
pub enum NormalizedEntry {
    Thinking { content: String },
    ToolUse {
        tool_name: String,
        input: Value,
    },
    ToolResult {
        tool_name: String,
        output: String,
        success: bool,
    },
    AssistantMessage { content: String },
    ErrorMessage { message: String },
    SystemMessage { content: String },
}

#[async_trait]
pub trait CodingAgentExecutor: Send + Sync {
    fn agent_type(&self) -> AgentType;
    
    fn build_exec_command(&self, prompt: &str, env: &ExecutionEnv) -> Vec<String>;
    
    fn build_env_vars(&self, env: &ExecutionEnv) -> Vec<String>;
    
    fn normalize_log_line(&self, line: &str) -> Option<NormalizedEntry>;
    
    async fn check_availability(&self) -> Result<bool, Box<dyn std::error::Error>>;
    
    fn default_config(&self) -> AgentConfig;
}
```

### Step 2: 实现 ClaudeCodeExecutor

**File:** `backend/crates/executors/src/claude_code.rs`

```rust
use crate::executor::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ClaudeCodeExecutor {
    pub version: String,
}

impl ClaudeCodeExecutor {
    pub fn new() -> Self {
        Self {
            version: "2.1.7".to_string(),
        }
    }
}

impl Default for ClaudeCodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ClaudeMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[async_trait]
impl CodingAgentExecutor for ClaudeCodeExecutor {
    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
    }
    
    fn build_exec_command(&self, prompt: &str, _env: &ExecutionEnv) -> Vec<String> {
        vec![
            "npx".to_string(),
            "-y".to_string(),
            format!("@anthropic-ai/claude-code@{}", self.version),
            "--verbose".to_string(),
            "--output-format=stream-json".to_string(),
            "--include-partial-messages".to_string(),
            "--permission-mode=bypassPermissions".to_string(),
        ]
    }
    
    fn build_env_vars(&self, env: &ExecutionEnv) -> Vec<String> {
        let mut vars = vec![];
        
        if let Some(token) = env.api_keys.get("ANTHROPIC_AUTH_TOKEN") {
            vars.push(format!("ANTHROPIC_AUTH_TOKEN={}", token));
        }
        
        vars
    }
    
    fn normalize_log_line(&self, line: &str) -> Option<NormalizedEntry> {
        let msg: ClaudeMessage = serde_json::from_str(line).ok()?;
        
        match msg.msg_type.as_str() {
            "thinking" => Some(NormalizedEntry::Thinking {
                content: msg.content,
            }),
            "tool_use" => Some(NormalizedEntry::ToolUse {
                tool_name: msg.name?,
                input: msg.input?,
            }),
            "tool_result" => Some(NormalizedEntry::ToolResult {
                tool_name: "unknown".to_string(),
                output: msg.content,
                success: true,
            }),
            "result" => Some(NormalizedEntry::AssistantMessage {
                content: msg.content,
            }),
            "error" => Some(NormalizedEntry::ErrorMessage {
                message: msg.content,
            }),
            _ => Some(NormalizedEntry::SystemMessage {
                content: line.to_string(),
            }),
        }
    }
    
    async fn check_availability(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let output = Command::new("npx")
            .arg("--version")
            .output()
            .await?;
        
        Ok(output.status.success())
    }
    
    fn default_config(&self) -> AgentConfig {
        AgentConfig {
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@anthropic-ai/claude-code".to_string(),
            ],
            env_vars: vec![],
            requires_api_key: true,
            supports_streaming: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize_thinking() {
        let executor = ClaudeCodeExecutor::new();
        let line = r#"{"type":"thinking","content":"I need to read the file"}"#;
        
        let result = executor.normalize_log_line(line);
        assert!(result.is_some());
        
        match result.unwrap() {
            NormalizedEntry::Thinking { content } => {
                assert_eq!(content, "I need to read the file");
            }
            _ => panic!("Expected Thinking entry"),
        }
    }
}
```

### Step 3: 导出模块

**File:** `backend/crates/executors/src/lib.rs`

```rust
pub mod executor;
pub mod claude_code;

pub use executor::{
    AgentType, CodingAgentExecutor, ExecutionEnv, AgentConfig, NormalizedEntry
};
pub use claude_code::ClaudeCodeExecutor;
```

### Step 4: 编译验证

**Command:**
```bash
cd backend
cargo build --package executors
```

**Expected:** 编译成功

### Step 5: 运行单元测试

**Command:**
```bash
cd backend
cargo test --package executors
```

**Expected:** 测试通过

### Step 6: 提交

```bash
cd backend
git add crates/executors/
git commit -m "feat(executors): implement ClaudeCodeExecutor

- Add CodingAgentExecutor trait for unified interface
- Implement ClaudeCodeExecutor with JSON stream parsing
- Add log normalization for different message types
- Include unit tests for message parsing

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 后续任务概览

以下任务将在后续实现（详细步骤略）：

### Task 5: ExecutionService 实现
- 管理 Docker 容器生命周期
- 启动 Claude Code 进程
- 捕获 stdout/stderr
- 集成 MsgStore

### Task 6: 日志持久化
- 实现异步日志写入数据库
- 批量插入优化
- 错误处理和重试

### Task 7: WebSocket 服务
- 实现 WebSocket 端点
- JSON Patch 流式推送
- 连接管理

### Task 8: API 端点
- GET /api/execution-processes/:id
- GET /api/execution-processes/:id/logs
- WebSocket /api/execution-processes/:id/stream

### Task 9: 集成测试
- 端到端测试
- Docker 容器测试
- 性能测试

### Task 10: 文档和部署
- API 文档
- 部署指南
- 监控和日志

---

## 执行计划完成

计划已保存到: `docs/plans/2026-01-19-claude-code-message-capture-implementation.md`

**两种执行选项:**

**1. Subagent-Driven (当前会话)** - 我为每个任务派发新的 subagent，任务间进行代码审查，快速迭代

**2. Parallel Session (独立会话)** - 在新会话中使用 executing-plans 技能，批量执行并设置检查点

**您希望使用哪种方式？**

