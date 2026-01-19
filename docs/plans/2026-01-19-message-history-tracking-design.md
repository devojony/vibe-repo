# VibeRepo 消息历史追踪设计方案

## 文档信息
- **日期**: 2026-01-19
- **版本**: v0.2.0
- **状态**: 设计阶段

## 目录
- [1. 背景与需求](#1-背景与需求)
- [2. 技术调研](#2-技术调研)
- [3. 架构设计](#3-架构设计)
- [4. 多 Agent 支持](#4-多-agent-支持)
- [5. 核心组件](#5-核心组件)
- [6. 数据模型](#6-数据模型)
- [7. 部署架构](#7-部署架构)

---

## 1. 背景与需求

### 1.1 核心需求

VibeRepo 需要追踪容器内 AI Coding Agents 的完整消息历史，包括：
- ✅ 用户输入
- ✅ Agent 响应
- ✅ 系统提示
- ✅ 思考过程
- ✅ 工具调用
- ✅ 错误信息

**支持的 Coding Agents**:
- Claude Code (Anthropic)
- Gemini CLI (Google)
- Codex (OpenAI)
- Cursor
- GitHub Copilot
- 其他符合标准接口的 agents

### 1.2 技术要求

- **实时性**: 实时流式传输，后端可以实时监控 Agent 执行过程
- **完整性**: 捕获所有消息类型，不遗漏任何信息
- **效率**: 带宽效率高，避免重复传输大量数据
- **可扩展**: 支持多个并发 Agent 执行
- **持久化**: 消息历史需要保存到数据库，支持审计和回溯

---

## 2. 技术调研

### 2.1 协议选型

我们调研了以下技术方案：

| 方案 | 优势 | 劣势 | 结论 |
|------|------|------|------|
| **ACP 协议** | 标准化，Agent-to-Agent 通信 | Claude Code 不支持，需要中间层 | ❌ 不适用 |
| **MCP 协议** | Claude Code 原生支持 | 主要用于工具调用，不适合消息历史 | ⚠️ 部分适用 |
| **vibe-kanban 方案** | 已验证有效，实时性好，效率高 | 需要自行实现 | ✅ 采用 |

### 2.2 vibe-kanban 方案分析

**vibe-kanban** 是一个成熟的 AI agent 编排平台，使用以下技术栈：

- **后端**: Rust (Axum + Tokio)
- **前端**: React + TypeScript
- **通信**: WebSocket + JSON Patch (RFC 6902)
- **容器**: Docker (Sibling Containers 模式)

**核心架构**：
```
Claude Code 进程
    ↓ stdout/stderr (JSON 流)
MsgStore (内存缓冲区，100MB 循环队列)
    ↓ Tokio broadcast channel
日志规范化器 (按 agent 类型解析)
    ↓ NormalizedEntry
JSON Patch 生成器 (RFC 6902)
    ↓ WebSocket
前端实时更新
    ↓ 同时
数据库持久化 (ExecutionProcessLogs 表)
```

**关键优势**：
- ✅ 实时性极佳（WebSocket 推送）
- ✅ 带宽效率高（JSON Patch 只传输变化）
- ✅ 完整的消息历史（stdout/stderr 全捕获）
- ✅ 结构化数据（规范化为统一格式）
- ✅ 支持多订阅者（broadcast channel）

### 2.3 技术决策

**决策**: 采用 vibe-kanban 方案，原因如下：

1. **无需 Agent 修改**: 通过 stdio 捕获，Claude Code 无需支持额外协议
2. **已验证有效**: vibe-kanban 已在生产环境使用
3. **技术栈匹配**: VibeRepo 也使用 Rust + Axum
4. **实时性优秀**: WebSocket + JSON Patch 提供最佳用户体验

---

## 3. 架构设计

### 3.1 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                    VibeRepo 后端 (Rust)                  │
│                                                           │
│  ┌─────────────────────────────────────────────────┐   │
│  │         Workspace Service                        │   │
│  │  - 创建/管理 Workspace                           │   │
│  │  - 初始化 AgentFS (文件系统)                     │   │
│  │  - 管理 Docker 容器生命周期                      │   │
│  └─────────────────┬───────────────────────────────┘   │
│                    │                                     │
│  ┌─────────────────▼───────────────────────────────┐   │
│  │         Execution Service                        │   │
│  │  - 启动 Claude Code 进程                         │   │
│  │  - 捕获 stdout/stderr                            │   │
│  │  - 管理进程生命周期                              │   │
│  └─────────────────┬───────────────────────────────┘   │
│                    │                                     │
│  ┌─────────────────▼───────────────────────────────┐   │
│  │         MsgStore (消息存储)                      │   │
│  │  - 100MB 循环缓冲区                              │   │
│  │  - Tokio broadcast channel                       │   │
│  │  - 多订阅者支持                                  │   │
│  └─────────┬───────────────────┬───────────────────┘   │
│            │                   │                         │
│  ┌─────────▼─────────┐  ┌─────▼──────────────────┐    │
│  │  日志规范化器      │  │  原始日志持久化         │    │
│  │  - 解析 JSON 流    │  │  - 保存到数据库         │    │
│  │  - 生成 Patch      │  │  - execution_logs 表    │    │
│  └─────────┬─────────┘  └────────────────────────┘    │
│            │                                             │
│  ┌─────────▼───────────────────────────────────────┐   │
│  │         WebSocket 服务                           │   │
│  │  - 实时推送 JSON Patch                           │   │
│  │  - 连接管理                                      │   │
│  └─────────┬───────────────────────────────────────┘   │
└────────────┼─────────────────────────────────────────┘
             │ WebSocket
             │
┌────────────▼─────────────────────────────────────────┐
│                    前端 (可选)                         │
│  - 接收 JSON Patch                                    │
│  - 应用 RFC 6902 更新                                 │
│  - 实时显示执行过程                                   │
└───────────────────────────────────────────────────────┘
```

### 3.2 关键设计决策

1. **AgentFS 角色**: 仅作为文件系统（FUSE mount），不使用其 MCP Server 模式
2. **MsgStore 核心**: 内存中的消息队列，作为整个系统的消息中心
3. **双路径持久化**:
   - 原始日志 → 数据库（完整审计）
   - 规范化日志 → WebSocket（实时展示）
4. **容器隔离**: 每个 Workspace 独立的 Docker 容器

---

## 4. 多 Agent 支持

### 4.1 设计目标

VibeRepo 需要支持多种 coding agents，而不仅仅是 Claude Code。参考 vibe-kanban 的实现，我们需要：

1. **统一的 Executor 接口** - 抽象不同 agent 的启动和管理
2. **日志规范化** - 将不同 agent 的输出格式统一化
3. **动态选择** - 用户可以为每个任务选择不同的 agent
4. **可扩展性** - 轻松添加新的 agent 支持

### 4.2 支持的 Agents

基于 vibe-kanban 的经验，我们将支持以下 agents：

| Agent | 提供商 | 输出格式 | 特殊要求 |
|-------|--------|---------|---------|
| **Claude Code** | Anthropic | JSON 流 | 支持控制协议、权限管理 |
| **Gemini CLI** | Google | 文本流 | 需要 Google API Key |
| **Codex** | OpenAI | 文本流 | 需要 OpenAI API Key |
| **Cursor** | Cursor | JSON 流 | 需要编辑器集成 |
| **GitHub Copilot** | GitHub | 文本流 | 需要 GitHub 认证 |
| **Amp** | Amp | 文本流 | - |
| **OpenCode** | OpenCode | 文本流 | - |
| **Qwen** | 阿里云 | 文本流 | 需要阿里云 API Key |

### 4.3 Executor 抽象

**统一接口设计**:

```rust
// crates/executors/src/executor.rs

#[async_trait]
pub trait CodingAgentExecutor: Send + Sync {
    /// Agent 类型标识
    fn agent_type(&self) -> AgentType;

    /// 构建执行命令
    fn build_exec_command(&self, task: &Task, env: &ExecutionEnv) -> Vec<String>;

    /// 构建环境变量
    fn build_env_vars(&self, env: &ExecutionEnv) -> Vec<String>;

    /// 规范化日志输出
    fn normalize_log_line(&self, line: &str) -> Option<NormalizedEntry>;

    /// 检查 agent 是否可用
    async fn check_availability(&self) -> Result<bool, ExecutorError>;

    /// 获取默认配置
    fn default_config(&self) -> AgentConfig;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentType {
    ClaudeCode,
    GeminiCli,
    Codex,
    Cursor,
    Copilot,
    Amp,
    OpenCode,
    Qwen,
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
```

### 4.4 具体实现示例

#### Claude Code Executor

```rust
// crates/executors/src/claude_code.rs

pub struct ClaudeCodeExecutor {
    config: ClaudeCodeConfig,
}

#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    pub version: String,
    pub permission_mode: PermissionMode,
    pub output_format: OutputFormat,
}

#[async_trait]
impl CodingAgentExecutor for ClaudeCodeExecutor {
    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
    }

    fn build_exec_command(&self, task: &Task, _env: &ExecutionEnv) -> Vec<String> {
        vec![
            "npx".to_string(),
            "-y".to_string(),
            format!("@anthropic-ai/claude-code@{}", self.config.version),
            "--verbose".to_string(),
            "--output-format=stream-json".to_string(),
            "--input-format=stream-json".to_string(),
            "--include-partial-messages".to_string(),
            format!("--permission-mode={}", self.config.permission_mode),
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
        // 解析 Claude Code 的 JSON 流输出
        if let Ok(msg) = serde_json::from_str::<ClaudeMessage>(line) {
            return Some(match msg.msg_type.as_str() {
                "thinking" => NormalizedEntry::Thinking {
                    content: msg.content,
                },
                "tool_use" => NormalizedEntry::ToolUse {
                    tool_name: msg.tool_name.unwrap_or_default(),
                    input: msg.input,
                    status: ToolStatus::Running,
                },
                "result" => NormalizedEntry::AssistantMessage {
                    content: msg.content,
                },
                _ => NormalizedEntry::SystemMessage {
                    content: line.to_string(),
                },
            });
        }
        None
    }

    async fn check_availability(&self) -> Result<bool, ExecutorError> {
        // 检查 npx 是否可用
        let output = Command::new("npx")
            .arg("--version")
            .output()
            .await?;
        Ok(output.status.success())
    }

    fn default_config(&self) -> AgentConfig {
        AgentConfig {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@anthropic-ai/claude-code".to_string()],
            env_vars: vec![],
            requires_api_key: true,
            supports_streaming: true,
        }
    }
}
```

#### Gemini CLI Executor

```rust
// crates/executors/src/gemini_cli.rs

pub struct GeminiCliExecutor {
    config: GeminiCliConfig,
}

#[derive(Debug, Clone)]
pub struct GeminiCliConfig {
    pub model: String,
}

#[async_trait]
impl CodingAgentExecutor for GeminiCliExecutor {
    fn agent_type(&self) -> AgentType {
        AgentType::GeminiCli
    }

    fn build_exec_command(&self, task: &Task, _env: &ExecutionEnv) -> Vec<String> {
        vec![
            "gemini".to_string(),
            "code".to_string(),
            "--model".to_string(),
            self.config.model.clone(),
            "--stream".to_string(),
        ]
    }

    fn build_env_vars(&self, env: &ExecutionEnv) -> Vec<String> {
        let mut vars = vec![];
        if let Some(key) = env.api_keys.get("GOOGLE_API_KEY") {
            vars.push(format!("GOOGLE_API_KEY={}", key));
        }
        vars
    }

    fn normalize_log_line(&self, line: &str) -> Option<NormalizedEntry> {
        // Gemini CLI 输出纯文本，需要简单解析
        if line.starts_with("Thinking:") {
            Some(NormalizedEntry::Thinking {
                content: line.strip_prefix("Thinking:").unwrap().trim().to_string(),
            })
        } else if line.starts_with("Tool:") {
            Some(NormalizedEntry::ToolUse {
                tool_name: line.strip_prefix("Tool:").unwrap().trim().to_string(),
                input: serde_json::Value::Null,
                status: ToolStatus::Running,
            })
        } else if !line.is_empty() {
            Some(NormalizedEntry::AssistantMessage {
                content: line.to_string(),
            })
        } else {
            None
        }
    }

    async fn check_availability(&self) -> Result<bool, ExecutorError> {
        let output = Command::new("gemini")
            .arg("--version")
            .output()
            .await?;
        Ok(output.status.success())
    }

    fn default_config(&self) -> AgentConfig {
        AgentConfig {
            command: "gemini".to_string(),
            args: vec!["code".to_string()],
            env_vars: vec![],
            requires_api_key: true,
            supports_streaming: true,
        }
    }
}
```

### 4.5 Executor Factory

**动态创建 Executor**:

```rust
// crates/executors/src/factory.rs

pub struct ExecutorFactory;

impl ExecutorFactory {
    pub fn create(agent_type: AgentType) -> Result<Box<dyn CodingAgentExecutor>, ExecutorError> {
        match agent_type {
            AgentType::ClaudeCode => Ok(Box::new(ClaudeCodeExecutor {
                config: ClaudeCodeConfig {
                    version: "2.1.7".to_string(),
                    permission_mode: PermissionMode::BypassPermissions,
                    output_format: OutputFormat::StreamJson,
                },
            })),
            AgentType::GeminiCli => Ok(Box::new(GeminiCliExecutor {
                config: GeminiCliConfig {
                    model: "gemini-2.0-flash-exp".to_string(),
                },
            })),
            AgentType::Codex => Ok(Box::new(CodexExecutor::default())),
            AgentType::Cursor => Ok(Box::new(CursorExecutor::default())),
            AgentType::Copilot => Ok(Box::new(CopilotExecutor::default())),
            AgentType::Amp => Ok(Box::new(AmpExecutor::default())),
            AgentType::OpenCode => Ok(Box::new(OpenCodeExecutor::default())),
            AgentType::Qwen => Ok(Box::new(QwenExecutor::default())),
        }
    }

    pub async fn list_available_agents() -> Vec<AgentType> {
        let mut available = vec![];

        for agent_type in [
            AgentType::ClaudeCode,
            AgentType::GeminiCli,
            AgentType::Codex,
            AgentType::Cursor,
            AgentType::Copilot,
            AgentType::Amp,
            AgentType::OpenCode,
            AgentType::Qwen,
        ] {
            if let Ok(executor) = Self::create(agent_type.clone()) {
                if executor.check_availability().await.unwrap_or(false) {
                    available.push(agent_type);
                }
            }
        }

        available
    }
}
```

### 4.6 日志规范化

**统一的日志格式**:

```rust
// crates/executors/src/normalized_entry.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NormalizedEntry {
    UserMessage {
        content: String,
    },
    AssistantMessage {
        content: String,
    },
    Thinking {
        content: String,
    },
    ToolUse {
        tool_name: String,
        input: serde_json::Value,
        status: ToolStatus,
    },
    ToolResult {
        tool_name: String,
        output: String,
        status: ToolStatus,
    },
    SystemMessage {
        content: String,
    },
    ErrorMessage {
        error_type: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolStatus {
    Running,
    Success,
    Failed,
}
```

### 4.7 数据库扩展

**添加 agent_type 字段**:

```sql
ALTER TABLE execution_processes
ADD COLUMN agent_type VARCHAR(50) NOT NULL DEFAULT 'claude_code';

CREATE INDEX idx_execution_processes_agent_type
    ON execution_processes(agent_type);
```

```rust
// crates/entities/src/execution_process.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_processes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub task_id: Uuid,
    pub agent_type: String,  // 新增字段
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

## 5. 核心组件

### 5.1 MsgStore - 消息存储

**职责**: 内存中的消息队列，支持多订阅者

**实现**:
```rust
// crates/utils/src/msg_store.rs

pub struct MsgStore {
    inner: RwLock<Inner>,
    sender: broadcast::Sender<LogMsg>,
}

struct Inner {
    // 循环缓冲区，100MB 上限
    buffer: VecDeque<u8>,
    capacity: usize,
    // 消息索引，用于快速查找
    message_offsets: Vec<usize>,
}

pub enum LogMsg {
    Stdout(String),           // Claude Code 标准输出
    Stderr(String),           // Claude Code 错误输出
    JsonPatch(Patch),         // RFC 6902 JSON Patch
    SessionId(String),        // Claude Code 返回的 session ID
    ToolCall(ToolCallInfo),   // 工具调用信息
    Finished,                 // 执行完成信号
}

impl MsgStore {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            inner: RwLock::new(Inner {
                buffer: VecDeque::with_capacity(100 * 1024 * 1024),
                capacity: 100 * 1024 * 1024,
                message_offsets: Vec::new(),
            }),
            sender,
        }
    }

    // 写入消息（从 Claude Code 捕获）
    pub fn push(&self, msg: LogMsg) {
        let mut inner = self.inner.write().unwrap();

        // 序列化消息
        let bytes = serde_json::to_vec(&msg).unwrap();

        // 如果超过容量，移除最旧的消息
        while inner.buffer.len() + bytes.len() > inner.capacity {
            if let Some(offset) = inner.message_offsets.first() {
                inner.buffer.drain(0..*offset);
                inner.message_offsets.remove(0);
            }
        }

        // 添加新消息
        inner.message_offsets.push(inner.buffer.len());
        inner.buffer.extend(&bytes);

        // 广播给所有订阅者
        let _ = self.sender.send(msg);
    }

    // 订阅消息流（WebSocket 使用）
    pub fn subscribe(&self) -> broadcast::Receiver<LogMsg> {
        self.sender.subscribe()
    }

    // 获取历史消息（新订阅者使用）
    pub fn get_history(&self) -> Vec<LogMsg> {
        let inner = self.inner.read().unwrap();
        inner.message_offsets.iter()
            .map(|&offset| {
                let bytes = &inner.buffer[offset..];
                serde_json::from_slice(bytes).unwrap()
            })
            .collect()
    }
}
```

**设计要点**:
- **循环缓冲区**: 自动管理内存，防止无限增长
- **Broadcast channel**: 支持多个 WebSocket 客户端同时订阅
- **历史回放**: 新连接的客户端可以获取完整历史
- **线程安全**: 使用 RwLock 保护共享状态

### 5.2 ExecutionService - 执行服务（支持多 Agent）

**职责**: 启动和管理多种 Coding Agent 进程，捕获输出

**实现**:
```rust
// crates/services/src/execution_service.rs

pub struct ExecutionService {
    docker: Docker,
    db: DatabaseConnection,
    msg_stores: Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>>,
    executor_factory: ExecutorFactory,
}

impl ExecutionService {
    pub async fn start_execution(
        &self,
        workspace: &Workspace,
        task: &Task,
        agent_type: AgentType,  // 新增参数：选择 agent 类型
    ) -> Result<ExecutionProcess, ExecutionError> {
        // 1. 创建 Executor
        let executor = ExecutorFactory::create(agent_type.clone())?;

        // 2. 检查 agent 可用性
        if !executor.check_availability().await? {
            return Err(ExecutorError::AgentNotAvailable(agent_type));
        }

        // 3. 创建 MsgStore
        let msg_store = Arc::new(MsgStore::new());
        let exec_id = Uuid::new_v4();
        self.msg_stores.write().unwrap()
            .insert(exec_id, msg_store.clone());

        // 4. 创建数据库记录
        let execution = ExecutionProcess::create(
            &self.db,
            exec_id,
            workspace.id,
            task.id,
            agent_type.to_string(),  // 保存 agent 类型
            ExecutionStatus::Running,
        ).await?;

        // 5. 确保容器存在
        self.ensure_container_exists(workspace).await?;

        // 6. 构建执行环境
        let env = ExecutionEnv {
            api_keys: self.load_api_keys().await?,
            working_dir: PathBuf::from("/workspace"),
            workspace_id: workspace.id,
            task_prompt: task.prompt.clone(),
        };

        // 7. 在容器内启动 Agent
        let exec_config = self.build_exec_config(&executor, task, &env).await?;
        let exec = self.docker.create_exec(
            &workspace.container_id,
            exec_config,
        ).await?;

        // 8. 启动输出捕获（带日志规范化）
        self.spawn_output_capture_with_normalization(
            exec.id,
            msg_store.clone(),
            execution.id,
            executor,
        ).await?;

        // 9. 启动日志持久化
        self.spawn_log_persistence(
            msg_store.clone(),
            execution.id,
        ).await?;

        Ok(execution)
    }

    fn build_exec_config(
        &self,
        executor: &Box<dyn CodingAgentExecutor>,
        task: &Task,
        env: &ExecutionEnv,
    ) -> CreateExecOptions {
        let cmd = executor.build_exec_command(task, env);
        let env_vars = executor.build_env_vars(env);

        CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            working_dir: Some("/workspace"),
            env: Some(env_vars),
            ..Default::default()
        }
    }

    async fn spawn_output_capture_with_normalization(
        &self,
        exec_id: String,
        msg_store: Arc<MsgStore>,
        execution_id: Uuid,
        executor: Box<dyn CodingAgentExecutor>,
    ) -> Result<(), ExecutionError> {
        let docker = self.docker.clone();

        tokio::spawn(async move {
            let output = docker.start_exec(&exec_id, None).await?;

            match output {
                StartExecResults::Attached { mut output, .. } => {
                    while let Some(chunk) = output.next().await {
                        match chunk? {
                            LogOutput::StdOut { message } => {
                                let line = String::from_utf8_lossy(&message);

                                // 原始日志
                                msg_store.push(LogMsg::Stdout(line.to_string()));

                                // 规范化日志
                                if let Some(normalized) = executor.normalize_log_line(&line) {
                                    let patch = Self::generate_json_patch(&normalized);
                                    msg_store.push(LogMsg::JsonPatch(patch));
                                }
                            }
                            LogOutput::StdErr { message } => {
                                let line = String::from_utf8_lossy(&message);
                                msg_store.push(LogMsg::Stderr(line.to_string()));
                            }
                            _ => {}
                        }
                    }

                    msg_store.push(LogMsg::Finished);
                }
                _ => {}
            }

            Ok::<_, ExecutionError>(())
        });

        Ok(())
    }

    fn generate_json_patch(entry: &NormalizedEntry) -> json_patch::Patch {
        // 生成 RFC 6902 JSON Patch
        // 用于前端增量更新
        json_patch::Patch(vec![
            json_patch::PatchOperation::Add(json_patch::AddOperation {
                path: format!("/entries/-"),
                value: serde_json::to_value(entry).unwrap(),
            }),
        ])
    }

    async fn load_api_keys(&self) -> Result<HashMap<String, String>, ExecutionError> {
        let mut keys = HashMap::new();

        // 从环境变量或数据库加载 API keys
        if let Ok(token) = env::var("ANTHROPIC_AUTH_TOKEN") {
            keys.insert("ANTHROPIC_AUTH_TOKEN".to_string(), token);
        }
        if let Ok(key) = env::var("GOOGLE_API_KEY") {
            keys.insert("GOOGLE_API_KEY".to_string(), key);
        }
        if let Ok(key) = env::var("OPENAI_API_KEY") {
            keys.insert("OPENAI_API_KEY".to_string(), key);
        }

        Ok(keys)
    }
}


**设计要点**:
- **异步任务**: 使用 tokio::spawn 处理输出捕获和持久化
- **多 Agent 支持**: 通过 Executor trait 统一接口
- **日志规范化**: 将不同 agent 的输出统一为 NormalizedEntry
- **容器管理**: 自动创建和启动容器
- **错误处理**: 完善的错误传播机制

---

    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    task_id UUID NOT NULL REFERENCES tasks(id),
    status VARCHAR(20) NOT NULL DEFAULT 'running',
    -- 状态: running, completed, failed, killed
    exit_code INTEGER,
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_execution_processes_workspace_id
    ON execution_processes(workspace_id);
CREATE INDEX idx_execution_processes_task_id
    ON execution_processes(task_id);
CREATE INDEX idx_execution_processes_status
    ON execution_processes(status);
```

#### execution_logs 表

```sql
CREATE TABLE execution_logs (
    id BIGSERIAL PRIMARY KEY,
    execution_id UUID NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    log_line TEXT NOT NULL,
    -- JSON 格式的 LogMsg
    sequence_number INTEGER NOT NULL,
    -- 日志序号，用于排序
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_execution_logs_execution_id
    ON execution_logs(execution_id);
CREATE INDEX idx_execution_logs_sequence
    ON execution_logs(execution_id, sequence_number);
```

#### workspaces 表（扩展）

```sql
ALTER TABLE workspaces
ADD COLUMN container_id VARCHAR(255),
ADD COLUMN agentfs_initialized BOOLEAN DEFAULT FALSE,
ADD COLUMN agentfs_db_path VARCHAR(500);
```

### 5.2 Rust 模型

```rust
// crates/entities/src/execution_process.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_processes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub task_id: Uuid,
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum)]
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
```

```rust
// crates/entities/src/execution_log.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "execution_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub execution_id: Uuid,
    pub log_line: String,
    pub sequence_number: i32,
    pub created_at: DateTime<Utc>,
}
```

---

## 6. 部署架构

### 6.1 Sibling Containers 模式

VibeRepo 采用 **Sibling Containers** 部署模式：

```
┌─────────────────────────────────────────────────────┐
│              宿主机 (Host Machine)                   │
│                                                      │
│  ┌────────────────────────────────────────────┐   │
│  │  VibeRepo 容器 (主容器)                    │   │
│  │                                             │   │
│  │  ┌──────────────────────────────────────┐ │   │
│  │  │  VibeRepo 后端 (Rust 进程)           │ │   │
│  │  │  - Axum Web Server                   │ │   │
│  │  │  - ExecutionService                  │ │   │
│  │  │  - WebSocket Server                  │ │   │
│  │  └──────────────┬───────────────────────┘ │   │
│  │                 │                           │   │
│  │                 │ 挂载 docker.sock          │   │
│  │                 │ -v /var/run/docker.sock   │   │
│  └─────────────────┼───────────────────────────┘   │
│                    │                                 │
│  ┌─────────────────▼─────────────────────────┐    │
│  │         Docker Daemon                      │    │
│  └─────────────────┬─────────────────────────┘    │
│                    │                                 │
│     ┌──────────────▼──────────┐  ┌──────────┐    │
│     │  Agent Container 1      │  │ Agent 2  │    │
│     │  workspace-uuid-1       │  │          │    │
│     │                         │  │          │    │
│     │  ┌──────────────────┐  │  │          │    │
│     │  │ Claude Code      │  │  │          │    │
│     │  │ 进程             │  │  │          │    │
│     │  └──────────────────┘  │  │          │    │
│     │                         │  │          │    │
│     │  ┌──────────────────┐  │  │          │    │
│     │  │ AgentFS          │  │  │          │    │
│     │  │ (FUSE mount)     │  │  │          │    │
│     │  └──────────────────┘  │  │          │    │
│     │                         │  │          │    │
│     │  /workspace (挂载)     │  │          │    │
│     └─────────────────────────┘  └──────────┘    │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### 6.2 Docker Compose 配置

```yaml
version: '3.8'

services:
  vibe-repo:
    image: vibe-repo:latest
    container_name: vibe-repo
    ports:
      - "3000:3000"
    volumes:
      # 挂载 Docker socket，允许创建 sibling 容器
      - /var/run/docker.sock:/var/run/docker.sock
      # 数据持久化
      - ./data:/data
      # AgentFS 数据
      - ./data/agentfs:/data/agentfs
    environment:
      DATABASE_URL: postgresql://vibe:vibe@postgres:5432/vibe_repo
      DOCKER_HOST: unix:///var/run/docker.sock
      RUST_LOG: info
      SERVER_HOST: 0.0.0.0
      SERVER_PORT: 3000
    depends_on:
      - postgres
    restart: unless-stopped

  postgres:
    image: postgres:16-alpine
    container_name: vibe-repo-postgres
    environment:
      POSTGRES_USER: vibe
      POSTGRES_PASSWORD: vibe
      POSTGRES_DB: vibe_repo
    volumes:
      - postgres-data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    restart: unless-stopped

volumes:
  postgres-data:
```

### 6.3 Agent 容器镜像

```dockerfile
# Dockerfile.agent
FROM ubuntu:22.04

# 安装基础工具
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    fuse \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# 安装 AgentFS
RUN curl -fsSL https://agentfs.dev/install.sh | bash

# 安装 Claude Code (通过 npx 动态安装，无需预装)

# 创建工作目录
RUN mkdir -p /workspace
WORKDIR /workspace

# 设置环境变量
ENV AGENTFS_BASE=/workspace
ENV NODE_ENV=production

CMD ["/bin/bash"]
```

### 6.4 容器生命周期管理

**创建容器**:
```rust
let container = docker.create_container(
    Some(CreateContainerOptions {
        name: format!("vibe-workspace-{}", workspace.id),
    }),
    Config {
        image: Some("vibe-agent:latest"),
        working_dir: Some("/workspace"),
        host_config: Some(HostConfig {
            binds: Some(vec![
                format!("{}:/workspace", workspace.work_dir),
                format!("{}/.agentfs:/root/.agentfs", workspace.work_dir),
            ]),
            cap_add: Some(vec!["SYS_ADMIN"]),
            devices: Some(vec![DeviceMapping {
                path_on_host: Some("/dev/fuse".to_string()),
                path_in_container: Some("/dev/fuse".to_string()),
                cgroup_permissions: Some("rwm".to_string()),
            }]),
            security_opt: Some(vec!["apparmor=unconfined".to_string()]),
            auto_remove: Some(false), // 不自动删除，便于调试
            ..Default::default()
        }),
        ..Default::default()
    },
).await?;
```

**启动容器**:
```rust
docker.start_container(&container.id, None).await?;
```

**停止容器**:
```rust
docker.stop_container(&container.id, None).await?;
```

**删除容器**:
```rust
docker.remove_container(&container.id, Some(RemoveContainerOptions {
    force: true,
    ..Default::default()
})).await?;
```

---

## 7. 下一步

### 7.1 待设计内容

- [ ] 日志规范化器（解析 Claude Code JSON 流）
- [ ] WebSocket 服务实现
- [ ] API 端点设计
- [ ] 前端集成方案
- [ ] 错误处理和重试机制
- [ ] 性能优化和监控

### 7.2 待实现功能

- [ ] MsgStore 实现
- [ ] ExecutionService 实现
- [ ] 数据库迁移脚本
- [ ] Docker 镜像构建
- [ ] 集成测试

---

## 8. 参考资料

- [vibe-kanban GitHub](https://github.com/BloopAI/vibe-kanban)
- [RFC 6902 - JSON Patch](https://datatracker.ietf.org/doc/html/rfc6902)
- [Docker Rust SDK](https://docs.rs/bollard/latest/bollard/)
- [Tokio Broadcast Channel](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [AgentFS Documentation](https://agentfs.dev)
