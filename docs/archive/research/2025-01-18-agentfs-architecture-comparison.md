# AgentFS Docker 集成方案对比分析

## 测试日期

2025-01-18

## 测试目标

验证不同方案下 AgentFS 与 Docker 容器的集成可行性。

## 方案对比

### 方案 A：宿主机运行 `agentfs run` + Docker 容器

**架构：**
```
宿主机
  ↓ agentfs run --session {id}
  ↓ OverlayFS (HostFS + AgentFS Delta)
  ↓ Docker run --volume /workspace
容器
```

**测试结果：**
- ✅ 文件操作自动追踪
- ✅ Session 持久化
- ✅ 在 macOS 上可用（使用 sandbox-exec）
- ✅ 无需容器内安装 agentfs
- ⚠️ ToolCalls 需要手动记录
- ✅ 已经完整测试通过

**执行示例：**
```bash
agentfs run --session test-session docker run \
  -v $(pwd)/workspace:/workspace \
  -w /workspace \
  my-image bash -c "echo 'test' > /workspace/file.txt"
```

**优势：**
1. 简单直接：宿主机管理 agentfs，容器只需标准 Docker
2. 跨平台：macOS（sandbox-exec + NFS）和 Linux（FUSE + namespaces）
3. 无容器依赖：容器镜像无需预装 agentfs
4. 完整追踪：所有文件操作记录在 agentfs delta 数据库

**劣势：**
1. 依赖宿主机 agentfs 命令
2. 工具调用需要额外机制（包装脚本或手动记录）
3. 无法在容器内直接使用 agentfs SDK

---

### 方案 B：容器内运行 `agentfs run`

**架构：**
```
宿主机
  ↓ Docker run --volume agentfs.db
容器
  ↓ agentfs run --session {id}
  ↓ OverlayFS (HostFS + AgentFS Delta)
  ↓ Agent 执行
```

**测试结果：**
- ❌ 二进制兼容性问题
- ❌ 容器使用 musl libc（Alpine），agentfs 编译为 glibc
- ❌ GitHub Releases 未提供 musl 版本
- ⚠️ 需要容器内安装 agentfs
- ⚠️ 需要正确的权限配置

**预期执行示例（如果可行）：**
```bash
docker run --rm \
  -v $(pwd)/workspace:/workspace \
  -v $(pwd)/agentfs-x86_64-linux-musl:/usr/local/bin/agentfs:ro \
  -w /workspace \
  my-image \
  agentfs run --session test-session bash -c "echo 'test' > /workspace/file.txt"
```

**优势（理论）：**
1. 完全隔离：AgentFS 完全在容器内运行
2. 可移植性：整个 agentfs session 可以打包成 Docker 镜像
3. 并发友好：多个容器可以同时运行独立的 agentfs session
4. 简化架构：不依赖宿主机 agentfs

**劣势（实际）：**
1. **二进制兼容性**：Alpine/musl 容器无法运行 glibc 编译的 agentfs
2. **镜像体积**：需要在每个镜像中包含 agentfs 二进制
3. **权限问题**：容器内 `agentfs run` 可能需要额外的权限
4. **未提供 musl 版本**：AgentFS 官方只提供 glibc 版本

---

### 方案 C：映射 AgentFS Delta 数据库到容器

**架构：**
```
宿主机
  ↓ 创建 agentfs session
  ↓ 映射 delta.db 到容器
容器
  ↓ 读写 delta.db（通过 volume）
  ↓ 使用 SQLite API 或 agentfs SDK
  ↓ Agent 执行
```

**测试结果：**
- ⏸️ 未测试（需要容器内 agentfs SDK）
- ⏸️ 可能的性能开销（SQLite 文件锁定）
- ⏸️ 并发访问问题

**预期执行示例：**
```bash
# Host: initialize session
agentfs run --session test-session true

# Host: mount delta DB to container
docker run --rm \
  -v ~/.agentfs/run/test-session/delta.db:/workspace/agentfs.db \
  -v $(pwd)/workspace:/workspace \
  -w /workspace \
  my-image-with-agentfs-sdk \
  python agent.py
```

**优势：**
1. 灵活性：容器可以直接访问 agentfs 数据库
2. 完整 API：可以使用 agentfs SDK 的所有功能
3. 状态持久：delta DB 可以在容器重启后继续使用

**劣势：**
1. **容器内需要 agentfs SDK**：需要额外依赖
2. **SQLite 并发问题**：多个进程同时访问可能锁文件
3. **复杂度高**：需要处理数据库文件锁定
4. **性能开销**：跨容器边界的数据库访问

---

### 方案 D：NFS 挂载（未充分测试）

**架构：**
```
宿主机
  ↓ agentfs serve nfs --port 11111
  ↓ NFS Server
容器
  ↓ mount -t nfs host:/ /agentfs
  ↓ OverlayFS (NFS + AgentFS Delta)
  ↓ Agent 执行
```

**测试结果：**
- ✅ `agentfs serve nfs` 可以启动
- ⏸️ Docker 容器挂载 NFS 未测试
- ⚠️ macOS NFS 可能有权限问题
- ⚠️ 需要配置 Docker Desktop NFS 支持

**预期执行示例：**
```bash
# Host: start NFS server
agentfs serve nfs test-workspace --bind 0.0.0.0 --port 11111 &

# Container: mount NFS
docker run --rm \
  --cap-add=SYS_ADMIN \
  --device /dev/fuse \
  my-image \
  sh -c "
    mount -t nfs host:11111:/ /agentfs
    cd /agentfs
    echo 'test' > file.txt
  "
```

**优势：**
1. 标准协议：NFS 是成熟的标准
2. 网络访问：可以跨主机访问
3. 完整功能：支持所有 agentfs 操作

**劣势：**
1. **配置复杂**：NFS 挂载需要特殊权限和配置
2. **性能开销**：网络文件系统有延迟
3. **权限问题**：Docker 容器挂载 NFS 可能受限
4. **未充分测试**：不确定在容器内的可行性

---

## 推荐方案

### 首选：方案 A（宿主机运行 agentfs run）

**理由：**
1. ✅ 已验证可行：完整测试通过
2. ✅ 简单可靠：不依赖容器内环境
3. ✅ 跨平台：支持 macOS 和 Linux
4. ✅ 无额外依赖：容器镜像不需要任何修改

**实现建议：**
```rust
// VibeRepo 伪代码
pub async fn execute_task(workspace: &Workspace, task: &Task) -> Result<()> {
    let session_id = &workspace.agentfs_session_id;

    // 构建 agentfs run 命令
    let docker_cmd = format!(
        "docker run --rm -v {}:/workspace -w /workspace {} {}",
        workspace.work_dir,
        workspace.image,
        task.command
    );

    // 在 agentfs 环境中执行
    let status = Command::new("agentfs")
        .args(["run", "--session", session_id, "sh", "-c", &docker_cmd])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Task execution failed"));
    }

    // 查询文件变更
    let delta_db = format!("{}/delta.db", workspace.agentfs_delta_path);
    let changes = query_agentfs_changes(&delta_db)?;

    Ok(())
}
```

**数据库设计：**
```sql
-- workspaces 表添加字段
ALTER TABLE workspaces ADD COLUMN agentfs_session_id TEXT UNIQUE;
ALTER TABLE workspaces ADD COLUMN agentfs_delta_path TEXT;

-- 使用示例
INSERT INTO workspaces (repository_id, agentfs_session_id, agentfs_delta_path)
VALUES (1, 'workspace-abc-123', '/home/user/.agentfs/run/workspace-abc-123');
```

---

## 次选方案：方案 C（映射 Delta 数据库）

**适用场景：**
- 需要在容器内直接使用 agentfs SDK
- 需要更细粒度的控制（如直接查询 kv_store）
- 可以接受额外的复杂性

**实现挑战：**
1. **创建包含 agentfs SDK 的 Docker 镜像**
   ```dockerfile
   FROM ubuntu:22.04
   RUN apt-get update && apt-get install -y sqlite3 python3-pip
   RUN pip3 install agentfs-sdk
   ```

2. **处理 SQLite 文件锁定**
   - 使用 SQLite 的 WAL 模式
   - 或在写入时加锁

3. **确保只允许一个写入进程**
   - 使用文件锁（flock）
   - 或使用消息队列协调访问

---

## 方案 B 的可行性分析

### 问题根源

1. **二进制不兼容**
   - AgentFS 官方只提供 glibc 编译版本
   - 许多 Docker 镜像（Alpine、musl-based）使用 musl libc
   - glibc 和 musl 二进制不兼容

2. **解决选项**

   **选项 A：使用 glibc 镜像**
   ```bash
   docker run --rm \
     -v $(pwd)/agentfs:/agentfs \
     debian:bullseye-slim \
     /agentfs --version
   ```

   **选项 B：从源码编译（容器内）**
   ```bash
   docker run --rm \
     -v $(pwd):/workspace \
     -w /workspace \
     rust:1.75 \
     sh -c "
       cargo install agentfs-cli
       /root/.cargo/bin/agentfs --version
     "
   ```

   **选项 C：静态编译 agentfs（需要社区支持）**
   - 向 AgentFS 项目提交 PR，添加 musl 静态编译

3. **其他挑战**
   - **FUSE 权限**：容器内需要 `--cap-add=SYS_ADMIN --device /dev/fuse`
   - **用户命名空间**：Docker 容器内的命名空间可能有限制
   - **性能开销**：嵌套虚拟化（Docker + AgentFS Sandbox）

### 结论

方案 B 在技术上是可行的，但需要：
1. 使用基于 glibc 的容器镜像
2. 配置正确的 Docker 权限
3. 额外的镜像构建步骤

**建议：** 除非有特殊需求（如完全隔离、可移植性），否则优先使用方案 A。

---

## ToolCalls 追踪解决方案

所有方案中，ToolCalls 都需要额外的机制：

### 方案 1：命令包装层

```bash
# /usr/local/bin/git-wrapper
#!/bin/bash
# Record to agentfs kv_store
timestamp=$(date +%s)
echo "{\"tool\":\"git\",\"args\":\"$*\",\"timestamp\":$timestamp}" >> /tmp/tool_calls.log

# Execute actual command
/usr/bin/git "$@"
```

### 方案 2：宿主机手动记录（推荐给方案 A）

```rust
// VibeRepo Task 执行层
async fn execute_with_tool_tracking(
    workspace: &Workspace,
    task: &Task
) -> Result<()> {
    // 1. 记录工具调用开始
    let call_id = agentfs_tools_record(
        workspace,
        task.command,
        "{}",
    ).await?;

    // 2. 执行命令
    let result = execute_task_command(&task).await;

    // 3. 记录结果
    agentfs_tools_update_result(
        workspace,
        call_id,
        &result,
        result.duration_ms,
    ).await?;

    Ok(())
}
```

### 方案 3：使用 `--experimental-sandbox --strace`（仅 Linux）

```bash
agentfs run --session test-id \
  --experimental-sandbox --strace \
  python agent.py

# 解析 strace 输出，提取工具调用
```

---

## 性能对比

| 方案 | 启动开销 | 文件操作开销 | 追踪准确性 | 实现复杂度 |
|------|----------|------------|-----------|----------|
| A（宿主机 agentfs run） | 低 | 中 | 高 | 低 |
| B（容器内 agentfs run） | 中 | 中 | 高 | 高 |
| C（映射 delta DB） | 低 | 低 | 高 | 中 |
| D（NFS 挂载） | 高 | 高 | 高 | 高 |

---

## 最终建议

### VibeRepo 项目集成 AgentFS 的最佳实践

**采用方案 A（宿主机运行 agentfs run）**，原因：

1. **已验证可行**：完整测试通过
2. **简单可靠**：实现复杂度低
3. **跨平台**：支持 macOS 和 Linux
4. **无额外依赖**：容器镜像无需修改
5. **生产就绪**：与现有 Docker 工作流无缝集成

**具体实现：**
- Workspace 表添加 `agentfs_session_id` 和 `agentfs_delta_path` 字段
- Task 执行时使用 `agentfs run --session {id} docker run ...`
- 文件变更查询直接读取 delta 数据库
- ToolCalls 使用宿主机手动记录机制

**后续优化方向：**
- 如果需要更高隔离性，可以探索方案 C
- 如果 AgentFS 提供 musl 版本，可以考虑方案 B
- 如果需要跨主机访问，可以探索方案 D

---

## 附录：测试命令快速参考

### 方案 A 测试命令
```bash
# 初始化
agentfs init test-workspace
cd /tmp/test
echo "test" > file.txt

# 执行
agentfs run --session test-session docker run --rm \
  -v $(pwd):/workspace -w /workspace \
  alpine sh -c "echo 'container' > /workspace/container.txt"

# 查看变更
agentfs diff ~/.agentfs/run/test-session/delta.db
```

### 方案 B 测试命令（如果可行）
```bash
# 创建 glibc 镜像
docker run --rm -v $(pwd)/agentfs:/agentfs \
  debian:bullseye-slim /agentfs --version

# 在容器内运行
docker run --rm -v $(pwd)/agentfs:/usr/local/bin/agentfs \
  debian:bullseye-slim \
  agentfs run --session test-session bash -c "echo 'test' > file.txt"
```

### 方案 C 测试命令
```bash
# 初始化 session
agentfs run --session test-session true

# 查询 delta 数据库
sqlite3 ~/.agentfs/run/test-session/delta.db "SELECT * FROM fs_dentry;"
```
