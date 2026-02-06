# AgentFS Docker 集成测试报告

## 测试日期

2025-01-18

## 测试环境

- **操作系统**: macOS (Darwin)
- **AgentFS 版本**: 0.5.3
- **安装方式**: 从 GitHub Releases 下载预编译二进制文件

## 测试步骤

### 1. 安装 AgentFS

```bash
# 下载并安装
curl -L -o agentfs.tar.xz \
  https://github.com/tursodatabase/agentfs/releases/download/v0.5.3/agentfs-x86_64-apple-darwin.tar.xz
tar xf agentfs.tar.xz
sudo mv agentfs-x86_64-apple-darwin/agentfs /usr/local/bin/agentfs

# 验证安装
agentfs --version
# 输出: agentfs v0.5.3
```

### 2. 初始化 AgentFS 实例

```bash
# 创建测试目录
mkdir -p /tmp/agentfs-test/base-repo
cd /tmp/agentfs-test

# 创建基础文件
echo "Base file content" > base-repo/README.md
echo "fn main() { println!(\"Hello\"); }" > base-repo/main.rs

# 初始化 AgentFS
agentfs init test-workspace
# 输出: Created agent filesystem: .agentfs/test-workspace.db
#       Agent ID: test-workspace
```

### 3. 使用 agentfs run 进行沙盒执行

```bash
# 启动第一个 session，创建文件
agentfs run --session test-session sh -c "echo 'Created by agentfs' > base-repo/test.txt && cat base-repo/test.txt"

# 输出:
# Welcome to AgentFS!
#
# The following directories are writable:
#
#   - /private/tmp/agentfs-test (copy-on-write)
#   - /tmp
#   - /Users/devo/{.bun, .cache, .claude, .claude.json, .config, .gemini, .local, .npm}
#
# 🔒 Everything else is read-only.
#
# Created by agentfs
#
# Delta layer saved to: /Users/devo/.agentfs/run/test-session/delta.db
```

### 4. 验证 session 持久化

```bash
# 再次使用相同的 session ID，文件仍然存在
agentfs run --session test-session sh -c "ls -la base-repo/"

# 输出显示:
# total 3
# drwxr-xr-x  1 devo  staff    0 Jan 18 19:10 .
# drwxr-xr-x  1 devo  staff    0 Jan 18 19:10 ..
# -rw-r--r--  1 devo  staff   18 Jan 18 19:09 README.md
# -rw-r--r--  1 devo  staff   33 Jan 18 19:09 main.rs
# -rw-r--r--  1 devo  staff   19 Jan 18 19:10 test.txt  ← 持久化的文件
```

### 5. 在 agentfs 环境中运行 Docker 容器

```bash
# 直接在 agentfs 环境中执行 docker 命令
agentfs run --session test-session sh -c "which docker && docker ps --no-trunc | head -3"

# 输出:
# /usr/local/bin/docker
# CONTAINER ID                                                       IMAGE                                      COMMAND                                                                             CREATED        STATUS       PORTS     NAMES
# ad760f85307cb092c782b6ef107f033c416448f69af5dbc35b9a07144d2b22f9   ghcr.io/hank9999/kiro-rs:v2026.1.5-amd64   "./kiro-rs -c /app/config/config.json --credentials /app/config/credentials.json"   47 hours ago   Up 4 hours             kiro-rs-service
```

### 6. 在 Docker 容器中创建文件（通过 agentfs）

```bash
# 在 agentfs 环境中启动 Docker 容器并操作文件
agentfs run --session test-session sh -c "
  docker run --rm \
    -v \$(pwd)/base-repo:/workspace \
    -w /workspace \
    ghcr.io/hank9999/kiro-rs:v2026.1.5-amd64 \
    sh -c 'echo \"Created in Docker container\" > /workspace/docker-test.txt && cat /workspace/docker-test.txt && ls -la /workspace/'
"

# 输出:
# Starting Docker container...
# Created in Docker container
# total 4
# drwxr-xr-x    1 root     root             0 Jan 18 11:10 .
# drwxr-xr-x    1 root     root            36 Jan 18 11:13 ..
# -rw-r--r--    1 root     root            18 Jan 18 11:09 README.md
# -rw-r--r--    1 root     root            28 Jan 18 11:13 docker-test.txt  ← Docker 容器创建的
# -rw-r--r--    1 root     root            33 Jan 18 11:09 main.rs
# -rw-r--r--    1 root     root            19 Jan 18 11:10 test.txt
```

### 7. 查看所有变化（通过 AgentFS diff）

```bash
agentfs diff ~/.agentfs/run/test-session/delta.db

# 输出:
# Using agent: /Users/devo/run/test-session/delta.db
# Base: /private/tmp/agentfs-test
# M d /base-repo
# A f /base-repo/docker-test.txt
# A f /base-repo/test.txt
```

### 8. 检查数据库内容

```bash
# 查看文件数据
sqlite3 ~/.agentfs/run/test-session/delta.db "SELECT * FROM fs_data;"

# 输出:
# 3|0|Created by agentfs
# 4|0|Created in Docker container

# 查看文件元数据
sqlite3 ~/.agentfs/run/test-session/delta.db "SELECT ino, mode, size, mtime FROM fs_inode;"

# 输出:
# 1|16877|0|1768734619
# 2|16877|0|1768734632
# 3|33188|19|1768734632
# 4|33188|28|1768734821
```

## 测试结果

### ✅ 成功项

| 项目 | 结果 | 说明 |
|------|------|------|
| AgentFS CLI 安装 | ✅ 成功 | macOS x86_64 预编译二进制文件可用 |
| AgentFS 初始化 | ✅ 成功 | 成功创建 SQLite 数据库 |
| agentfs run 执行 | ✅ 成功 | macOS 上使用 sandbox-exec 实现 |
| Session 持久化 | ✅ 成功 | 使用 `--session` 参数可以在多次运行间共享状态 |
| Docker 命令执行 | ✅ 成功 | 在 agentfs 环境中可以直接运行 docker 命令 |
| 容器文件操作追踪 | ✅ 成功 | Docker 容器内的文件操作被 AgentFS 追踪 |
| 文件内容记录 | ✅ 成功 | 所有文件内容都记录在 `fs_data` 表中 |
| 文件元数据记录 | ✅ 成功 | inode 信息记录在 `fs_inode` 表中 |

### ⚠️ 部分成功/限制

| 项目 | 结果 | 说明 |
|------|------|------|
| agentfs mount | ❌ 不支持 | macOS 上 FUSE 不可用（仅 Linux） |
| NFS 服务器 | ✅ 可用 | `agentfs serve nfs` 可启动，但未测试容器挂载 |
| ToolCalls 追踪 | ❌ 未记录 | `tool_calls` 表为空，需要手动记录 |

## 关键发现

### 1. **核心发现：在 agentfs 环境中运行 Docker**

VibeRepo 可以直接在 `agentfs run` 的沙盒环境中启动和管理 Docker 容器：

```bash
# VibeRepo 的执行流程
agentfs run --session {workspace_id} docker run [docker-options] [image] [command]
```

**优势：**
- 所有文件操作自动记录到 AgentFS delta 数据库
- 不需要复杂的 NFS 挂载配置
- Session 持久化保证状态在多次执行间共享
- 完整的 copy-on-write 语义

**限制：**
- 只能追踪文件系统操作（读取、写入、创建、删除）
- 无法自动追踪 `tool_calls`（需要额外机制）

### 2. **AgentFS 数据结构**

Delta 数据库包含以下表：
- `fs_inode`: 文件/目录元数据（权限、大小、时间戳）
- `fs_dentry`: 目录项（名称与 inode 的映射）
- `fs_data`: 文件内容（分块存储）
- `fs_whiteout`: 记录删除的文件（OverlayFS whiteout）
- `fs_origin`: 记录 copy-on-write 来源
- `tool_calls`: 工具调用记录（当前为空）
- `kv_store`: 键值存储（当前为空）

### 3. **Session 管理**

- Session 数据存储在 `~/.agentfs/run/<SESSION_ID>/`
- 包含 `delta.db` 和挂载点信息
- 可以通过 `agentfs ps` 查看活动 session
- 可以通过 `agentfs prune` 清理未使用的资源

## 对 VibeRepo 集成方案的影响

### 推荐方案：使用 `agentfs run` 作为容器执行环境

```
Workspace (VibeRepo)
    ↓
agentfs run --session {workspace_id} [COMMAND]
    ↓
OverlayFS (HostFS + AgentFS Delta)
    ↓
Docker 容器（通过 volume 映射）
    ↓
所有文件修改记录到 AgentFS
```

### 数据库设计建议

**workspaces 表添加字段：**
```sql
ALTER TABLE workspaces ADD COLUMN agentfs_session_id TEXT UNIQUE;
ALTER TABLE workspaces ADD COLUMN agentfs_delta_path TEXT;
```

### 执行流程设计

1. **创建 Workspace**
   ```rust
   // 生成唯一的 session ID
   let session_id = format!("workspace-{}", uuid);
   let delta_path = format!("{}/.agentfs/run/{}", home, session_id);

   // 执行 agentfs run 初始化
   Command::new("agentfs")
       .args(["run", "--session", &session_id, "true"])
       .status()?;

   // 保存到数据库
   workspace.agentfs_session_id = Some(session_id);
   workspace.agentfs_delta_path = Some(delta_path);
   ```

2. **执行 Task**
   ```rust
   // 在 agentfs 环境中运行 Docker 容器
   let cmd = format!(
       "docker run --rm -v {}:/workspace -w /workspace {} {}",
       workspace.work_dir.unwrap(),
       workspace.image,
       task.command
   );

   Command::new("agentfs")
       .args(["run", "--session", &session_id, "sh", "-c", &cmd])
       .status()?;
   ```

3. **查询文件变更**
   ```rust
   // 查询 delta 数据库
   let delta_db = format!("{}/delta.db", workspace.agentfs_delta_path);
   let changes = query_agentfs_changes(&delta_db)?;
   ```

### ToolCalls 追踪方案

由于 `agentfs run` 不自动记录 `tool_calls`，有以下选项：

**方案 A：命令包装层（推荐）**
- 包装所有 git、npm、cargo 等命令
- 在包装脚本中记录到 AgentFS kv_store
- 示例：
  ```bash
  # /usr/local/bin/git-wrapper
  echo "$(date) git $*" >> /tmp/tool_calls.log
  /usr/bin/git "$@"
  ```

**方案 B：使用 `--experimental-sandbox --strace`（Linux）**
- 在 Linux 上使用 ptrace 拦截系统调用
- 解析系统调用记录工具调用
- 在 macOS 上不可用

**方案 C：手动记录（简单）**
- 在 VibeRepo 的 Task 执行层手动记录工具调用
- 使用 AgentFS SDK 的 `tools.record()` API
- 示例：
  ```rust
  agentfs.tools.record("git_clone", params, result).await?;
  ```

## 后续工作

### 必须完成

1. **测试完整的 VibeRepo 工作流**
   - 创建 Workspace
   - 执行 Task（使用 agentfs run）
   - 查询文件变更
   - 删除 Workspace（清理 agentfs session）

2. **实现 ToolCalls 追踪**
   - 选择方案（推荐方案 A 或 C）
   - 实现包装脚本或 API 调用
   - 验证记录准确性

3. **测试 KV Store 持久化**
   - 测试 Agent 持久化运行时状态
   - 验证跨 session 状态保持

### 可选完成

4. **测试 NFS 挂载到容器**
   - 如果需要容器内直接访问 AgentFS
   - 可能需要配置 Docker Desktop 的 NFS 支持

5. **性能测试**
   - 大文件操作性能
   - 并发文件操作
   - Delta 数据库大小增长

6. **错误处理**
   - AgentFS 命令失败
   - Session 冲突
   - Docker 容器失败

## 结论

✅ **AgentFS 与 Docker 集成可行**

**推荐方案：**
- 使用 `agentfs run` 作为 VibeRepo 的容器执行环境
- Session ID 与 Workspace 一对一映射
- 所有文件操作自动追踪
- ToolCalls 追踪使用手动记录方案

**下一步：**
开始实现 VibeRepo 与 AgentFS 的集成代码。
