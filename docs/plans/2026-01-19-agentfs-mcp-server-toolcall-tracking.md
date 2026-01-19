# AgentFS MCP Server 与 ToolCall 追踪测试报告

## 测试日期
2026-01-19

## 关键发现

### ✅ AgentFS 支持 MCP Server 模式

AgentFS 可以作为 **MCP (Model Context Protocol) 服务器**运行，这是追踪 ToolCall 的正确方式。

## MCP Server 功能

### 启动命令
```bash
agentfs serve mcp <agent-id>
```

### 暴露的工具

#### 文件系统工具
| 工具 | 描述 |
|------|------|
| `read_file` | 读取文件内容 |
| `write_file` | 写入文件内容 |
| `readdir` | 列出目录内容 |
| `mkdir` | 创建目录 |
| `remove` | 删除文件或目录 |
| `rename` | 重命名文件或目录 |
| `stat` | 获取文件元数据 |
| `access` | 检查文件访问权限 |

#### Key-Value 工具
| 工具 | 描述 |
|------|------|
| `kv_get` | 通过 key 获取 value |
| `kv_set` | 设置 key-value 对 |
| `kv_delete` | 删除 key |
| `kv_list` | 列出所有 keys |

### 工具限制（安全）
```bash
# 只读模式
agentfs serve mcp my-agent --tools read_file,readdir,stat,kv_get,kv_list

# 仅文件系统
agentfs serve mcp my-agent --tools read_file,write_file,readdir,mkdir,remove

# 仅 KV 存储
agentfs serve mcp my-agent --tools kv_get,kv_set,kv_delete,kv_list
```

## ToolCall 追踪机制

### 数据库表结构

**tool_calls 表**：
```sql
CREATE TABLE tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,              -- 工具名称（如 read_file, write_file）
    parameters TEXT,                 -- 工具参数（JSON）
    result TEXT,                     -- 执行结果
    error TEXT,                      -- 错误信息
    status TEXT NOT NULL DEFAULT 'pending',  -- 状态：pending, completed, error
    started_at INTEGER NOT NULL,     -- 开始时间戳
    completed_at INTEGER,            -- 完成时间戳
    duration_ms INTEGER              -- 执行时长（毫秒）
);

CREATE INDEX idx_tool_calls_name ON tool_calls (name);
CREATE INDEX idx_tool_calls_started_at ON tool_calls (started_at);
```

### 记录时机

**自动记录**：
- 当 AI 助手（如 Claude Desktop）通过 MCP 协议调用工具时
- AgentFS 自动记录每次工具调用到 `tool_calls` 表

**不会记录**：
- 直接使用 `agentfs run` 执行的命令
- 手动文件操作
- 非 MCP 协议的操作

## VibeRepo 集成方案

### 方案概述

使用 AgentFS MCP Server 模式，让 AI Agent 通过 MCP 协议与文件系统交互，所有操作自动追踪。

### 架构设计

```
┌─────────────────┐
│   VibeRepo      │
│   Backend       │
└────────┬────────┘
         │
         │ 1. 创建 Workspace
         │    agentfs init <workspace-id>
         │
         ▼
┌─────────────────┐
│   AgentFS       │
│   Database      │
│  .agentfs/      │
│  workspace.db   │
└────────┬────────┘
         │
         │ 2. 启动 MCP Server
         │    agentfs serve mcp <workspace-id>
         │
         ▼
┌─────────────────┐
│  MCP Server     │
│  (stdio)        │
└────────┬────────┘
         │
         │ 3. MCP Protocol
         │    (JSON-RPC 2.0)
         │
         ▼
┌─────────────────┐
│  AI Agent       │
│  (Claude Code)  │
│                 │
│  Tool Calls:    │
│  - read_file    │
│  - write_file   │
│  - mkdir        │
│  - etc.         │
└────────┬────────┘
         │
         │ 4. 自动记录
         │
         ▼
┌─────────────────┐
│  tool_calls     │
│  表             │
│                 │
│  完整审计追踪   │
└─────────────────┘
```

### 实施步骤

#### 1. Workspace 创建时初始化 AgentFS

```rust
pub async fn create_workspace(repository_id: i32) -> Result<Workspace> {
    let workspace_id = format!("workspace-{}", Uuid::new_v4());
    let work_dir = format!("/data/workspaces/{}", workspace_id);

    // 创建工作目录
    fs::create_dir_all(&work_dir)?;

    // 初始化 AgentFS
    let status = Command::new("agentfs")
        .args(["init", "--base", &work_dir, &workspace_id])
        .current_dir(&work_dir)
        .status()?;

    if !status.success() {
        return Err(VibeRepoError::AgentFSInitFailed);
    }

    // 保存到数据库
    let workspace = workspaces::ActiveModel {
        repository_id: Set(repository_id),
        workspace_id: Set(workspace_id.clone()),
        work_dir: Set(work_dir.clone()),
        agentfs_db_path: Set(format!("{}/.agentfs/{}.db", work_dir, workspace_id)),
        ..Default::default()
    };

    Ok(workspace.insert(db).await?)
}
```

#### 2. 启动 MCP Server 并连接 Agent

**方式 A：容器内运行（推荐）**

```rust
pub async fn execute_task_with_mcp(
    workspace: &Workspace,
    task: &Task
) -> Result<TaskResult> {
    let workspace_id = &workspace.workspace_id;
    let work_dir = &workspace.work_dir;

    // 构建 Docker 命令，启动 MCP server 并连接 Claude Code
    let docker_cmd = format!(
        r#"
        # 启动 AgentFS MCP server（后台）
        agentfs serve mcp {workspace_id} > /tmp/mcp-server.log 2>&1 &
        MCP_PID=$!

        # 等待 MCP server 就绪
        sleep 1

        # 设置环境变量
        export ANTHROPIC_AUTH_TOKEN="{token}"
        export ANTHROPIC_BASE_URL="{base_url}"

        # 运行 Claude Code，连接到 MCP server
        # Claude Code 会通过 MCP 协议与 AgentFS 交互
        echo "{prompt}" | claude-code --mcp-server stdio

        # 停止 MCP server
        kill $MCP_PID
        "#,
        workspace_id = workspace_id,
        token = env::var("ANTHROPIC_AUTH_TOKEN")?,
        base_url = env::var("ANTHROPIC_BASE_URL")?,
        prompt = task.prompt
    );

    // 在容器中执行
    let status = Command::new("docker")
        .args([
            "run", "--rm",
            "--device", "/dev/fuse",
            "--cap-add", "SYS_ADMIN",
            "--security-opt", "apparmor=unconfined",
            "-v", &format!("{}/.agentfs:/root/.agentfs", work_dir),
            "-v", &format!("{}:/workspace", work_dir),
            "-w", "/workspace",
            &workspace.image,
            "bash", "-c", &docker_cmd
        ])
        .status()?;

    Ok(TaskResult {
        success: status.success(),
        exit_code: status.code(),
    })
}
```

**方式 B：宿主机运行 MCP Server**

```rust
pub async fn start_mcp_server(workspace: &Workspace) -> Result<Child> {
    let workspace_id = &workspace.workspace_id;
    let work_dir = &workspace.work_dir;

    // 启动 MCP server 进程
    let child = Command::new("agentfs")
        .args(["serve", "mcp", workspace_id])
        .current_dir(work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    Ok(child)
}

pub async fn execute_task_with_mcp_host(
    workspace: &Workspace,
    task: &Task
) -> Result<TaskResult> {
    // 启动 MCP server
    let mut mcp_server = start_mcp_server(workspace).await?;

    // 获取 stdin/stdout 用于 MCP 通信
    let stdin = mcp_server.stdin.take().unwrap();
    let stdout = mcp_server.stdout.take().unwrap();

    // 在容器中运行 Agent，连接到宿主机的 MCP server
    // 通过 stdin/stdout 进行 MCP 协议通信
    let result = execute_agent_with_mcp_connection(
        workspace,
        task,
        stdin,
        stdout
    ).await?;

    // 停止 MCP server
    mcp_server.kill()?;

    Ok(result)
}
```

#### 3. 查询 ToolCall 记录

```rust
pub async fn get_tool_calls(workspace: &Workspace) -> Result<Vec<ToolCall>> {
    let db_path = &workspace.agentfs_db_path;

    // 连接到 AgentFS SQLite 数据库
    let conn = SqliteConnection::establish(db_path)?;

    // 查询 tool_calls 表
    let tool_calls = sqlx::query_as::<_, ToolCall>(
        r#"
        SELECT
            id,
            name,
            parameters,
            result,
            error,
            status,
            started_at,
            completed_at,
            duration_ms
        FROM tool_calls
        ORDER BY started_at DESC
        "#
    )
    .fetch_all(&conn)
    .await?;

    Ok(tool_calls)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: i64,
    pub name: String,
    pub parameters: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
}
```

#### 4. API 端点

```rust
// GET /api/workspaces/:id/tool-calls
pub async fn get_workspace_tool_calls(
    Path(workspace_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ToolCall>>> {
    let workspace = workspaces::Entity::find_by_id(workspace_id)
        .one(&state.db)
        .await?
        .ok_or(VibeRepoError::WorkspaceNotFound)?;

    let tool_calls = get_tool_calls(&workspace).await?;

    Ok(Json(tool_calls))
}

// GET /api/workspaces/:id/tool-calls/:tool_call_id
pub async fn get_tool_call_detail(
    Path((workspace_id, tool_call_id)): Path<(i32, i64)>,
    State(state): State<AppState>,
) -> Result<Json<ToolCall>> {
    let workspace = workspaces::Entity::find_by_id(workspace_id)
        .one(&state.db)
        .await?
        .ok_or(VibeRepoError::WorkspaceNotFound)?;

    let db_path = &workspace.agentfs_db_path;
    let conn = SqliteConnection::establish(db_path)?;

    let tool_call = sqlx::query_as::<_, ToolCall>(
        "SELECT * FROM tool_calls WHERE id = ?"
    )
    .bind(tool_call_id)
    .fetch_one(&conn)
    .await?;

    Ok(Json(tool_call))
}
```

## 测试验证

### 手动测试步骤

1. **初始化 AgentFS**
   ```bash
   mkdir -p /tmp/test-workspace
   cd /tmp/test-workspace
   agentfs init --base . test-agent
   ```

2. **启动 MCP Server**
   ```bash
   agentfs serve mcp test-agent
   ```

3. **连接 MCP 客户端**
   - 使用 Claude Desktop 配置 MCP server
   - 或使用 MCP 测试工具

4. **执行工具调用**
   - 通过 MCP 协议调用 `write_file`, `read_file` 等工具

5. **查询记录**
   ```bash
   sqlite3 .agentfs/test-agent.db "SELECT * FROM tool_calls;"
   ```

### 预期结果

每次通过 MCP 协议的工具调用都会在 `tool_calls` 表中创建记录，包含：
- 工具名称
- 参数（JSON 格式）
- 执行结果
- 状态和时长

## 优势

### 1. 自动追踪
- 无需手动记录
- MCP 协议自动处理

### 2. 完整审计
- 所有工具调用都被记录
- 包含参数、结果、时长

### 3. 标准协议
- 使用 MCP 标准
- 兼容多种 AI 工具

### 4. 安全控制
- 可以限制暴露的工具
- 细粒度权限控制

## 注意事项

### 1. MCP Server 生命周期
- 需要管理 MCP server 进程
- 确保正确启动和停止

### 2. 通信方式
- MCP 使用 stdio 通信
- 需要正确处理进程间通信

### 3. 容器集成
- 在容器内运行 MCP server 需要特殊配置
- 或者在宿主机运行，容器通过 IPC 连接

### 4. 错误处理
- MCP 协议错误
- 工具执行失败
- 超时处理

## 下一步

1. **实现 MCP Server 管理**
   - 启动/停止 MCP server
   - 进程监控和重启

2. **实现 MCP 客户端连接**
   - 在容器内连接 MCP server
   - 或通过 IPC 连接宿主机 MCP server

3. **实现 ToolCall 查询 API**
   - 查询工具调用历史
   - 统计和分析

4. **测试完整流程**
   - 端到端测试
   - 性能测试

## 结论

AgentFS 的 MCP Server 模式提供了完整的 ToolCall 追踪能力。通过 MCP 协议，所有 AI Agent 的工具调用都会被自动记录到数据库中，提供完整的审计追踪。

**推荐方案**：
- 使用 AgentFS MCP Server 模式
- AI Agent 通过 MCP 协议与文件系统交互
- 自动获得完整的 ToolCall 记录

这比手动记录更可靠、更标准化，并且与 Claude Desktop 等工具完美集成。
