# Rust 惯用设计：Coding Agent Executor 架构

## 文档信息
- **日期**: 2026-01-19
- **版本**: v1.0.0
- **目标**: 利用 Rust 语言特性设计更优雅的 Executor 架构

## 设计原则

1. **类型安全** - 利用 Rust 的类型系统在编译时捕获错误
2. **零成本抽象** - 使用泛型和 enum dispatch 避免运行时开销
3. **错误处理** - 使用 thiserror 定义清晰的错误类型
4. **异步优先** - 充分利用 Tokio 的异步特性
5. **所有权明确** - 清晰的所有权和生命周期管理

---
## 错误处理设计

### 使用 thiserror 定义清晰的错误类型

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Agent not available: {agent_type}")]
    AgentNotAvailable { agent_type: String },
    
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Message parsing error: {0}")]
    MessageParsing(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Process spawn error: {0}")]
    ProcessSpawn(String),
    
    #[error("Timeout after {seconds}s")]
    Timeout { seconds: u64 },
    
    #[error("Agent crashed: {message}")]
    AgentCrashed { message: String },
}

pub type Result<T> = std::result::Result<T, ExecutorError>;
```

**优势**:
- 清晰的错误信息
- 自动实现 `Display` 和 `Error` trait
- 支持错误链（`#[from]`）
- 编译时类型安全

---

## Executor Trait 设计

### 使用关联类型和泛型

```rust
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// 规范化的消息类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NormalizedMessage {
    /// 用户消息
    UserMessage { content: String },
    
    /// 助手消息
    AssistantMessage { content: String },
    
    /// 思考过程
    Thinking { content: String },
    
    /// 工具调用
    ToolUse {
        tool_name: String,
        input: serde_json::Value,
        status: ToolStatus,
    },
    
    /// 工具结果
    ToolResult {
        tool_name: String,
        output: String,
        status: ToolStatus,
    },
    
    /// 系统消息
    SystemMessage { content: String },
    
    /// 错误消息
    ErrorMessage { message: String },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ToolStatus {
    Running,
    Success,
    Failed,
}

/// 执行环境配置
#[derive(Debug, Clone)]
pub struct ExecutionEnv {
    pub working_dir: PathBuf,
    pub api_keys: HashMap<String, String>,
    pub timeout_secs: Option<u64>,
}

/// Coding Agent Executor trait
#[async_trait]
pub trait CodingAgentExecutor: Send + Sync {
    /// 关联类型：Agent 特定的配置
    type Config: Clone + Send + Sync;
    
    /// 返回 agent 类型标识
    fn agent_type(&self) -> &'static str;
    
    /// 构建执行命令
    fn build_command(&self, prompt: &str, env: &ExecutionEnv) -> Vec<String>;
    
    /// 构建环境变量
    fn build_env_vars(&self, env: &ExecutionEnv) -> HashMap<String, String>;
    
    /// 解析原始输出行为规范化消息
    fn parse_message(&self, line: &str) -> Option<NormalizedMessage>;
    
    /// 检查 agent 是否可用
    async fn check_availability(&self) -> Result<bool>;
    
    /// 获取默认配置
    fn default_config() -> Self::Config;
    
    /// 执行任务并返回消息流
    async fn execute(
        &self,
        prompt: &str,
        env: ExecutionEnv,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<NormalizedMessage>> + Send>>>;
}
```

**改进点**:
1. **关联类型** `Config` - 每个 executor 可以有自己的配置类型
2. **Stream 返回** - 使用 `futures::Stream` 进行异步迭代
3. **明确的生命周期** - 所有类型都是 `'static` 或明确标注
4. **零拷贝** - 使用 `&'static str` 作为 agent_type

---

## Enum Dispatch 设计

### 零成本抽象的 Agent 类型

```rust
/// Agent 枚举 - 使用 enum dispatch 避免动态分发开销
#[derive(Debug, Clone)]
pub enum Agent {
    ClaudeCode(ClaudeCodeExecutor),
    GeminiCli(GeminiCliExecutor),
    OpenCodeAcp(OpenCodeAcpExecutor),
}

impl Agent {
    /// 工厂方法
    pub fn from_type(agent_type: &str) -> Result<Self> {
        match agent_type {
            "claude-code" => Ok(Agent::ClaudeCode(ClaudeCodeExecutor::new())),
            "gemini-cli" => Ok(Agent::GeminiCli(GeminiCliExecutor::new())),
            "opencode-acp" => Ok(Agent::OpenCodeAcp(OpenCodeAcpExecutor::new())),
            _ => Err(ExecutorError::InvalidConfig(
                format!("Unknown agent type: {}", agent_type)
            )),
        }
    }
    
    /// 统一的执行接口
    pub async fn execute(
        &self,
        prompt: &str,
        env: ExecutionEnv,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<NormalizedMessage>> + Send>>> {
        match self {
            Agent::ClaudeCode(executor) => executor.execute(prompt, env).await,
            Agent::GeminiCli(executor) => executor.execute(prompt, env).await,
            Agent::OpenCodeAcp(executor) => executor.execute(prompt, env).await,
        }
    }
    
    /// 获取 agent 类型
    pub fn agent_type(&self) -> &'static str {
        match self {
            Agent::ClaudeCode(executor) => executor.agent_type(),
            Agent::GeminiCli(executor) => executor.agent_type(),
            Agent::OpenCodeAcp(executor) => executor.agent_type(),
        }
    }
    
    /// 检查可用性
    pub async fn check_availability(&self) -> Result<bool> {
        match self {
            Agent::ClaudeCode(executor) => executor.check_availability().await,
            Agent::GeminiCli(executor) => executor.check_availability().await,
            Agent::OpenCodeAcp(executor) => executor.check_availability().await,
        }
    }
}
```

**优势**:
- **零成本抽象** - 编译器可以完全内联，无虚函数表开销
- **穷尽性检查** - 编译器确保所有 agent 类型都被处理
- **类型安全** - 每个 agent 有自己的类型和配置
- **易于扩展** - 添加新 agent 只需添加新的 enum variant

**性能对比**:
- Trait Object (`Box<dyn CodingAgentExecutor>`): 动态分发，运行时开销
- Enum Dispatch: 静态分发，零运行时开销

---

## 消息流处理设计

### 使用 Stream trait 进行异步迭代

```rust
use futures::{Stream, StreamExt};
use tokio::process::{Command, ChildStdout};
use tokio::io::{BufReader, AsyncBufReadExt};

/// Claude Code Executor 实现
pub struct ClaudeCodeExecutor {
    config: ClaudeCodeConfig,
}

#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    pub model: String,
    pub include_partial_messages: bool,
    pub permission_mode: String,
}

impl ClaudeCodeExecutor {
    pub fn new() -> Self {
        Self {
            config: ClaudeCodeConfig {
                model: "sonnet".to_string(),
                include_partial_messages: true,
                permission_mode: "bypassPermissions".to_string(),
            },
        }
    }
}

#[async_trait]
impl CodingAgentExecutor for ClaudeCodeExecutor {
    type Config = ClaudeCodeConfig;
    
    fn agent_type(&self) -> &'static str {
        "claude-code"
    }
    
    fn build_command(&self, prompt: &str, env: &ExecutionEnv) -> Vec<String> {
        vec![
            "npx".to_string(),
            "-y".to_string(),
            "@anthropic-ai/claude-code@2.1.7".to_string(),
            "--output-format=stream-json".to_string(),
            "--include-partial-messages".to_string(),
            format!("--permission-mode={}", self.config.permission_mode),
            "-p".to_string(),
            prompt.to_string(),
        ]
    }
    
    fn build_env_vars(&self, env: &ExecutionEnv) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        if let Some(key) = env.api_keys.get("ANTHROPIC_API_KEY") {
            vars.insert("ANTHROPIC_API_KEY".to_string(), key.clone());
        }
        vars
    }
    
    fn parse_message(&self, line: &str) -> Option<NormalizedMessage> {
        let msg: serde_json::Value = serde_json::from_str(line).ok()?;
        let msg_type = msg.get("type")?.as_str()?;
        
        match msg_type {
            "thinking" => Some(NormalizedMessage::Thinking {
                content: msg.get("content")?.as_str()?.to_string(),
            }),
            "tool_use" => Some(NormalizedMessage::ToolUse {
                tool_name: msg.get("name")?.as_str()?.to_string(),
                input: msg.get("input")?.clone(),
                status: ToolStatus::Running,
            }),
            "tool_result" => Some(NormalizedMessage::ToolResult {
                tool_name: "unknown".to_string(),
                output: msg.get("content")?.as_str()?.to_string(),
                status: if msg.get("is_error")?.as_bool()? {
                    ToolStatus::Failed
                } else {
                    ToolStatus::Success
                },
            }),
            "result" => Some(NormalizedMessage::AssistantMessage {
                content: msg.get("content")?.as_str()?.to_string(),
            }),
            "error" => Some(NormalizedMessage::ErrorMessage {
                message: msg.get("message")?.as_str()?.to_string(),
            }),
            _ => None,
        }
    }
    
    async fn check_availability(&self) -> Result<bool> {
        let output = Command::new("npx")
            .arg("--version")
            .output()
            .await?;
        Ok(output.status.success())
    }
    
    fn default_config() -> Self::Config {
        ClaudeCodeConfig {
            model: "sonnet".to_string(),
            include_partial_messages: true,
            permission_mode: "bypassPermissions".to_string(),
        }
    }
    
    async fn execute(
        &self,
        prompt: &str,
        env: ExecutionEnv,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<NormalizedMessage>> + Send>>> {
        // 构建命令
        let cmd_parts = self.build_command(prompt, &env);
        let env_vars = self.build_env_vars(&env);
        
        // 启动进程
        let mut child = Command::new(&cmd_parts[0])
            .args(&cmd_parts[1..])
            .envs(env_vars)
            .current_dir(&env.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ExecutorError::ProcessSpawn(e.to_string()))?;
        
        // 获取 stdout
        let stdout = child.stdout.take()
            .ok_or_else(|| ExecutorError::ProcessSpawn("Failed to capture stdout".to_string()))?;
        
        // 创建异步流
        let stream = BufReader::new(stdout)
            .lines()
            .filter_map(move |line_result| async move {
                match line_result {
                    Ok(line) => {
                        if line.trim().is_empty() {
                            return None;
                        }
                        // 解析消息
                        match self.parse_message(&line) {
                            Some(msg) => Some(Ok(msg)),
                            None => Some(Err(ExecutorError::MessageParsing(
                                serde_json::Error::custom("Failed to parse message")
                            ))),
                        }
                    }
                    Err(e) => Some(Err(ExecutorError::Io(e))),
                }
            });
        
        Ok(Box::pin(stream))
    }
}
```

**优势**:
- **异步流式处理** - 使用 `Stream` trait 进行高效的异步迭代
- **零拷贝** - 直接从 stdout 流式读取，无需缓冲整个输出
- **背压支持** - Stream 自动处理背压，防止内存溢出
- **组合性** - 可以使用 `StreamExt` 的各种组合子（map, filter, etc.）

---

## 配置管理设计

### Builder 模式构建执行环境

```rust
/// 执行环境构建器
#[derive(Debug, Default)]
pub struct ExecutionEnvBuilder {
    working_dir: Option<PathBuf>,
    api_keys: HashMap<String, String>,
    timeout_secs: Option<u64>,
}

impl ExecutionEnvBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
    
    pub fn api_key(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.api_keys.insert(key.into(), value.into());
        self
    }
    
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
    
    pub fn build(self) -> Result<ExecutionEnv> {
        let working_dir = self.working_dir
            .ok_or_else(|| ExecutorError::InvalidConfig(
                "working_dir is required".to_string()
            ))?;
        
        Ok(ExecutionEnv {
            working_dir,
            api_keys: self.api_keys,
            timeout_secs: self.timeout_secs,
        })
    }
}

/// Agent 配置构建器
#[derive(Debug)]
pub struct AgentBuilder {
    agent_type: Option<String>,
    config: Option<serde_json::Value>,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            agent_type: None,
            config: None,
        }
    }
    
    pub fn agent_type(mut self, agent_type: impl Into<String>) -> Self {
        self.agent_type = Some(agent_type.into());
        self
    }
    
    pub fn config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
    
    pub fn build(self) -> Result<Agent> {
        let agent_type = self.agent_type
            .ok_or_else(|| ExecutorError::InvalidConfig(
                "agent_type is required".to_string()
            ))?;
        
        Agent::from_type(&agent_type)
    }
}
```

**使用示例**:

```rust
// 构建执行环境
let env = ExecutionEnvBuilder::new()
    .working_dir("/tmp/test")
    .api_key("ANTHROPIC_API_KEY", "sk-xxx")
    .timeout(300)
    .build()?;

// 构建 Agent
let agent = AgentBuilder::new()
    .agent_type("claude-code")
    .build()?;

// 执行任务
let stream = agent.execute("Add docstring to test.py", env).await?;

// 处理消息流
tokio::pin!(stream);
while let Some(result) = stream.next().await {
    match result {
        Ok(msg) => println!("Message: {:?}", msg),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

**优势**:
- **类型安全** - 编译时检查必需字段
- **链式调用** - 流畅的 API 设计
- **默认值** - 使用 `Default` trait 提供合理默认值
- **验证** - 在 `build()` 时进行配置验证

---

## 设计对比

### 旧设计 vs 新设计

#### 旧设计（Trait Object）

```rust
// 使用 Box<dyn Trait> 动态分发
pub trait CodingAgentExecutor {
    fn agent_type(&self) -> AgentType;
    fn build_exec_command(&self, task: &Task, env: &ExecutionEnv) -> Vec<String>;
    // ...
}

// 运行时多态
let executor: Box<dyn CodingAgentExecutor> = match agent_type {
    AgentType::ClaudeCode => Box::new(ClaudeCodeExecutor::new()),
    AgentType::GeminiCli => Box::new(GeminiCliExecutor::new()),
    // ...
};
```

**问题**:
- ❌ 动态分发有运行时开销
- ❌ 无法使用关联类型
- ❌ 难以优化（编译器无法内联）
- ❌ 需要堆分配（Box）

#### 新设计（Enum Dispatch）

```rust
// 使用 enum 静态分发
pub enum Agent {
    ClaudeCode(ClaudeCodeExecutor),
    GeminiCli(GeminiCliExecutor),
    OpenCodeAcp(OpenCodeAcpExecutor),
}

impl Agent {
    pub async fn execute(&self, prompt: &str, env: ExecutionEnv) 
        -> Result<Pin<Box<dyn Stream<Item = Result<NormalizedMessage>> + Send>>> 
    {
        match self {
            Agent::ClaudeCode(executor) => executor.execute(prompt, env).await,
            Agent::GeminiCli(executor) => executor.execute(prompt, env).await,
            Agent::OpenCodeAcp(executor) => executor.execute(prompt, env).await,
        }
    }
}
```

**优势**:
- ✅ 零成本抽象（编译器完全内联）
- ✅ 支持关联类型
- ✅ 编译器优化友好
- ✅ 栈分配（无 Box 开销）
- ✅ 穷尽性检查

---

## 完整使用示例

### 示例 1: 执行 Claude Code 任务

```rust
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 构建执行环境
    let env = ExecutionEnvBuilder::new()
        .working_dir("/tmp/demo")
        .api_key("ANTHROPIC_API_KEY", std::env::var("ANTHROPIC_API_KEY")?)
        .timeout(300)
        .build()?;
    
    // 2. 创建 Agent
    let agent = Agent::from_type("claude-code")?;
    
    // 3. 检查可用性
    if !agent.check_availability().await? {
        return Err(ExecutorError::AgentNotAvailable {
            agent_type: agent.agent_type().to_string(),
        });
    }
    
    // 4. 执行任务
    let prompt = "Add docstring to all functions in test.py";
    let stream = agent.execute(prompt, env).await?;
    
    // 5. 处理消息流
    tokio::pin!(stream);
    while let Some(result) = stream.next().await {
        match result {
            Ok(NormalizedMessage::Thinking { content }) => {
                println!("💭 Thinking: {}", content);
            }
            Ok(NormalizedMessage::ToolUse { tool_name, input, .. }) => {
                println!("🔧 Tool: {} with {:?}", tool_name, input);
            }
            Ok(NormalizedMessage::ToolResult { tool_name, output, status }) => {
                println!("✅ Result from {}: {:?} (status: {:?})", tool_name, output, status);
            }
            Ok(NormalizedMessage::AssistantMessage { content }) => {
                println!("🤖 Assistant: {}", content);
            }
            Ok(NormalizedMessage::ErrorMessage { message }) => {
                eprintln!("❌ Error: {}", message);
            }
            Err(e) => {
                eprintln!("⚠️  Stream error: {}", e);
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

### 示例 2: 多 Agent 并发执行

```rust
use futures::future::join_all;

async fn run_multiple_agents() -> Result<()> {
    let env = ExecutionEnvBuilder::new()
        .working_dir("/tmp/demo")
        .api_key("ANTHROPIC_API_KEY", "sk-xxx")
        .api_key("GEMINI_API_KEY", "yyy")
        .build()?;
    
    let agents = vec![
        Agent::from_type("claude-code")?,
        Agent::from_type("gemini-cli")?,
    ];
    
    let prompt = "Explain the main function";
    
    // 并发执行多个 agent
    let tasks = agents.into_iter().map(|agent| {
        let env = env.clone();
        let prompt = prompt.to_string();
        
        tokio::spawn(async move {
            let stream = agent.execute(&prompt, env).await?;
            
            tokio::pin!(stream);
            let mut messages = Vec::new();
            while let Some(result) = stream.next().await {
                if let Ok(msg) = result {
                    messages.push(msg);
                }
            }
            
            Ok::<_, ExecutorError>((agent.agent_type(), messages))
        })
    });
    
    let results = join_all(tasks).await;
    
    for result in results {
        match result {
            Ok(Ok((agent_type, messages))) => {
                println!("Agent {}: {} messages", agent_type, messages.len());
            }
            Ok(Err(e)) => eprintln!("Agent error: {}", e),
            Err(e) => eprintln!("Task error: {}", e),
        }
    }
    
    Ok(())
}
```

---

## 架构总结

### 核心组件

```
┌─────────────────────────────────────────────────────────┐
│                    ExecutionService                      │
│  - 管理 Agent 生命周期                                    │
│  - 协调消息流                                             │
│  - 持久化到数据库                                         │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                        Agent (Enum)                      │
│  ┌─────────────────┐  ┌─────────────────┐              │
│  │ ClaudeCode      │  │ GeminiCli       │              │
│  │ Executor        │  │ Executor        │  ...         │
│  └─────────────────┘  └─────────────────┘              │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              CodingAgentExecutor Trait                   │
│  - execute() -> Stream<NormalizedMessage>               │
│  - parse_message()                                       │
│  - check_availability()                                  │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                    NormalizedMessage                     │
│  - UserMessage                                           │
│  - AssistantMessage                                      │
│  - Thinking                                              │
│  - ToolUse / ToolResult                                  │
│  - ErrorMessage                                          │
└─────────────────────────────────────────────────────────┘
```

### 关键特性

1. **类型安全** ✅
   - 使用 thiserror 定义清晰的错误类型
   - 编译时检查所有 agent 类型
   - 关联类型确保配置类型安全

2. **零成本抽象** ✅
   - Enum dispatch 避免动态分发
   - 编译器可以完全内联
   - 无虚函数表开销

3. **异步优先** ✅
   - 使用 Stream trait 进行异步迭代
   - 支持背压和流控制
   - Tokio 生态集成

4. **易于扩展** ✅
   - 添加新 agent 只需实现 trait
   - 在 Agent enum 中添加新 variant
   - 编译器确保所有地方都更新

5. **可测试性** ✅
   - 每个组件都可以独立测试
   - Mock 友好的接口设计
   - 清晰的错误处理

---

## 实施建议

### Phase 1: 核心抽象（1 周）

1. **定义核心类型**
   - `ExecutorError` with thiserror
   - `NormalizedMessage` enum
   - `CodingAgentExecutor` trait

2. **实现 Agent enum**
   - 基础结构
   - 工厂方法
   - 统一接口

3. **单元测试**
   - 错误类型测试
   - 消息解析测试

### Phase 2: Claude Code 实现（1 周）

1. **实现 ClaudeCodeExecutor**
   - 命令构建
   - 消息解析
   - Stream 处理

2. **集成测试**
   - 端到端测试
   - 错误场景测试

### Phase 3: 多 Agent 支持（1 周）

1. **实现 GeminiCliExecutor**
   - 复用 Claude Code 逻辑
   - 适配消息格式

2. **实现 OpenCodeAcpExecutor**
   - ACP 协议集成
   - 事件流处理

3. **性能测试**
   - 并发执行测试
   - 内存使用分析

---

## 依赖项

```toml
[dependencies]
# 异步运行时
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# 错误处理
thiserror = "1"
anyhow = "1"

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 异步 trait
async-trait = "0.1"

# 日志
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## 总结

这个设计充分利用了 Rust 的语言特性：

- **类型系统** - 编译时安全
- **零成本抽象** - Enum dispatch
- **所有权** - 清晰的生命周期
- **异步** - Stream trait
- **错误处理** - thiserror

相比原始设计，新设计具有：
- ✅ 更好的性能（零成本抽象）
- ✅ 更强的类型安全
- ✅ 更清晰的错误处理
- ✅ 更易于扩展
- ✅ 更好的可测试性

**准备好开始实现了吗？** 🚀
