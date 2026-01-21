# AgentFS 容器集成测试设计

## 文档信息

- **创建日期**: 2026-01-19
- **状态**: 设计阶段
- **目标**: 验证 agentfs init + 容器内 run 的集成方案

## 背景

### 现有研究结论

根据 `docs/plans/2025-01-18-agentfs-research-summary.md`，已验证的方案是：
- **方案 A（宿主机运行）**: `agentfs run --session <ID> docker run ...`
- 优势：已验证、跨平台、无容器依赖

### 新方案动机

探索另一种架构模式：
1. **宿主机**: 使用 `agentfs init` 在 workspace 初始化文件系统
2. **容器内**: 使用 `agentfs run --session` 执行 agent 命令
3. **数据共享**: 通过 Docker 挂载共享 session 数据

### 核心问题

`agentfs init` 和 `agentfs run --session` 使用不同的存储结构：
- `init`: 创建 `.agentfs/<ID>.db`
- `run --session`: 期望 `~/.agentfs/run/<SESSION-ID>/delta.db`

**解决方案**: 使用符号链接桥接两种模式

## 测试目标

验证以下工作流程的可行性：

1. ✅ 宿主机使用 `agentfs init` 初始化文件系统
2. ✅ 通过符号链接兼容 `run --session` 的数据结构
3. ✅ Docker 挂载 session 数据到容器
4. ✅ 容器内安装并运行 agentfs
5. ✅ 容器内使用 `agentfs run --session` 执行命令
6. ✅ 文件变更正确追踪到宿主机的数据库

## 测试环境

### 基础配置

| 项目 | 配置 |
|------|------|
| 测试目录 | `/tmp/agentfs-test/` |
| 容器镜像 | `docker.1ms.run/ubuntu:latest` |
| Session ID | `task-001` |
| Base 目录 | `/tmp/agentfs-test/task-001` |

### 目录结构

```
/tmp/agentfs-test/
└── task-001/                    # TASK 工作目录
    ├── .agentfs/                # agentfs 数据目录
    │   ├── task-001.db          # 实际数据库文件
    │   ├── task-001.db-wal      # WAL 文件
    │   ├── delta.db -> task-001.db       # 符号链接
    │   └── delta.db-wal -> task-001.db-wal
    └── (workspace files)        # 工作文件
```

### Docker 挂载映射

```
宿主机                                    容器内
/tmp/agentfs-test/task-001/.agentfs  →  /root/.agentfs/run/task-001
/tmp/agentfs-test/task-001           →  /workspace
```

## 测试步骤

### 步骤 1: 宿主机准备

```bash
# 1.1 创建 TASK 目录
mkdir -p /tmp/agentfs-test/task-001
cd /tmp/agentfs-test/task-001

# 1.2 初始化 agentfs
agentfs init --base /tmp/agentfs-test/task-001 task-001

# 1.3 创建符号链接
cd .agentfs
ln -s task-001.db delta.db
ln -s task-001.db-wal delta.db-wal

# 1.4 验证结构
ls -la
```

**预期输出:**
```
lrwxr-xr-x  delta.db -> task-001.db
lrwxr-xr-x  delta.db-wal -> task-001.db-wal
-rw-r--r--  task-001.db
-rw-r--r--  task-001.db-wal
```

### 步骤 2: 启动容器

```bash
docker run --rm -it \
  -v /tmp/agentfs-test/task-001/.agentfs:/root/.agentfs/run/task-001 \
  -v /tmp/agentfs-test/task-001:/workspace \
  -w /workspace \
  docker.1ms.run/ubuntu:latest \
  bash
```

**挂载说明:**
- 第一个 `-v`: 挂载 agentfs 数据目录到容器的 run 目录
- 第二个 `-v`: 挂载工作目录
- `-w`: 设置工作目录为 `/workspace`

### 步骤 3: 容器内操作

```bash
# 3.1 安装 agentfs
curl -fsSL https://agentfs.ai/install | bash
export PATH="$HOME/.local/bin:$PATH"

# 3.2 验证安装
agentfs --version

# 3.3 检查符号链接（如果宿主机符号链接失效）
ls -la /root/.agentfs/run/task-001/
# 如果 delta.db 不存在，创建符号链接：
cd /root/.agentfs/run/task-001
ln -s task-001.db delta.db
ln -s task-001.db-wal delta.db-wal

# 3.4 使用 session 执行测试命令
agentfs run --session task-001 bash -c "
  echo 'Hello from container' > /workspace/test.txt
  mkdir -p /workspace/subdir
  echo 'Nested file' > /workspace/subdir/nested.txt
  ls -la /workspace
"
```

**预期行为:**
- agentfs 成功安装（ubuntu 是 glibc 环境）
- 命令在隔离环境中执行
- 显示 AgentFS 欢迎信息
- 文件操作被追踪

### 步骤 4: 宿主机验证

退出容器后，在宿主机执行：

```bash
# 4.1 查看文件变更
agentfs diff /tmp/agentfs-test/task-001/.agentfs/task-001.db

# 4.2 检查 base 目录（应该保持干净）
ls -la /tmp/agentfs-test/task-001/

# 4.3 检查数据库文件大小变化
ls -lh /tmp/agentfs-test/task-001/.agentfs/
```

**预期结果:**
- `agentfs diff` 显示创建的文件和目录：
  ```
  A f /workspace/test.txt
  A d /workspace/subdir
  A f /workspace/subdir/nested.txt
  ```
- base 目录保持干净（文件未实际写入）
- `task-001.db-wal` 文件大小增加

### 步骤 5: 清理

```bash
# 5.1 清理测试目录
rm -rf /tmp/agentfs-test

# 5.2 清理可能的符号链接
rm -f ~/.agentfs/run/task-001-test
```

## 成功标准

测试成功需要满足以下所有条件：

| # | 标准 | 验证方法 |
|---|------|----------|
| 1 | 容器内 agentfs 可运行 | `agentfs --version` 成功 |
| 2 | 符号链接有效 | `delta.db` 指向 `task-001.db` |
| 3 | Session 共享成功 | 容器内可使用宿主机创建的 session |
| 4 | 文件变更追踪 | `agentfs diff` 显示所有操作 |
| 5 | Copy-on-Write 隔离 | base 目录保持不变 |
| 6 | 数据持久化 | 容器退出后数据保留在宿主机 |

## 潜在问题和解决方案

### 问题 1: 容器内 agentfs 安装失败

**原因:**
- 网络问题
- 镜像不兼容（非 glibc 环境）

**解决方案 A: 挂载宿主机二进制**
```bash
docker run --rm -it \
  -v $(which agentfs):/usr/local/bin/agentfs:ro \
  -v /tmp/agentfs-test/task-001/.agentfs:/root/.agentfs/run/task-001 \
  ...
```

**解决方案 B: 使用 glibc 基础镜像**
```bash
# 替换为 debian 或其他 glibc 镜像
docker.1ms.run/debian:bullseye-slim
```

### 问题 2: 权限问题

**原因:**
- 容器内用户 UID/GID 与宿主机不匹配
- 数据库文件权限不足

**解决方案:**
```bash
# 方案 A: 使用宿主机用户
docker run --user $(id -u):$(id -g) ...

# 方案 B: 调整文件权限
chmod -R 777 /tmp/agentfs-test/task-001/.agentfs
```

### 问题 3: 符号链接在 Docker 挂载中失效

**原因:**
- 某些 Docker 配置不支持符号链接
- 文件系统类型限制

**解决方案: 容器内创建符号链接**

创建启动脚本 `/tmp/agentfs-test/entrypoint.sh`:

```bash
#!/bin/bash
SESSION_ID="task-001"
SESSION_DIR="/root/.agentfs/run/$SESSION_ID"

# 检查并创建符号链接
if [ ! -L "$SESSION_DIR/delta.db" ]; then
  cd "$SESSION_DIR"
  ln -s ${SESSION_ID}.db delta.db
  ln -s ${SESSION_ID}.db-wal delta.db-wal
  echo "Created symlinks in container"
fi

# 执行传入的命令
exec "$@"
```

使用方式:
```bash
chmod +x /tmp/agentfs-test/entrypoint.sh

docker run --rm -it \
  -v /tmp/agentfs-test/entrypoint.sh:/entrypoint.sh:ro \
  -v /tmp/agentfs-test/task-001/.agentfs:/root/.agentfs/run/task-001 \
  -v /tmp/agentfs-test/task-001:/workspace \
  -w /workspace \
  --entrypoint /entrypoint.sh \
  docker.1ms.run/ubuntu:latest \
  bash
```

### 问题 4: 数据库锁定冲突

**原因:**
- 宿主机和容器同时访问数据库
- SQLite WAL 模式锁定

**解决方案:**
- 确保同一时间只有一个进程访问
- 容器运行时不在宿主机操作数据库

## 与方案 A 的对比

| 维度 | 方案 A（宿主机 run） | 本方案（容器内 run） |
|------|---------------------|---------------------|
| **复杂度** | 低 | 中（需要符号链接） |
| **容器依赖** | 无 | 需要安装 agentfs |
| **跨平台** | 优秀 | 依赖容器镜像 |
| **数据隔离** | 宿主机层面 | 容器层面 |
| **灵活性** | 低 | 高（容器内完全控制） |
| **验证状态** | ✅ 已验证 | 🧪 待测试 |

## 后续步骤

### 如果测试成功

1. **编写自动化测试脚本**
   - 创建 `tests/agentfs-container-integration.sh`
   - 包含所有测试步骤和验证

2. **更新实施建议**
   - 在研究总结中添加方案对比
   - 提供两种方案的选择指南

3. **VibeRepo 集成设计**
   - 设计 WorkspaceService 如何选择方案
   - 考虑配置项支持两种模式

### 如果测试失败

1. **记录失败原因**
   - 详细的错误信息
   - 环境差异分析

2. **评估替代方案**
   - 是否需要修改 agentfs
   - 是否坚持使用方案 A

3. **更新文档**
   - 标记本方案为不可行
   - 说明技术限制

## 测试脚本

完整的自动化测试脚本将在测试执行后创建，包含：
- 环境检查
- 自动化执行所有步骤
- 结果验证
- 清理操作

## 参考文档

- [AgentFS 研究总结](./2025-01-18-agentfs-research-summary.md)
- [AgentFS 架构对比](./2025-01-18-agentfs-architecture-comparison.md)
- [AgentFS 容器 Session 测试报告](./2025-01-18-agentfs-container-session-test-report.md)
- [AgentFS 介绍](../agentfs-introduction.md)

## 结论

本测试旨在验证一种新的 AgentFS 集成模式，通过符号链接桥接 `init` 和 `run --session` 两种存储模式，实现宿主机初始化、容器内执行的工作流程。

**关键创新:**
- 符号链接方案（已在宿主机验证可行）
- 灵活的符号链接创建位置（宿主机或容器内）
- 完整的错误处理和备选方案

**下一步:** 执行测试并根据结果更新本文档。
