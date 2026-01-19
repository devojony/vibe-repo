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
    output_summary TEXT,
    output_file_path VARCHAR(500),
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
    pub output_summary: Option<String>,  // 输出摘要（最后 4KB）
    pub output_file_path: Option<String>,  // 完整输出文件路径
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

### 输出存储策略（混合存储）

#### 设计原则

1. **快速访问**：数据库存储输出摘要，用于快速查看和 API 响应
2. **完整保留**：文件系统存储完整输出，用于详细调试
3. **自动清理**：定期清理旧的输出文件，避免磁盘占用过大

#### 存储规则

| 输出大小 | 数据库 (output_summary) | 文件系统 (output_file_path) |
|---------|------------------------|----------------------------|
| ≤ 4KB | 完整输出 | 不创建文件 |
| > 4KB | 最后 4KB + 截断提示 | 完整输出 |

#### 文件存储路径

```
/data/gitautodev/init-script-logs/
  ├── workspace-{id}/
  │   └── script-{script_id}-{timestamp}.log
```

**路径格式**：
- 基础目录：`/data/gitautodev/init-script-logs/`
- 工作区目录：`workspace-{workspace_id}/`
- 文件名：`script-{script_id}-{unix_timestamp}.log`

**示例**：
```
/data/gitautodev/init-script-logs/workspace-123/script-456-1737283200.log
```

#### 文件清理策略

1. **保留时间**：默认保留 30 天
2. **清理触发**：
   - 定期任务（每天凌晨 2 点）
   - 工作区删除时立即清理
3. **清理规则**：
   - 删除超过保留期的日志文件
   - 删除空的工作区目录

#### 实现细节

**写入流程**：
```rust
async fn save_script_output(
    script_id: i32,
    workspace_id: i32,
    stdout: String,
    stderr: String,
) -> Result<(Option<String>, Option<String>)> {
    let full_output = format!("=== STDOUT ===\n{}\n\n=== STDERR ===\n{}", stdout, stderr);

    if full_output.len() <= 4096 {
        // 小于 4KB，只存数据库
        Ok((Some(full_output), None))
    } else {
        // 大于 4KB，混合存储
        let summary = extract_last_4kb(&full_output);
        let file_path = write_to_file(script_id, workspace_id, &full_output).await?;
        Ok((Some(summary), Some(file_path)))
    }
}

fn extract_last_4kb(output: &str) -> String {
    const MAX_SIZE: usize = 4096;
    if output.len() <= MAX_SIZE {
        output.to_string()
    } else {
        let start = output.len() - MAX_SIZE;
        format!("... [Output truncated, showing last 4KB]\n\n{}", &output[start..])
    }
}

async fn write_to_file(
    script_id: i32,
    workspace_id: i32,
    content: &str,
) -> Result<String> {
    let base_dir = "/data/gitautodev/init-script-logs";
    let workspace_dir = format!("{}/workspace-{}", base_dir, workspace_id);

    // 创建目录
    tokio::fs::create_dir_all(&workspace_dir).await?;

    // 生成文件名
    let timestamp = Utc::now().timestamp();
    let filename = format!("script-{}-{}.log", script_id, timestamp);
    let file_path = format!("{}/{}", workspace_dir, filename);

    // 写入文件
    tokio::fs::write(&file_path, content).await?;

    Ok(file_path)
}
```

**读取流程**：
```rust
async fn get_full_output(script: &init_script::Model) -> Result<String> {
    if let Some(file_path) = &script.output_file_path {
        // 从文件读取完整输出
        tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| VibeRepoError::Internal(
                format!("Failed to read output file: {}", e)
            ))
    } else {
        // 从数据库读取（小输出）
        Ok(script.output_summary.clone().unwrap_or_default())
    }
}
```

**清理任务**：
```rust
async fn cleanup_old_logs(retention_days: i64) -> Result<()> {
    let base_dir = "/data/gitautodev/init-script-logs";
    let cutoff_time = Utc::now() - Duration::days(retention_days);

    let mut entries = tokio::fs::read_dir(base_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            cleanup_workspace_logs(&entry.path(), cutoff_time).await?;
        }
    }

    Ok(())
}

async fn cleanup_workspace_logs(
    workspace_dir: &Path,
    cutoff_time: DateTime<Utc>,
) -> Result<()> {
    let mut entries = tokio::fs::read_dir(workspace_dir).await?;
    let mut has_files = false;

    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let modified = metadata.modified()?;

        if modified < cutoff_time.into() {
            tokio::fs::remove_file(entry.path()).await?;
        } else {
            has_files = true;
        }
    }

    // 如果目录为空，删除目录
    if !has_files {
        tokio::fs::remove_dir(workspace_dir).await?;
    }

    Ok(())
}
```

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

**5. 获取脚本执行摘要** - `GET /api/workspaces/:id/init-script/logs`

功能：获取脚本执行的输出摘要（最后 4KB）

响应：
```json
{
  "status": "Success",
  "output_summary": "... [Output truncated, showing last 4KB]\n\nReading package lists...\nBuilding dependency tree...\nDone.",
  "has_full_log": true,
  "executed_at": "2026-01-19T10:30:00Z"
}
```

**6. 下载完整日志** - `GET /api/workspaces/:id/init-script/logs/full`

功能：下载完整的脚本执行日志

响应：
- Content-Type: `text/plain`
- Content-Disposition: `attachment; filename="script-456-1737283200.log"`
- Body: 完整日志内容

**7. 重新执行脚本** - `POST /api/workspaces/:id/init-script/execute`

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
    pub timeout_seconds: i32,
    pub status: String,  // "Pending", "Running", "Success", "Failed", "Timeout"
    pub output_summary: Option<String>,  // 输出摘要（最后 4KB）
    pub has_full_log: bool,  // 是否有完整日志文件
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
    pub output_summary: Option<String>,  // 输出摘要
    pub has_full_log: bool,  // 是否有完整日志
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

3. **混合存储**：小输出（≤4KB）仅存数据库，大输出混合存储（数据库摘要 + 文件完整日志）

4. **超时控制**：脚本执行使用配置的 `timeout_seconds`，默认 5 分钟（300 秒）

5. **幂等性**：可以多次执行同一个脚本（通过 `/execute` 端点）

6. **超时状态**：脚本执行超时时，状态设置为 "Timeout"，输出中包含超时信息

7. **日志清理**：定期清理 30 天前的日志文件，工作区删除时立即清理

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
   - 合并 stdout 和 stderr
   - 根据大小决定存储策略（≤4KB 仅数据库，>4KB 混合存储）

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
    }

    // 保存输出（混合存储策略）
    let (summary, file_path) = save_script_output(
        script.id,
        workspace_id,
        stdout,
        stderr
    ).await?;
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
        // 执行成功，保存输出
        let (summary, file_path) = save_script_output(
            script.id,
            workspace_id,
            output.stdout,
            output.stderr
        ).await?;

        if output.exit_code == 0 {
            update_status("Success", summary, file_path).await?;
        } else {
            update_status("Failed", summary, file_path).await?;
        }
    }
    Ok(Err(e)) => {
        // 执行出错
        let error_msg = format!("Execution error: {}", e);
        update_status("Failed", Some(error_msg), None).await?;
    }
    Err(_) => {
        // 超时
        let timeout_msg = format!("Script execution timed out after {} seconds", timeout_seconds);
        update_status("Timeout", Some(timeout_msg), None).await?;
    }
}
```

#### 输出存储实现

参见前面"输出存储策略（混合存储）"章节的详细实现。

关键点：
- ≤ 4KB：仅存数据库
- > 4KB：数据库存摘要（最后 4KB），文件存完整输出
- 文件路径：`/data/gitautodev/init-script-logs/workspace-{id}/script-{id}-{timestamp}.log`

### 错误处理

| 错误场景 | 处理方式 |
|---------|---------|
| 容器未运行 | 返回错误，不更新脚本状态 |
| 脚本执行失败 | 状态设为 "Failed"，记录错误输出 |
| 脚本超时 | 状态设为 "Timeout"，记录超时信息 |
| Docker API 错误 | 状态设为 "Failed"，记录错误信息 |
| 文件写入失败 | 仅存数据库摘要，记录警告日志 |
| 文件读取失败 | 返回数据库摘要，提示完整日志不可用 |

### 安全考虑

1. **脚本注入防护**：脚本内容直接传递给 bash，不进行字符串拼接
2. **资源限制**：依赖容器本身的资源限制（CPU、内存）
3. **超时保护**：强制超时避免无限执行
4. **文件权限**：日志文件仅应用可读写，设置适当的文件权限
5. **路径安全**：使用固定的基础目录，防止路径遍历攻击
6. **磁盘配额**：通过定期清理控制磁盘使用

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

4. **test_exec_in_container_large_output**
   - 执行产生大量输出的命令（> 4KB）
   - 验证输出被正确存储到文件
   - 验证数据库只存储摘要

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
   - 获取脚本执行日志摘要
   - 验证输出内容

8. **test_download_full_log**
   - 下载完整日志文件
   - 验证文件内容完整性

9. **test_execute_script_container_not_running**
   - 容器未运行时执行脚本
   - 验证返回 400 错误

10. **test_log_file_cleanup**
    - 创建旧的日志文件
    - 触发清理任务
    - 验证旧文件被删除

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
4. **混合存储策略**：小输出存数据库，大输出存文件系统，兼顾性能和完整性
5. **异步执行**：不阻塞工作区创建流程
6. **错误隔离**：脚本失败不影响工作区状态
7. **自动清理**：定期清理旧日志文件，控制磁盘使用
8. **安全可靠**：超时保护、路径安全、错误处理

### 关键改进（相比初版设计）

**输出存储优化**：
- 初版：所有输出存数据库，限制 64KB
- 改进：混合存储，≤4KB 存数据库，>4KB 存文件 + 数据库摘要
- 优势：
  - 不丢失任何输出信息
  - 数据库不会因大输出膨胀
  - 快速访问常见场景（小输出）
  - 完整保留调试信息（大输出）

**API 增强**：
- 新增 `/logs/full` 端点下载完整日志
- 响应中包含 `has_full_log` 标识
- 支持流式下载大文件

**运维友好**：
- 自动清理 30 天前的日志
- 工作区删除时立即清理相关日志
- 文件组织清晰，便于手动管理

该设计遵循 YAGNI 原则，提供足够的功能同时保持简单性，为未来扩展留有空间。
