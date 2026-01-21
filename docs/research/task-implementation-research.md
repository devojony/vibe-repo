# Task 功能实现研究

**版本**: v0.1.0  
**日期**: 2026-01-20  
**状态**: 研究中

## 1. 现状分析

### 1.1 已实现内容

#### 数据库层
- ✅ Task Entity (`backend/src/entities/task.rs`)
- ✅ Migration (`m20260117_000005_create_tasks.rs`)
- ✅ TaskLog Entity 和 Migration

#### 数据模型
```rust
tasks 表字段:
- id: i32 (主键)
- workspace_id: i32 (外键 → workspaces)
- issue_number: i32 (Issue编号)
- issue_title: String (Issue标题)
- issue_body: Option<String> (Issue内容)
- task_status: String (任务状态, 默认"Pending")
- priority: String (优先级, 默认"Medium")
- assigned_agent_id: Option<i32> (外键 → agents, 可空)
- branch_name: Option<String> (分支名)
- pr_number: Option<i32> (PR编号)
- pr_url: Option<String> (PR链接)
- error_message: Option<String> (错误信息)
- retry_count: i32 (重试次数, 默认0)
- max_retries: i32 (最大重试次数, 默认3)
- started_at: Option<DateTimeUtc> (开始时间)
- completed_at: Option<DateTimeUtc> (完成时间)
- created_at: DateTimeUtc (创建时间)
- updated_at: DateTimeUtc (更新时间)
- deleted_at: Option<DateTimeUtc> (软删除时间)
```

#### 关系定义
- `belongs_to Workspace` (多对一, CASCADE DELETE)
- `belongs_to Agent` (多对一, SET NULL on delete)
- `has_many TaskLog` (一对多)

#### 索引
- `idx_tasks_workspace_id` - workspace查询优化
- `idx_tasks_status` - 状态过滤优化
- `idx_tasks_issue_number` - Issue关联查询
- `idx_tasks_assigned_agent_id` - Agent查询优化
- `idx_tasks_deleted_at` - 软删除查询优化

### 1.2 缺失内容

#### Service层
- ❌ TaskService (业务逻辑层)
- ❌ Task状态机管理
- ❌ Task执行器

#### API层
- ❌ Task CRUD API
- ❌ Task状态更新API
- ❌ Task执行控制API

#### 核心功能
- ❌ Issue → Task 转换逻辑
- ❌ Task执行流程
- ❌ Agent调度逻辑
- ❌ 重试机制
- ❌ 错误处理

## 2. 核心概念

### 2.1 Task生命周期

```
Issue创建 → Task创建 → Agent分配 → 执行 → PR创建 → 完成
   ↓          ↓          ↓        ↓       ↓        ↓
Webhook   Pending   Assigned  Running  Review  Completed
                                 ↓
                              失败 → 重试 → 达到上限 → Failed
```

### 2.2 Task状态定义

**建议状态枚举**:
- `Pending` - 待处理 (初始状态)
- `Assigned` - 已分配Agent
- `Running` - 执行中
- `Review` - PR已创建，等待审核
- `Completed` - 已完成
- `Failed` - 失败 (达到最大重试次数)
- `Cancelled` - 已取消

### 2.3 触发方式

1. **Webhook触发** (主要方式)
   - Issue创建/更新时触发
   - 自动创建Task
   - 实时性好 (秒级)

2. **轮询触发** (备选方式)
   - 定期轮询Git平台获取新Issue
   - 适合本地开发和内网环境
   - 详见: `docs/issue-polling-fallback-design.md`

3. **手动触发**
   - API手动创建Task
   - 用于测试或特殊场景

## 3. 实现方案

### 3.1 TaskService设计

#### 核心方法
```rust
impl TaskService {
    // CRUD操作
    async fn create_task(workspace_id, issue_number, issue_title, issue_body) -> Result<Task>
    async fn get_task_by_id(id) -> Result<Task>
    async fn list_tasks_by_workspace(workspace_id, filters) -> Result<Vec<Task>>
    async fn update_task_status(id, status) -> Result<Task>
    async fn soft_delete_task(id) -> Result<()>
    
    // 业务逻辑
    async fn assign_agent(task_id, agent_id) -> Result<Task>
    async fn start_task(task_id) -> Result<Task>
    async fn complete_task(task_id, pr_url) -> Result<Task>
    async fn fail_task(task_id, error_msg, error_type) -> Result<Task>
    async fn retry_task(task_id) -> Result<Task>
    async fn cancel_task(task_id) -> Result<Task>
    
    // 查询
    async fn get_pending_tasks(workspace_id) -> Result<Vec<Task>>
    async fn get_running_tasks(workspace_id) -> Result<Vec<Task>>
    async fn get_task_by_issue(workspace_id, issue_number) -> Result<Option<Task>>
}
```

### 3.2 API设计

#### RESTful端点
```
POST   /api/tasks                          - 创建Task
GET    /api/tasks/:id                      - 获取Task详情
GET    /api/workspaces/:id/tasks           - 列出Workspace的Tasks
PATCH  /api/tasks/:id/status               - 更新状态
DELETE /api/tasks/:id                      - 删除Task (软删除)

POST   /api/tasks/:id/assign               - 分配Agent
POST   /api/tasks/:id/start                - 开始执行
POST   /api/tasks/:id/complete             - 标记完成
POST   /api/tasks/:id/fail                 - 标记失败
POST   /api/tasks/:id/retry                - 重试
POST   /api/tasks/:id/cancel               - 取消
```

### 3.3 执行流程

#### 阶段1: Task创建 (Webhook触发)
```
1. Webhook接收Issue事件
2. 验证Issue是否需要处理 (标签、mention等)
3. 查找对应的Workspace
4. 创建Task记录 (状态: Pending)
5. 返回Task ID
```

#### 阶段2: Agent分配
```
1. 从Workspace获取可用Agents
2. 选择合适的Agent (负载均衡、能力匹配)
3. 更新Task.assigned_agent_id
4. 更新状态为Assigned
```

#### 阶段3: Task执行
```
1. 更新状态为Running
2. 记录started_at
3. 在容器中执行Agent命令
4. 实时记录日志到TaskLog
5. 监控执行状态
```

#### 阶段4: 结果处理
```
成功:
  1. Agent创建PR
  2. 更新pr_number和pr_url
  3. 更新状态为Review
  4. 记录completed_at

失败:
  1. 记录error_message和error_type
  2. retry_count++
  3. 如果 retry_count < max_retries:
     - 更新状态为Pending
     - 等待重试
  4. 否则:
     - 更新状态为Failed
     - 记录completed_at
```

## 4. 关键技术研究

### 4.1 Webhook → Task 触发流程

**现有实现** (`backend/src/api/webhooks/event_handler.rs`):
```rust
// 当前只处理 issue_comment 事件
pub async fn handle_comment_event(comment_info: CommentInfo) -> Result<()> {
    // 检测 @mention
    let has_mention = detect_mention(&comment_info.comment_body, bot_username);
    
    if has_mention {
        // TODO: 触发 AI agent workflow
    }
}
```

**需要扩展**:
1. 支持 `issues` 事件 (Issue创建/更新)
2. 解析Issue信息 (number, title, body, labels)
3. 查找对应的Workspace
4. 创建Task记录

**实现方案**:
```rust
// 新增 handle_issue_event
pub async fn handle_issue_event(
    issue_info: IssueInfo,
    db: &DatabaseConnection
) -> Result<()> {
    // 1. 根据repository_id查找workspace
    let workspace = find_workspace_by_repo(issue_info.repository_id, db).await?;
    
    // 2. 创建Task
    let task_service = TaskService::new(db.clone());
    let task = task_service.create_task(
        workspace.id,
        issue_info.number,
        issue_info.title,
        issue_info.body,
    ).await?;
    
    // 3. 触发Task执行 (异步)
    tokio::spawn(async move {
        execute_task(task.id).await
    });
    
    Ok(())
}
```

### 4.2 Agent执行环境

**容器架构** (已实现):
- ✅ ContainerService - 容器生命周期管理
- ✅ DockerService - Docker API封装
- ✅ 容器状态跟踪 (creating, running, stopped, exited, failed)
- ✅ 健康检查机制

**Agent配置** (`backend/src/entities/agent.rs`):
```rust
pub struct Agent {
    id: i32,
    workspace_id: i32,
    name: String,
    tool_type: String,      // "opencode", "aider", etc.
    enabled: bool,
    command: String,         // 执行命令
    env_vars: Json,          // 环境变量
    timeout: i32,            // 超时时间(秒)
}
```

**执行方案**:
```
Task执行 → 选择Agent → 在容器中执行Agent命令
                ↓
        docker exec workspace-{id} {agent.command} --issue {issue_number}
                ↓
        捕获输出 → 记录到TaskLog
```

**DockerService API** (已实现):
```rust
pub async fn exec_in_container(
    &self,
    container_id: &str,
    cmd: Vec<String>,
    timeout_secs: u64,
) -> Result<ExecOutput>

pub struct ExecOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}
```

**Task执行实现**:
```rust
async fn execute_task(task_id: i32, db: &DatabaseConnection) -> Result<()> {
    // 1. 获取Task和相关信息
    let task = get_task(task_id).await?;
    let workspace = get_workspace(task.workspace_id).await?;
    let agent = get_agent(task.assigned_agent_id).await?;
    let container = get_container_by_workspace(workspace.id).await?;
    
    // 2. 更新状态为Running
    update_task_status(task_id, "Running").await?;
    update_task_started_at(task_id, Utc::now()).await?;
    
    // 3. 构建命令
    let cmd = build_agent_command(&agent, &task)?;
    // 例如: ["opencode", "--issue", "123", "--title", "Fix bug"]
    
    // 4. 执行命令
    let docker = DockerService::new()?;
    let output = docker.exec_in_container(
        &container.container_id,
        cmd,
        agent.timeout as u64,
    ).await?;
    
    // 5. 记录日志
    log_task_output(task_id, &output.stdout, &output.stderr).await?;
    
    // 6. 处理结果
    if output.exit_code == 0 {
        // 成功: 查找PR URL
        let pr_url = extract_pr_url(&output.stdout)?;
        complete_task(task_id, pr_url).await?;
    } else {
        // 失败: 重试或标记失败
        handle_task_failure(task_id, &output.stderr).await?;
    }
    
    Ok(())
}

fn build_agent_command(agent: &Agent, task: &Task) -> Result<Vec<String>> {
    // 解析agent.command模板
    // 例如: "opencode --model claude-3.5"
    let mut cmd: Vec<String> = agent.command.split_whitespace()
        .map(|s| s.to_string())
        .collect();
    
    // 添加Issue参数
    cmd.push("--issue".to_string());
    cmd.push(task.issue_number.to_string());
    
    // 可选: 添加其他参数
    if let Some(title) = &task.issue_title {
        cmd.push("--title".to_string());
        cmd.push(title.clone());
    }
    
    Ok(cmd)
}
```

### 4.3 Agent调度策略

**简单策略** (Phase 1):
```rust
async fn select_agent(workspace_id: i32) -> Result<Agent> {
    // 1. 获取workspace的所有enabled agents
    let agents = agent_service.list_agents_by_workspace(workspace_id).await?;
    let enabled = agents.into_iter().filter(|a| a.enabled).collect();
    
    // 2. 选择第一个可用的agent (简单策略)
    enabled.first().ok_or(NoAgentAvailable)
}
```

**高级策略** (Phase 2+):
- 负载均衡: 选择当前任务数最少的Agent
- 能力匹配: 根据Issue标签选择专门的Agent
- 优先级: 高优先级Task优先分配

### 4.4 并发控制

**Workspace级别限制**:
```rust
// workspace表已有字段
max_concurrent_tasks: i32  // 默认值待定 (建议: 1-3)

async fn can_start_task(workspace_id: i32) -> Result<bool> {
    let workspace = get_workspace(workspace_id).await?;
    let running_count = count_running_tasks(workspace_id).await?;
    
    Ok(running_count < workspace.max_concurrent_tasks)
}
```

**任务队列**:
```
Pending Tasks → 检查并发限制 → 可执行 → Running
                     ↓
                  达到上限 → 等待
```

### 4.5 重试机制

**数据库支持** (已有):
- `retry_count: i32` - 当前重试次数
- `max_retries: i32` - 最大重试次数 (默认3)

**重试策略**:
```rust
async fn handle_task_failure(task_id: i32, error: &str) -> Result<()> {
    let task = get_task(task_id).await?;
    
    if task.retry_count < task.max_retries {
        // 可重试
        update_task_status(task_id, "Pending").await?;
        increment_retry_count(task_id).await?;
        
        // 指数退避: 2^retry_count 分钟
        let delay = 2_u64.pow(task.retry_count as u32) * 60;
        tokio::time::sleep(Duration::from_secs(delay)).await;
        
        // 重新执行
        execute_task(task_id).await?;
    } else {
        // 达到上限，标记为Failed
        update_task_status(task_id, "Failed").await?;
        record_error(task_id, error).await?;
    }
    
    Ok(())
}
```

**错误分类**:
- 可重试: 网络错误、超时、临时资源不足
- 不可重试: 语法错误、权限错误、配置错误

### 4.6 日志和监控

**TaskLog表** (已实现):
```rust
pub struct TaskLog {
    id: i32,
    task_id: i32,
    log_level: String,      // "info", "error", "debug"
    message: String,
    created_at: DateTimeUtc,
}
```

**日志收集方案**:
```rust
async fn execute_task_with_logging(task_id: i32) -> Result<()> {
    let log_service = TaskLogService::new(db);
    
    // 记录开始
    log_service.log(task_id, "info", "Task started").await?;
    
    // 执行并捕获输出
    let output = docker.exec_in_container(
        container_id,
        &agent.command,
    ).await?;
    
    // 逐行记录输出
    for line in output.lines() {
        log_service.log(task_id, "info", line).await?;
    }
    
    // 记录完成
    log_service.log(task_id, "info", "Task completed").await?;
    
    Ok(())
}
```

## 5. 待研究问题

### 5.1 高优先级
- [ ] Docker exec API如何使用? (需要研究DockerService实现)
- [ ] 如何实时流式传输容器输出?
- [ ] Agent命令格式标准化 (如何传递Issue信息?)
- [ ] Task执行超时处理机制

### 5.2 中优先级
- [ ] 多Agent协作场景 (一个Task需要多个Agent?)
- [ ] Task依赖关系 (Task A完成后才能执行Task B)
- [ ] 资源清理策略 (失败的Task如何清理?)
- [ ] 性能优化 (大量Task并发时的数据库压力)

### 5.3 低优先级
- [ ] Task暂停/恢复功能
- [ ] Task优先级动态调整
- [ ] 跨Workspace的Task调度
- [ ] Task执行统计和分析

## 6. 完整执行流程

### 6.1 端到端流程图

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Issue创建 (Git Provider)                                     │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. Webhook接收 (POST /api/webhooks/:repository_id)              │
│    - 验证签名                                                    │
│    - 解析Issue事件                                               │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. 创建Task (handle_issue_event)                                │
│    - 查找Workspace (by repository_id)                            │
│    - 创建Task记录 (状态: Pending)                                │
│    - 异步触发执行                                                │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. 检查并发限制                                                  │
│    - 获取Workspace.max_concurrent_tasks                          │
│    - 统计当前Running状态的Task数量                               │
│    - 如果达到上限 → 等待                                         │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. 分配Agent                                                     │
│    - 获取Workspace的所有enabled Agents                           │
│    - 选择Agent (简单策略: 第一个可用)                            │
│    - 更新Task.assigned_agent_id                                  │
│    - 更新状态为Assigned                                          │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ 6. 执行Task                                                      │
│    - 更新状态为Running                                           │
│    - 记录started_at                                              │
│    - 构建Agent命令                                               │
│    - docker.exec_in_container()                                  │
│    - 实时记录stdout/stderr到TaskLog                              │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ 7. 处理结果                                                      │
│    ├─ 成功 (exit_code = 0)                                      │
│    │   - 解析PR URL                                             │
│    │   - 更新pr_url, pr_number                                  │
│    │   - 更新状态为Review                                        │
│    │   - 记录completed_at                                        │
│    │                                                             │
│    └─ 失败 (exit_code != 0)                                     │
│        - 记录error_message                                       │
│        - retry_count++                                           │
│        ├─ retry_count < max_retries                             │
│        │   - 更新状态为Pending                                   │
│        │   - 延迟后重新执行                                      │
│        └─ retry_count >= max_retries                            │
│            - 更新状态为Failed                                    │
│            - 记录completed_at                                    │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 状态转换图

```
                    ┌──────────┐
                    │ Pending  │ ◄─── 创建Task / 重试
                    └────┬─────┘
                         │
                         ▼
                    ┌──────────┐
                    │ Assigned │ ◄─── 分配Agent
                    └────┬─────┘
                         │
                         ▼
                    ┌──────────┐
                    │ Running  │ ◄─── 开始执行
                    └────┬─────┘
                         │
                ┌────────┴────────┐
                │                 │
                ▼                 ▼
          ┌──────────┐      ┌──────────┐
          │  Review  │      │  Failed  │
          └──────────┘      └────┬─────┘
                │                 │
                ▼                 │
          ┌──────────┐            │
          │Completed │            │
          └──────────┘            │
                                  │
                            retry_count < max_retries
                                  │
                                  ▼
                            ┌──────────┐
                            │ Pending  │
                            └──────────┘
```

## 7. 详细实现计划

### Phase 1: 基础CRUD和状态管理 (2-3天)

**目标**: 实现Task的基本CRUD操作和状态管理

**任务清单**:
1. ✅ 数据库Schema (已完成)
2. ✅ Task Entity (已完成)
3. [ ] TaskService实现
   - `create_task()` - 创建Task
   - `get_task_by_id()` - 获取Task
   - `list_tasks_by_workspace()` - 列出Tasks
   - `update_task_status()` - 更新状态
   - `soft_delete_task()` - 软删除
4. [ ] Task API实现
   - `POST /api/tasks` - 创建
   - `GET /api/tasks/:id` - 获取
   - `GET /api/workspaces/:id/tasks` - 列表
   - `PATCH /api/tasks/:id/status` - 更新状态
   - `DELETE /api/tasks/:id` - 删除
5. [ ] 单元测试 (TDD)
6. [ ] 集成测试
7. [ ] OpenAPI文档

**验收标准**:
- 所有CRUD操作正常工作
- 状态转换验证正确
- 测试覆盖率 > 80%
- API文档完整

### Phase 2: Webhook集成 (1-2天)

**目标**: 实现Issue事件 → Task创建的自动化流程

**任务清单**:
1. [ ] 扩展Webhook处理
   - 添加`issues`事件支持 (目前只有`issue_comment`)
   - 解析Issue payload
2. [ ] 实现`handle_issue_event()`
   - 根据repository_id查找Workspace
   - 创建Task记录
   - 异步触发执行
3. [ ] 添加过滤逻辑
   - 检查Issue标签 (例如: 只处理带`vibe-auto`标签的Issue)
   - 检查@mention
4. [ ] 测试
   - 模拟Webhook请求
   - 验证Task创建

**验收标准**:
- Issue创建时自动创建Task
- 过滤逻辑正确
- Webhook签名验证通过

### Phase 3: Agent分配和执行 (3-4天)

**目标**: 实现Task的自动执行流程

**任务清单**:
1. [ ] Agent选择逻辑
   - 实现简单策略 (第一个enabled Agent)
   - 添加`assign_agent()` API
2. [ ] Task执行器
   - 实现`execute_task()`核心逻辑
   - 构建Agent命令
   - 调用`docker.exec_in_container()`
   - 捕获输出
3. [ ] 并发控制
   - 检查`max_concurrent_tasks`
   - 实现任务队列
4. [ ] 结果处理
   - 解析PR URL
   - 更新Task状态
   - 记录完成时间
5. [ ] 测试
   - Mock Docker执行
   - 测试成功/失败场景

**验收标准**:
- Task能在容器中成功执行
- 并发限制生效
- 执行结果正确记录

### Phase 4: 日志和监控 (2天)

**目标**: 实现Task执行日志的收集和查询

**任务清单**:
1. [ ] TaskLogService实现
   - `create_log()` - 创建日志
   - `list_logs_by_task()` - 查询日志
2. [ ] 实时日志收集
   - 流式读取stdout/stderr
   - 逐行写入TaskLog
3. [ ] TaskLog API
   - `GET /api/tasks/:id/logs` - 获取日志
   - 支持分页
   - 支持日志级别过滤
4. [ ] 测试

**验收标准**:
- 执行日志完整记录
- 日志查询性能良好
- 支持实时查看

### Phase 5: 重试机制 (2天)

**目标**: 实现失败Task的自动重试

**任务清单**:
1. [ ] 错误分类
   - 定义可重试错误类型
   - 定义不可重试错误类型
2. [ ] 重试逻辑
   - 实现`retry_task()` API
   - 指数退避策略
   - 更新retry_count
3. [ ] 失败处理
   - 达到max_retries后标记Failed
   - 记录error_message和error_type
4. [ ] 测试
   - 测试重试流程
   - 测试退避策略

**验收标准**:
- 失败Task自动重试
- 重试次数限制生效
- 退避策略正确

### Phase 6: 高级特性 (可选, 3-5天)

**目标**: 实现高级调度和监控功能

**任务清单**:
1. [ ] 优先级调度
   - 支持Task优先级
   - 高优先级Task优先执行
2. [ ] 负载均衡
   - 多Agent负载均衡
   - 基于当前任务数选择Agent
3. [ ] 性能监控
   - 记录执行时间
   - 统计成功率
   - 资源使用监控
4. [ ] 通知机制
   - Task完成通知
   - Task失败告警

**验收标准**:
- 优先级调度正常工作
- 负载均衡效果明显
- 监控数据准确

## 8. 技术决策记录

### 8.1 为什么在容器中执行Agent?

**决策**: Agent在Docker容器中执行，而不是宿主机

**理由**:
1. **隔离性**: 每个Workspace有独立的容器环境
2. **安全性**: 限制Agent的文件系统访问
3. **资源控制**: 通过Docker限制CPU/内存
4. **一致性**: 所有Workspace使用相同的执行环境
5. **清理简单**: 删除容器即可清理所有资源

**权衡**:
- 优点: 安全、隔离、可控
- 缺点: 需要Docker环境、性能开销

### 8.2 为什么使用异步执行?

**决策**: Task执行使用`tokio::spawn`异步执行

**理由**:
1. **非阻塞**: Webhook响应不等待Task完成
2. **并发**: 支持多个Task同时执行
3. **超时处理**: 长时间运行的Task不阻塞API
4. **用户体验**: 快速返回Task ID

**实现**:
```rust
// Webhook handler
tokio::spawn(async move {
    execute_task(task_id).await
});
```

### 8.3 为什么使用软删除?

**决策**: Task使用软删除 (deleted_at字段)

**理由**:
1. **审计**: 保留历史记录
2. **恢复**: 可以恢复误删除的Task
3. **统计**: 可以统计所有Task (包括已删除)
4. **关联**: 保持与TaskLog的关联

**实现**:
- 查询时过滤`deleted_at IS NULL`
- 删除时设置`deleted_at = NOW()`

### 8.4 Agent命令格式标准

**决策**: Agent命令使用统一的参数格式

**格式**:
```bash
{agent.command} --issue {issue_number} --title "{issue_title}" --body "{issue_body}"
```

**示例**:
```bash
opencode --model claude-3.5 --issue 123 --title "Fix login bug" --body "..."
aider --model gpt-4 --issue 456 --title "Add feature" --body "..."
```

**理由**:
1. **标准化**: 所有Agent使用相同的参数格式
2. **可扩展**: 容易添加新参数
3. **可测试**: 容易构造测试命令

## 9. 风险和挑战

### 9.1 性能风险

**问题**: 大量Task并发执行时的性能瓶颈

**影响**:
- 数据库连接池耗尽
- Docker API限流
- 内存占用过高

**缓解措施**:
1. 实现并发限制 (max_concurrent_tasks)
2. 使用连接池
3. 实现任务队列
4. 监控资源使用

### 9.2 错误处理复杂性

**问题**: Agent执行可能出现各种错误

**挑战**:
- 网络错误
- 超时
- 容器崩溃
- Agent bug
- 资源不足

**缓解措施**:
1. 详细的错误分类
2. 完善的日志记录
3. 合理的重试策略
4. 告警机制

### 9.3 Docker依赖

**问题**: 系统强依赖Docker环境

**影响**:
- 部署复杂度增加
- Docker故障影响整个系统
- 需要Docker权限

**缓解措施**:
1. 提供Docker健康检查
2. 优雅降级 (Docker不可用时禁用Task执行)
3. 详细的部署文档

### 9.4 Agent输出解析

**问题**: 如何可靠地从Agent输出中提取PR URL?

**挑战**:
- 不同Agent输出格式不同
- 输出可能包含噪音
- 可能没有PR URL (失败情况)

**缓解措施**:
1. 定义Agent输出规范
2. 使用正则表达式提取
3. 支持多种格式
4. 失败时记录完整输出

## 10. 下一步行动

### Phase 1: 基础CRUD (优先级: P0)
- [ ] 实现TaskService基础方法
- [ ] 实现Task CRUD API
- [ ] 编写单元测试
- [ ] 编写集成测试

### Phase 2: 状态管理 (优先级: P0)
- [ ] 实现状态机
- [ ] 实现状态转换验证
- [ ] 实现状态更新API

### Phase 3: Agent集成 (优先级: P1)
- [ ] 研究Agent调度策略
- [ ] 实现Agent分配逻辑
- [ ] 实现Agent执行接口

### Phase 4: 执行引擎 (优先级: P1)
- [ ] 研究执行环境方案
- [ ] 实现Task执行器
- [ ] 实现日志收集

### Phase 5: 高级特性 (优先级: P2)
- [ ] 实现重试机制
- [ ] 实现并发控制
- [ ] 实现优先级调度

## 10. 下一步行动

### 立即开始 (推荐)

**Phase 1: 基础CRUD** - 从这里开始
1. 参考 `docs/plans/2026-01-17-workspace-phase4-task-api.md`
2. 使用TDD方法实现TaskService
3. 实现Task CRUD API
4. 编写测试

**预计时间**: 2-3天

### 需要进一步研究的问题

1. **Agent命令参数传递**
   - [ ] 研究OpenCode的命令行参数格式
   - [ ] 研究Aider的命令行参数格式
   - [ ] 设计统一的参数传递接口

2. **PR URL提取**
   - [ ] 研究不同Agent的输出格式
   - [ ] 设计PR URL提取规则
   - [ ] 实现提取逻辑

3. **并发控制细节**
   - [ ] 研究任务队列实现方案
   - [ ] 设计优先级调度算法
   - [ ] 实现并发限制

4. **监控和告警**
   - [ ] 设计监控指标
   - [ ] 选择告警渠道 (邮件/Webhook)
   - [ ] 实现告警逻辑

## 11. 参考资料

### 内部文档
- `docs/plans/2026-01-17-workspace-phase4-task-api.md` - Task API实现计划
- `docs/issue-polling-fallback-design.md` - Issue轮询方案设计 (Webhook备选)
- `backend/src/entities/task.rs` - Task Entity定义
- `backend/src/migration/m20260117_000005_create_tasks.rs` - 数据库Schema
- `backend/src/services/docker_service.rs` - Docker执行API
- `backend/src/api/webhooks/event_handler.rs` - Webhook事件处理

### 相关模块
- Workspace模块 - Task的执行环境
- Agent模块 - Task的执行者
- Container模块 - Task的运行容器
- Webhook模块 - Task的触发源

### 技术栈
- SeaORM - ORM框架
- Axum - Web框架
- Bollard - Docker API客户端
- Tokio - 异步运行时

## 12. 更新日志

- **2026-01-20**: 初始版本
  - 完成现状分析
  - 完成核心概念设计
  - 完成技术方案研究
  - 完成实现计划制定
