# AgentFS Docker 集成研究总结

## 研究完成时间

2025-01-18

## 研究目标

探索将 session.db 文件映射到 Docker 容器内，然后在容器内使用 `agentfs run` 运行 agent 的可行性。

## 测试结果

### ❌ 方案 B 不适用（容器内运行 agentfs run）

**核心问题：二进制兼容性**

1. **AgentFS 二进制版本**
   - 官方只提供 glibc 编译版本
   - 文件名：`agentfs-x86_64-unknown-linux-gnu.tar.xz`

2. **容器环境**
   - 测试镜像：`ghcr.io/hank9999/kiro-rs:v2026.1.5-amd64`
   - 使用 musl libc（Alpine-based）
   - glibc 和 musl 二进制不兼容

3. **错误信息**
   ```
   exec /agentfs: no such file or directory
   ```
   这是典型的动态链接器错误，表示容器 libc 版本不匹配。

4. **潜在解决方案（不推荐）**

   **方案 A：使用 glibc 镜像**
   ```bash
   docker run --rm -v $(pwd)/agentfs:/agentfs \
     debian:bullseye-slim /agentfs --version
   ```
   - ✅ 理论可行
   - ❌ 增加镜像体积和复杂度
   - ❌ 与现有 kiro-rs 镜像不兼容

   **方案 B：容器内编译**
   ```bash
   docker run --rm -v $(pwd):/workspace -w /workspace \
     rust:1.75 sh -c "cargo install agentfs-cli"
   ```
   - ✅ 可以运行
   - ❌ 编译时间长（Rust 编译）
   - ❌ 每个镜像都需要编译

   **方案 C：等待官方 musl 版本**
   - 向 AgentFS 项目提交 PR
   - 或请求官方提供 musl 静态编译版本

### ✅ 方案 A 是最佳选择（已验证可行）

**方案 A：宿主机运行 `agentfs run` + Docker 容器**

```
宿主机: agentfs run --session {id} docker run ...
  ↓
OverlayFS: HostFS (base) + AgentFS (delta)
  ↓
容器: 标准文件系统，所有操作自动追踪
```

**测试验证：**
- ✅ 文件操作完全追踪
- ✅ Session 持久化工作
- ✅ macOS 兼容（使用 sandbox-exec）
- ✅ 无需容器内安装 agentfs
- ✅ 简单易用

**执行示例：**
```bash
agentfs run --session test-session docker run \
  -v $(pwd)/workspace:/workspace \
  -w /workspace \
  alpine sh -c "echo 'test' > /workspace/file.txt"

# 查看变更
agentfs diff ~/.agentfs/run/test-session/delta.db
```

**输出：**
```
M d /workspace
A f /workspace/file.txt
```

## 最终建议

### 推荐方案：方案 A（宿主机运行 agentfs run）

**理由：**
1. **已验证可行**：完整测试通过，无遗留问题
2. **跨平台**：支持 macOS（sandbox-exec + NFS）和 Linux（FUSE + namespaces）
3. **零依赖**：容器镜像无需任何修改
4. **简单可靠**：实现复杂度低，易维护

### VibeRepo 集成实现

**1. 数据库迁移**
```sql
ALTER TABLE workspaces ADD COLUMN agentfs_session_id TEXT UNIQUE;
ALTER TABLE workspaces ADD COLUMN agentfs_delta_path TEXT;
```

**2. 创建 Workspace**
```rust
pub async fn create_workspace(repository_id: i32) -> Result<Workspace> {
    // 生成 session ID
    let session_id = format!("workspace-{}", Uuid::new_v4());

    // 初始化 agentfs session
    let cmd = Command::new("agentfs")
        .args(["run", "--session", &session_id, "true"])
        .current_dir(&work_dir)
        .status()?;

    // 保存到数据库
    let workspace = workspaces::ActiveModel {
        repository_id: Set(repository_id),
        agentfs_session_id: Set(Some(session_id.clone())),
        agentfs_delta_path: Set(Some(format!(
            "{}/.agentfs/run/{}",
            std::env::var("HOME")?,
            session_id
        ))),
        workspace_status: Set("Initializing".to_string()),
        ..Default::default()
    };

    // ... 插入数据库
}
```

**3. 执行 Task**
```rust
pub async fn execute_task(workspace: &Workspace, task: &Task) -> Result<()> {
    let session_id = workspace.agentfs_session_id.as_ref().unwrap();
    let work_dir = &workspace.work_dir;

    // 构建 docker 命令
    let docker_cmd = format!(
        "docker run --rm -v {}:/workspace -w /workspace {} {}",
        work_dir,
        workspace.image,
        task.command
    );

    // 在 agentfs 环境中执行
    let status = Command::new("agentfs")
        .args(["run", "--session", session_id, "sh", "-c", &docker_cmd])
        .status()?;

    if !status.success() {
        return Err(VibeRepoError::TaskExecutionFailed(
            task.id,
            status.to_string()
        ));
    }

    Ok(())
}
```

**4. 查询文件变更**
```rust
pub fn get_file_changes(workspace: &Workspace) -> Result<Vec<FileChange>> {
    let delta_db = &workspace.agentfs_delta_path.as_ref().unwrap();
    let conn = SqliteConnection::establish(&format!("{}/delta.db", delta_db))?;

    // 查询 fs_dentry 表
    let changes: Vec<(String, String, i32)> = dentry::Entity::find()
        .select_only()
        .columns([
            (dentry::Column::Name, String),
            (dentry::Column::ParentIno, i32),
            (dentry::Column::Ino, i32),
        ])
        .all(&conn)
        .await?;

    Ok(changes.into_iter().map(|(name, parent, ino)| {
        FileChange { name, parent_inode: parent, inode: ino }
    }).collect())
}
```

### ToolCalls 追踪实现

**推荐：宿主机手动记录**
```rust
// 在 Task 执行前后记录
pub async fn execute_with_tool_tracking(
    workspace: &Workspace,
    task: &Task
) -> Result<TaskResult> {
    let session_id = &workspace.agentfs_session_id;

    // 记录工具调用开始
    let start_time = Utc::now();
    let tool_record = agentfs_tools_record(
        session_id,
        "docker_run",
        &task.command,
    ).await?;

    // 执行命令
    let result = execute_task_command(task).await;

    // 记录结果
    let duration_ms = (Utc::now() - start_time).num_milliseconds();
    agentfs_tools_update_result(
        session_id,
        tool_record.id,
        &result,
        duration_ms,
    ).await?;

    Ok(result)
}
```

## 下一步行动

### 立即执行

1. **创建数据库迁移**
   - 添加 `agentfs_session_id` 字段
   - 添加 `agentfs_delta_path` 字段

2. **实现 WorkspaceService 集成**
   - 创建 workspace 时初始化 agentfs session
   - 删除 workspace 时清理 agentfs session

3. **实现 Task 执行集成**
   - 使用 `agentfs run` 包装 docker 命令
   - 添加错误处理和日志

4. **实现文件变更查询 API**
   - 查询 agentfs delta 数据库
   - 返回变更列表

### 后续优化

5. **ToolCalls 追踪**
   - 实现手动记录机制
   - 考虑命令包装方案

6. **KV Store 集成**
   - 支持 Agent 运行时状态持久化
   - 提供 API 读写 kv_store

7. **测试**
   - 单元测试
   - 集成测试
   - 性能测试

## 文档清单

- ✅ `docs/plans/2025-01-18-agentfs-docker-integration-test-plan.md`
- ✅ `docs/plans/2025-01-18-agentfs-docker-integration-test-results.md`
- ✅ `docs/plans/2025-01-18-agentfs-architecture-comparison.md`
- ✅ `docs/plans/2025-01-18-agentfs-research-summary.md` (本文档)

## 结论

经过深入测试和分析，**方案 A（宿主机运行 agentfs run）是 VibeRepo 与 AgentFS 集成的最佳方案**。

**核心优势：**
1. 已完整测试验证
2. 实现简单可靠
3. 跨平台兼容
4. 无容器依赖

**关键实现点：**
- Workspace 表添加 agentfs session 字段
- Task 执行时使用 `agentfs run --session {id} docker run ...`
- 直接查询 agentfs delta 数据库获取文件变更
- ToolCalls 通过宿主机手动记录

**准备就绪，可以开始编码实现。**
