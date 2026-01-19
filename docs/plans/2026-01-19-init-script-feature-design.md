# 初始化脚本功能设计

## 变更日期
2026-01-19

## 概述

将 workspace 模块的 `custom_dockerfile_path` 功能替换为初始化脚本功能。用户可以为每个工作区配置一个初始化脚本，该脚本将在容器启动成功后自动执行，用于安装依赖、配置环境等初始化操作。

## 需求分析

### 功能目标
- 取消用户自定义 Dockerfile 的功能
- 提供添加 shell 脚本的功能
- 脚本在容器启动后自动执行
- 记录脚本执行状态和输出
- 脚本执行失败时保留容器用于调试

### 设计决策
1. **脚本用途**：容器初始化脚本（安装依赖、配置环境）
2. **存储方式**：直接存储脚本内容在数据库中
3. **执行时机**：容器启动后立即自动执行
4. **失败处理**：标记为失败但保留容器用于调试
5. **脚本数量**：每个工作区支持单个初始化脚本

## 数据库设计

### 修改 `workspaces` 表

**移除字段**：
- `custom_dockerfile_path` (Option<String>)

### 新增 `init_scripts` 表

```sql
CREATE TABLE init_scripts (
    id SERIAL PRIMARY KEY,
    workspace_id INTEGER NOT NULL UNIQUE,
    script_content TEXT NOT NULL,
    timeout_seconds INTEGER NOT NULL DEFAULT 300,
    status VARCHAR(50) NOT NULL DEFAULT 'Pending',
    output TEXT,
    executed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_init_scripts_workspace_id
        FOREIGN KEY (workspace_id)
        REFERENCES workspaces(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE
);

CREATE UNIQUE INDEX idx_init_scripts_workspace_id ON init_scripts(workspace_id);
CREATE INDEX idx_init_scripts_status ON init_scripts(status);
```

### 实体定义

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "init_scripts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub workspace_id: i32,
    pub script_content: String,
    pub timeout_seconds: i32,  // 脚本执行超时时间（秒），默认 300
    pub status: String,  // "Pending", "Running", "Success", "Failed", "Timeout"
    pub output: Option<String>,
    pub executed_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}
```

### 关系定义

- **Workspace** `has_one` **InitScript**（一对一关系）
- **InitScript** `belongs_to` **Workspace**
- 外键约束：`init_scripts.workspace_id` → `workspaces.id`
  - ON DELETE: CASCADE（删除工作区时自动删除脚本）
  - ON UPDATE: CASCADE

### 脚本状态说明

| 状态 | 说明 |
|------|------|
| `Pending` | 脚本已创建，等待执行 |
| `Running` | 脚本正在执行中 |
| `Success` | 脚本执行成功 |
| `Failed` | 脚本执行失败（非零退出码） |
| `Timeout` | 脚本执行超时 |

## 迁移策略

创建新的迁移文件：`m20260119_000001_replace_dockerfile_with_init_script.rs`

**迁移步骤**：
1. 创建 `init_scripts` 表
2. 创建索引和外键约束
3. 删除 `workspaces` 表的 `custom_dockerfile_path` 列

**注意事项**：
- 现有数据中的 `custom_dockerfile_path` 将被丢弃（因为功能已变更）
- 迁移是不可逆的（down 迁移会丢失 init_scripts 数据）

---

## API 设计

### API 端点变更

#### 修改现有端点

**1. 创建工作区** - `POST /api/workspaces`

变更：
- 移除 `custom_dockerfile_path` 字段
- 新增 `init_script` 字段（可选）

**2. 获取工作区** - `GET /api/workspaces/:id`

变更：
- 响应中移除 `custom_dockerfile_path`
- 响应中包含关联的 `init_script` 信息（如果存在）

**3. 列表工作区** - `GET /api/workspaces`

变更：
- 响应中移除 `custom_dockerfile_path`
- 响应中包含关联的 `init_script` 信息（如果存在）

#### 新增端点

**4. 更新初始化脚本** - `PUT /api/workspaces/:id/init-script`

功能：更新或创建工作区的初始化脚本

请求体：
```json
{
  "script_content": "#!/bin/bash\napt-get update && apt-get install -y git",
  "execute_immediately": false
}
```

响应：`200 OK` 返回 `InitScriptResponse`

**5. 获取脚本执行日志** - `GET /api/workspaces/:id/init-script/logs`

功能：获取脚本执行的输出和错误信息

响应：
```json
{
  "status": "Success",
  "output": "Reading package lists...\nBuilding dependency tree...\nDone.",
  "executed_at": "2026-01-19T10:30:00Z"
}
```

**6. 重新执行脚本** - `POST /api/workspaces/:id/init-script/execute`

功能：手动触发脚本重新执行（用于失败后重试）

响应：`202 Accepted` 脚本开始执行

### 请求/响应模型

#### CreateWorkspaceRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceRequest {
    pub repository_id: i32,
    pub init_script: Option<String>,  // 新增：初始化脚本内容
    #[serde(default = "default_script_timeout")]
    pub script_timeout_seconds: i32,  // 新增：脚本超时时间（秒），默认 300
    #[serde(default = "default_image_source")]
    pub image_source: String,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: i32,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_memory_limit")]
    pub memory_limit: String,
    #[serde(default = "default_disk_limit")]
    pub disk_limit: String,
}

fn default_script_timeout() -> i32 {
    300  // 5 分钟
}
```

#### WorkspaceResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceResponse {
    pub id: i32,
    pub repository_id: i32,
    pub workspace_status: String,
    pub container_id: Option<String>,
    pub container_status: Option<String>,
    pub image_source: String,
    pub init_script: Option<InitScriptResponse>,  // 新增：关联的脚本信息
    pub max_concurrent_tasks: i32,
    pub cpu_limit: f64,
    pub memory_limit: String,
    pub disk_limit: String,
    pub work_dir: Option<String>,
    pub health_status: Option<String>,
    pub last_health_check: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}
```

#### InitScriptResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InitScriptResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub script_content: String,
    pub timeout_seconds: i32,  // 新增：超时时间
    pub status: String,  // "Pending", "Running", "Success", "Failed", "Timeout"
    pub output: Option<String>,
    pub executed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

#### UpdateInitScriptRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateInitScriptRequest {
    pub script_content: String,
    #[serde(default = "default_script_timeout")]
    pub timeout_seconds: i32,  // 新增：超时时间，默认 300
    #[serde(default)]
    pub execute_immediately: bool,  // 是否立即执行，默认 false
}

fn default_script_timeout() -> i32 {
    300  // 5 分钟
}
```

#### InitScriptLogsResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InitScriptLogsResponse {
    pub status: String,
    pub output: Option<String>,
    pub executed_at: Option<String>,
}
```

### API 行为说明

1. **创建工作区时提供脚本**：
   - 如果提供了 `init_script`，创建工作区后会自动创建 InitScript 记录
   - 容器启动成功后自动执行脚本

2. **更新脚本**：
   - 如果工作区没有脚本，则创建新脚本
   - 如果已有脚本，则更新脚本内容
   - `execute_immediately=true` 时立即执行（需要容器正在运行）

3. **脚本执行**：
   - 自动执行：容器启动后自动执行状态为 "Pending" 的脚本
   - 手动执行：通过 `/execute` 端点触发

4. **错误处理**：
   - 脚本执行失败不影响容器运行
   - 工作区状态保持 "Active"，但 init_script.status 为 "Failed"
   - 可以通过 `/logs` 查看错误信息
   - 可以通过 `/execute` 重新执行

---

## Service 层实现

### 新增 InitScriptService

创建专门的服务来管理初始化脚本的生命周期。

#### 服务结构

```rust
#[derive(Clone)]
pub struct InitScriptService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}

impl InitScriptService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }
}
```

#### 核心方法

**1. 创建脚本记录**

```rust
pub async fn create_init_script(
    &self,
    workspace_id: i32,
    script_content: String,
    timeout_seconds: i32,
) -> Result<init_script::Model>
```

功能：为工作区创建初始化脚本记录，状态为 "Pending"

**2. 获取工作区的脚本**

```rust
pub async fn get_init_script_by_workspace_id(
    &self,
    workspace_id: i32,
) -> Result<Option<init_script::Model>>
```

功能：查询工作区关联的脚本（一对一关系）

**3. 更新脚本内容**

```rust
pub async fn update_init_script(
    &self,
    workspace_id: i32,
    script_content: String,
    timeout_seconds: i32,
) -> Result<init_script::Model>
```

功能：更新现有脚本内容和超时时间，重置状态为 "Pending"

**4. 执行脚本**

```rust
pub async fn execute_script(
    &self,
    workspace_id: i32,
    container_id: &str,
) -> Result<init_script::Model>
```

功能：在容器中执行脚本，更新执行状态和输出

**5. 更新脚本状态**

```rust
pub async fn update_script_status(
    &self,
    script_id: i32,
    status: &str,
    output: Option<String>,
) -> Result<init_script::Model>
```

功能：更新脚本执行状态和输出信息

### WorkspaceService 集成

#### 修改 create_workspace_with_container 方法

```rust
pub async fn create_workspace_with_container(
    &self,
    repository_id: i32,
    init_script: Option<String>,
) -> Result<workspace::Model>
```

**执行流程**：

1. **创建工作区记录**
   ```rust
   let mut workspace = self.create_workspace(repository_id).await?;
   ```

2. **创建并启动容器**
   ```rust
   if let Some(docker) = &self.docker {
       let container_id = docker.create_container(...).await?;
       docker.start_container(&container_id).await?;
       // 更新工作区状态为 "Active"
   }
   ```

3. **创建初始化脚本记录（如果提供）**
   ```rust
   if let Some(script_content) = init_script {
       let init_script_service = InitScriptService::new(self.db.clone(), self.docker.clone());
       init_script_service.create_init_script(workspace.id, script_content).await?;
   }
   ```

4. **执行初始化脚本**
   ```rust
   if let Some(container_id) = &workspace.container_id {
       if let Some(script) = init_script_service.get_init_script_by_workspace_id(workspace.id).await? {
           if script.status == "Pending" {
               // 异步执行脚本，不阻塞工作区创建
               tokio::spawn(async move {
                   let _ = init_script_service.execute_script(workspace.id, container_id).await;
               });
           }
       }
   }
   ```

### 脚本执行流程

```
容器启动成功
    ↓
检查是否有 Pending 状态的脚本
    ↓
更新脚本状态为 "Running"
    ↓
在容器中执行脚本 (docker.exec_in_container)
    ↓
捕获标准输出和标准错误
    ↓
根据退出码判断成功/失败
    ↓
更新脚本状态为 "Success" 或 "Failed"
    ↓
记录执行时间和输出
    ↓
完成（不影响工作区状态）
```

### 关键设计决策

1. **异步执行**：脚本执行使用 `tokio::spawn` 异步执行，不阻塞工作区创建流程

2. **独立状态**：脚本执行失败不影响工作区状态，工作区保持 "Active"

3. **输出限制**：脚本输出存储在数据库中，建议限制大小（如 64KB）

4. **超时控制**：脚本执行使用配置的 `timeout_seconds`，默认 5 分钟（300 秒）

5. **幂等性**：可以多次执行同一个脚本（通过 `/execute` 端点）

6. **超时状态**：脚本执行超时时，状态设置为 "Timeout"，输出中包含超时信息

---

## Docker 集成

### DockerService 新增方法

在 `DockerService` 中添加在容器内执行命令的功能。

#### exec_in_container 方法

```rust
pub async fn exec_in_container(
    &self,
    container_id: &str,
    cmd: Vec<String>,
    timeout_secs: u64,
) -> Result<ExecOutput>
```

**功能**：
- 在运行中的容器内执行命令
- 捕获标准输出和标准错误
- 返回退出码
- 支持超时控制

**返回结构**：
```rust
#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}
```

### 脚本执行实现

#### 执行流程

1. **检查容器状态**
   - 确保容器正在运行
   - 如果容器未运行，返回错误

2. **创建 Exec 实例**
   ```rust
   let exec_config = CreateExecOptions {
       cmd: Some(vec!["/bin/bash", "-c", &script_content]),
       attach_stdout: Some(true),
       attach_stderr: Some(true),
       ..Default::default()
   };
   let exec = docker.create_exec(container_id, exec_config).await?;
   ```

3. **启动执行并设置超时**
   ```rust
   let timeout = Duration::from_secs(timeout_seconds as u64);
   let result = tokio::time::timeout(
       timeout,
       docker.start_exec(&exec.id, None)
   ).await;
   ```

4. **处理输出**
   - 流式读取标准输出和标准错误
   - 限制输出大小为 64KB
   - 超过限制时截断并添加提示信息

5. **获取退出码**
   ```rust
   let inspect = docker.inspect_exec(&exec.id).await?;
   let exit_code = inspect.exit_code.unwrap_or(-1);
   ```

6. **判断执行结果**
   - `exit_code == 0` → Success
   - `exit_code != 0` → Failed
   - 超时 → Timeout

### 技术细节

#### 使用 bollard API

```rust
use bollard::exec::{CreateExecOptions, StartExecResults};
use futures::StreamExt;

// 创建 exec
let exec = self.docker
    .create_exec(container_id, exec_config)
    .await?;

// 启动 exec 并获取输出流
if let StartExecResults::Attached { mut output, .. } =
    self.docker.start_exec(&exec.id, None).await?
{
    let mut stdout = String::new();
    let mut stderr = String::new();

    while let Some(Ok(msg)) = output.next().await {
        match msg {
            LogOutput::StdOut { message } => {
                stdout.push_str(&String::from_utf8_lossy(&message));
            }
            LogOutput::StdErr { message } => {
                stderr.push_str(&String::from_utf8_lossy(&message));
            }
            _ => {}
        }

        // 限制输出大小
        if stdout.len() + stderr.len() > 65536 {
            break;
        }
    }
}
```

#### 超时处理

```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(timeout_seconds as u64),
    execute_script_internal(container_id, script_content)
).await;

match result {
    Ok(Ok(output)) => {
        // 执行成功
        if output.exit_code == 0 {
            update_status("Success", Some(output.stdout)).await?;
        } else {
            update_status("Failed", Some(output.stderr)).await?;
        }
    }
    Ok(Err(e)) => {
        // 执行出错
        update_status("Failed", Some(e.to_string())).await?;
    }
    Err(_) => {
        // 超时
        update_status("Timeout", Some("Script execution timed out")).await?;
    }
}
```

#### 输出限制

```rust
const MAX_OUTPUT_SIZE: usize = 65536; // 64KB

fn truncate_output(output: String) -> String {
    if output.len() > MAX_OUTPUT_SIZE {
        let truncated = &output[..MAX_OUTPUT_SIZE];
        format!("{}\n\n[Output truncated at 64KB]", truncated)
    } else {
        output
    }
}
```

### 错误处理

| 错误场景 | 处理方式 |
|---------|---------|
| 容器未运行 | 返回错误，不更新脚本状态 |
| 脚本执行失败 | 状态设为 "Failed"，记录错误输出 |
| 脚本超时 | 状态设为 "Timeout"，记录超时信息 |
| Docker API 错误 | 状态设为 "Failed"，记录错误信息 |
| 输出过大 | 截断输出，添加截断提示 |

### 安全考虑

1. **脚本注入防护**：脚本内容直接传递给 bash，不进行字符串拼接
2. **资源限制**：依赖容器本身的资源限制（CPU、内存）
3. **超时保护**：强制超时避免无限执行
4. **输出限制**：限制输出大小避免数据库膨胀

---

## 错误处理策略

### API 层错误处理

#### HTTP 状态码映射

| 错误类型 | HTTP 状态码 | 说明 |
|---------|------------|------|
| 工作区不存在 | 404 Not Found | workspace_id 无效 |
| 脚本不存在 | 404 Not Found | 工作区没有关联的脚本 |
| 容器未运行 | 400 Bad Request | 无法执行脚本，容器未运行 |
| 验证错误 | 400 Bad Request | 请求参数无效 |
| Docker 不可用 | 503 Service Unavailable | Docker 服务不可用 |
| 内部错误 | 500 Internal Server Error | 数据库或其他内部错误 |

#### 错误响应格式

```json
{
  "error": "Container not running",
  "message": "Cannot execute script: container workspace-123 is not running",
  "code": "CONTAINER_NOT_RUNNING"
}
```

### Service 层错误处理

#### InitScriptService 错误场景

1. **创建脚本时工作区不存在**
   ```rust
   // 先验证工作区存在
   let workspace = Workspace::find_by_id(workspace_id)
       .one(&self.db)
       .await?
       .ok_or_else(|| VibeRepoError::NotFound(
           format!("Workspace {} not found", workspace_id)
       ))?;
   ```

2. **执行脚本时容器未运行**
   ```rust
   let workspace = self.get_workspace_by_id(workspace_id).await?;
   let container_id = workspace.container_id
       .ok_or_else(|| VibeRepoError::Validation(
           "Workspace has no container".to_string()
       ))?;

   // 检查容器状态
   if workspace.container_status != Some("running".to_string()) {
       return Err(VibeRepoError::Validation(
           "Container is not running".to_string()
       ));
   }
   ```

3. **Docker 服务不可用**
   ```rust
   let docker = self.docker.as_ref()
       .ok_or_else(|| VibeRepoError::ServiceUnavailable(
           "Docker service is not available".to_string()
       ))?;
   ```

4. **脚本执行失败**
   ```rust
   // 捕获所有执行错误，更新脚本状态
   match docker.exec_in_container(&container_id, cmd, timeout).await {
       Ok(output) => {
           // 处理输出
       }
       Err(e) => {
           // 更新状态为 Failed
           self.update_script_status(
               script.id,
               "Failed",
               Some(format!("Execution error: {}", e))
           ).await?;
           return Err(e);
       }
   }
   ```

### 数据库事务处理

#### 原子性保证

```rust
// 使用事务确保脚本创建和工作区更新的原子性
let txn = self.db.begin().await?;

// 创建脚本
let script = init_script::ActiveModel {
    workspace_id: Set(workspace_id),
    script_content: Set(script_content),
    timeout_seconds: Set(timeout_seconds),
    status: Set("Pending".to_string()),
    ..Default::default()
};
let script = InitScript::insert(script)
    .exec_with_returning(&txn)
    .await?;

txn.commit().await?;
```

### 日志记录

#### 关键操作日志

```rust
// 脚本创建
tracing::info!(
    workspace_id = workspace_id,
    script_id = script.id,
    "Created init script for workspace"
);

// 脚本开始执行
tracing::info!(
    workspace_id = workspace_id,
    script_id = script.id,
    container_id = container_id,
    "Starting init script execution"
);

// 脚本执行成功
tracing::info!(
    workspace_id = workspace_id,
    script_id = script.id,
    exit_code = output.exit_code,
    "Init script executed successfully"
);

// 脚本执行失败
tracing::error!(
    workspace_id = workspace_id,
    script_id = script.id,
    error = %e,
    "Init script execution failed"
);

// 脚本超时
tracing::warn!(
    workspace_id = workspace_id,
    script_id = script.id,
    timeout_seconds = timeout_seconds,
    "Init script execution timed out"
);
```

---

## 测试策略

### 单元测试

#### InitScriptService 测试

**测试文件**：`backend/src/services/init_script_service.rs`

**测试用例**：

1. **test_create_init_script_success**
   - 创建脚本记录
   - 验证默认状态为 "Pending"

2. **test_create_init_script_workspace_not_found**
   - 工作区不存在时返回错误

3. **test_get_init_script_by_workspace_id**
   - 获取存在的脚本
   - 获取不存在的脚本返回 None

4. **test_update_init_script_success**
   - 更新脚本内容和超时时间
   - 验证状态重置为 "Pending"

5. **test_update_script_status**
   - 更新脚本状态和输出
   - 验证 executed_at 时间戳

#### DockerService 测试

**测试文件**：`backend/src/services/docker_service.rs`

**测试用例**：

1. **test_exec_in_container_success**
   - 执行简单命令（echo "hello"）
   - 验证退出码为 0
   - 验证输出内容

2. **test_exec_in_container_failure**
   - 执行失败命令（exit 1）
   - 验证退出码非 0
   - 验证错误输出

3. **test_exec_in_container_timeout**
   - 执行长时间命令（sleep 60）
   - 设置短超时（5 秒）
   - 验证超时错误

4. **test_exec_in_container_output_limit**
   - 执行产生大量输出的命令
   - 验证输出被截断
   - 验证截断提示信息

### 集成测试

#### API 集成测试

**测试文件**：`backend/tests/workspaces/init_script_api_integration_tests.rs`

**测试用例**：

1. **test_create_workspace_with_init_script**
   - 创建工作区并提供脚本
   - 验证脚本记录被创建
   - 验证脚本状态为 "Pending"

2. **test_update_init_script**
   - 更新工作区的脚本
   - 验证脚本内容和超时时间更新

3. **test_get_workspace_includes_init_script**
   - 获取工作区详情
   - 验证响应包含脚本信息

4. **test_execute_init_script_success**
   - 手动触发脚本执行
   - 等待执行完成
   - 验证状态为 "Success"

5. **test_execute_init_script_failure**
   - 执行会失败的脚本（exit 1）
   - 验证状态为 "Failed"
   - 验证错误输出

6. **test_execute_init_script_timeout**
   - 执行长时间脚本
   - 设置短超时
   - 验证状态为 "Timeout"

7. **test_get_init_script_logs**
   - 获取脚本执行日志
   - 验证输出内容

8. **test_execute_script_container_not_running**
   - 容器未运行时执行脚本
   - 验证返回 400 错误

#### 端到端测试

**测试场景**：

1. **完整工作流测试**
   ```rust
   #[tokio::test]
   async fn test_workspace_with_init_script_e2e() {
       // 1. 创建工作区并提供脚本
       let workspace = create_workspace_with_script(
           "#!/bin/bash\napt-get update && apt-get install -y git"
       ).await;

       // 2. 等待容器启动
       wait_for_container_running(workspace.id).await;

       // 3. 等待脚本执行完成
       wait_for_script_completion(workspace.id).await;

       // 4. 验证脚本执行成功
       let script = get_init_script(workspace.id).await;
       assert_eq!(script.status, "Success");

       // 5. 验证 git 已安装
       let output = exec_in_workspace(workspace.id, "git --version").await;
       assert!(output.contains("git version"));
   }
   ```

### 测试辅助函数

```rust
// 创建测试工作区并提供脚本
async fn create_workspace_with_script(
    db: &DatabaseConnection,
    script_content: &str,
) -> workspace::Model {
    let repo = create_test_repository(db).await;
    let workspace = create_test_workspace(db, repo.id).await;
    let script_service = InitScriptService::new(db.clone(), None);
    script_service
        .create_init_script(workspace.id, script_content.to_string(), 300)
        .await
        .unwrap();
    workspace
}

// 等待脚本执行完成
async fn wait_for_script_completion(
    db: &DatabaseConnection,
    workspace_id: i32,
    timeout_secs: u64,
) -> init_script::Model {
    let start = std::time::Instant::now();
    loop {
        let script = InitScript::find()
            .filter(init_script::Column::WorkspaceId.eq(workspace_id))
            .one(db)
            .await
            .unwrap()
            .unwrap();

        if script.status != "Pending" && script.status != "Running" {
            return script;
        }

        if start.elapsed().as_secs() > timeout_secs {
            panic!("Script execution timeout");
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
```

### 测试覆盖目标

- **单元测试覆盖率**：> 80%
- **集成测试覆盖率**：所有 API 端点
- **关键路径测试**：100%（创建、执行、失败处理）

---

## 实施计划

### 第 1 阶段：数据库和实体

1. 创建迁移文件
2. 生成 init_script 实体
3. 更新 workspace 实体关系
4. 运行迁移并验证

### 第 2 阶段：Service 层

1. 实现 InitScriptService
2. 修改 WorkspaceService 集成
3. 添加单元测试
4. 验证业务逻辑

### 第 3 阶段：Docker 集成

1. 实现 exec_in_container 方法
2. 实现脚本执行逻辑
3. 添加超时和输出限制
4. 添加 Docker 测试

### 第 4 阶段：API 层

1. 更新 API 模型
2. 实现新的 API 端点
3. 更新现有端点
4. 更新 OpenAPI 文档

### 第 5 阶段：集成测试

1. 编写 API 集成测试
2. 编写端到端测试
3. 验证所有场景
4. 性能测试

### 第 6 阶段：文档和部署

1. 更新 API 文档
2. 编写用户指南
3. 更新 CHANGELOG
4. 部署到测试环境

---

## 总结

本设计方案将 `custom_dockerfile_path` 功能替换为更灵活的初始化脚本功能，主要特点：

1. **独立的脚本表**：使用关联表存储脚本，避免 workspace 表膨胀
2. **可配置超时**：每个脚本可以设置独立的超时时间
3. **完整的状态跟踪**：记录执行状态、输出和时间戳
4. **异步执行**：不阻塞工作区创建流程
5. **错误隔离**：脚本失败不影响工作区状态
6. **安全可靠**：超时保护、输出限制、错误处理

该设计遵循 YAGNI 原则，提供足够的功能同时保持简单性，为未来扩展留有空间。
