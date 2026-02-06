# AgentFS 容器内运行测试报告

## 测试日期

2025-01-18

## 测试目标

验证将宿主机上的 agentfs session 数据库挂载到容器内，然后在容器内使用 agentfs 的可行性。

## 测试步骤

### 1. 宿主机创建 session

```bash
cd /tmp/agentfs-container-test/workspace
agentfs run --session test-session true
```

**结果：**
- ✅ Session 成功创建
- ✅ Delta 数据库位置：`~/.agentfs/run/test-session/delta.db`
- ✅ 数据库表结构完整：
  - kv_store
  - fs_config
  - fs_inode
  - fs_dentry
  - fs_data
  - fs_symlink
  - tool_calls
  - fs_whiteout
  - fs_overlay_config
  - fs_origin

### 2. 挂载 session 目录到容器

```bash
docker run --rm \
  -v ~/.agentfs/run/test-session:/root/.agentfs/run/test-session \
  -v $(pwd)/workspace:/workspace \
  ...
```

**结果：**
- ✅ 挂载成功
- ✅ 容器内可以访问：`/root/.agentfs/run/test-session/delta.db`
- ✅ 文件类型正确：`SQLite 3.x database`

### 3. 容器内尝试使用 `agentfs run`

```bash
docker run --rm ... \
  mcr.microsoft.com/devcontainers/base:ubuntu-22.04 \
  agentfs run --session test-session bash -c "..."
```

**结果：**
- ❌ 失败
- ❌ 错误：`FUSE mount did not become ready within 10s`
- ❌ **原因**：容器内 FUSE 不可用（需要特殊权限）

### 4. 容器内直接写入 delta.db（SQL）

```bash
# 容器内
sqlite3 /root/.agentfs/run/test-session/delta.db \
  "INSERT INTO fs_data (ino, chunk_index, data) VALUES (1, 0, 'Direct SQL write');"
```

**结果：**
- ✅ **成功！**
- ✅ 容器内写入数据
- ✅ 宿主机可以读取数据
- ✅ 数据持久化正常

**输出：**
```
=== Check in container ===
1|0|Direct SQL write test

=== Check on HOST ===
1|0|Direct SQL write test
```

## 测试结果总结

### ✅ 可行方案

| 方案 | 可行性 | 说明 |
|------|--------|------|
| 挂载 delta.db 到容器 | ✅ 成功 | 数据库文件可以正常挂载和访问 |
| 容器内直接 SQL 操作 | ✅ 成功 | 可以绕过 AgentFS API 直接操作数据库 |
| 数据持久化 | ✅ 成功 | 容器内的修改在宿主机上可见 |

### ❌ 不可行方案

| 方案 | 可行性 | 原因 |
|------|--------|------|
| 容器内 `agentfs run` | ❌ 失败 | FUSE 在容器内不可用 |
| AgentFS SDK 集成 | ⏸️ 未测试 | 需要 Rust SDK，增加复杂度 |

## 核心发现

### 1. **数据库挂载可行但有限制**

**可行的：**
- 宿主机的 delta.db 可以挂载到容器内
- 容器内可以直接读写 SQLite 数据库
- 数据可以在宿主机和容器间共享

**限制：**
- 丢失 OverlayFS 功能（copy-on-write）
- 需要手动维护数据库一致性
- 无法使用 `agentfs run` 的沙盒功能

### 2. **直接 SQL 操作 vs AgentFS API**

**直接 SQL 操作：**
```bash
# 容器内
sqlite3 delta.db "
  INSERT INTO fs_data (ino, chunk_index, data) VALUES (1, 0, 'data');
  INSERT INTO fs_dentry (parent_ino, name, ino) VALUES (1, 'file.txt', 1);
"
```

**优点：**
- 完全控制数据库操作
- 不依赖 FUSE 或其他特殊权限
- 可以在容器内运行

**缺点：**
- 需要理解 AgentFS 内部表结构
- 需要手动维护关系（inode, dentry, data）
- 容易出错（表之间的关系复杂）

### 3. **与方案 A（宿主机 agentfs run）的对比**

| 特性 | 方案 A（宿主机） | 方案 C（容器内 SQL） |
|------|------------------|-------------------|
| 文件追踪 | ✅ 自动 | ❌ 手动 |
| OverlayFS | ✅ 支持 | ❌ 不支持 |
| 容器依赖 | ✅ 无需 | ⚠️ 需要 SQLite |
| 复杂度 | 低 | 高 |
| ToolCalls 追踪 | 需要额外机制 | 需要额外机制 |
| 数据隔离 | ✅ 完整 | ⚠️ 数据库共享 |

## 推荐方案

### 仍然推荐：方案 A（宿主机运行 agentfs run）

**理由：**
1. ✅ 已完整测试验证
2. ✅ 实现简单可靠
3. ✅ 完整的 OverlayFS 功能
4. ✅ 自动文件追踪
5. ✅ 零容器依赖

### 方案 C（容器内直接 SQL）的适用场景

**可以考虑使用的场景：**
- 需要在容器内直接查询 AgentFS 数据库（例如，提供文件历史 API）
- 需要在容器内自定义数据库操作（例如，特殊的索引或查询）
- 可以接受手动维护数据库一致性

**不推荐作为主要方案：**
- 太复杂：需要理解 AgentFS 内部表结构
- 容易出错：表之间的关系复杂（fs_inode, fs_dentry, fs_data, fs_whiteout 等）
- 丢失功能：无法使用 OverlayFS、copy-on-write 等

## 实现建议

### VibeRepo 使用方案 C（如果选择）

**1. 数据库迁移**
```sql
ALTER TABLE workspaces ADD COLUMN agentfs_session_id TEXT UNIQUE;
ALTER TABLE workspaces ADD COLUMN agentfs_delta_path TEXT;
```

**2. 创建 Workspace（宿主机初始化 session）**
```rust
pub async fn create_workspace(repository_id: i32) -> Result<Workspace> {
    let session_id = format!("workspace-{}", Uuid::new_v4());

    // 在宿主机创建 session
    let delta_db = format!("{}/.agentfs/run/{}/delta.db",
        std::env::var("HOME")?, session_id);

    Command::new("agentfs")
        .args(["run", "--session", &session_id, "true"])
        .status()?;

    // 保存到数据库
    Workspace { agentfs_session_id: Some(session_id), ... }
}
```

**3. 容器内写入 delta.db（需要正确维护表关系）**

**方法 A：使用 AgentFS CLI（推荐）**
```bash
# 容器内（如果 FUSE 可用）
agentfs run --session {session_id} sh -c "echo 'data' > /workspace/file.txt"
```

**方法 B：直接 SQL（复杂）**
```bash
# 容器内（FUSE 不可用时）
# 注意：这需要维护多个表的关系

# 1. 创建 inode
sqlite3 delta.db "
  INSERT INTO fs_inode (mode, nlink, size, atime, mtime, ctime)
  VALUES (33188, 1, <size>, <atime>, <mtime>, <ctime>);
"

# 2. 创建 dentry
sqlite3 delta.db "
  INSERT INTO fs_dentry (parent_ino, name, ino)
  VALUES (<parent_ino>, 'file.txt', <ino>);
"

# 3. 创建 data
sqlite3 delta.db "
  INSERT INTO fs_data (ino, chunk_index, data)
  VALUES (<ino>, 0, 'file content');
"
```

**方法 C：混合方案（最灵活）**
```bash
# 宿主机：使用 agentfs run 管理 OverlayFS
agentfs run --session {session_id} docker run -v ... <image> <command>

# 容器内：直接查询 delta.db（只读）
sqlite3 delta.db "SELECT * FROM fs_dentry WHERE parent_ino = ?;"
```

## 结论

### 测试验证了：

1. ✅ **宿主机 session delta.db 可以挂载到容器内**
2. ✅ **容器内可以直接读写 delta.db（通过 SQLite）**
3. ❌ **容器内 `agentfs run` 不可行**（FUSE 限制）
4. ✅ **数据持久化正常**

### 推荐：

**方案 A（宿主机 agentfs run）仍然是最佳选择。**

**方案 C（容器内直接 SQL）仅在以下情况下考虑：**
- 需要 AgentFS 数据库的自定义查询
- 需要在容器内提供 AgentFS 数据的 API
- 可以接受额外的复杂度和维护成本

### 关键结论

**将 session.db 挂载到容器内是可行的，但：**
1. 失去了 OverlayFS 的核心价值
2. 需要手动维护数据库一致性
3. 实现复杂度高

**因此，对于 VibeRepo 项目，方案 A 仍然是更优的选择。**
