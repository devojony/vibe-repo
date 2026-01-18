# AgentFS Docker 集成可行性测试计划

## 目标

验证 AgentFS 与 Docker 容器集成的可行性，特别是：
1. AgentFS 持久化挂载点是否可以映射到 Docker 容器
2. 容器内的文件操作是否能被 AgentFS 追踪
3. ToolCalls 是否能被记录（如果 `agentfs run` 支持的话）

## 测试环境

- **操作系统**: macOS (Darwin)
- **AgentFS 版本**: 0.5.3
- **容器镜像**: Alpine Linux

## 测试步骤

### 1. 安装 AgentFS CLI

```bash
# macOS: 使用 Homebrew 安装
brew install agentfs/tap/agentfs

# 或者从源码构建（如果 Homebrew 不可用）
cargo install agentfs-cli

# 验证安装
agentfs --version
```

### 2. 创建测试 Workspace

```bash
# 创建测试目录
mkdir -p /tmp/agentfs-test/workspace
cd /tmp/agentfs-test

# 初始化一个测试 AgentFS 实例
agentfs init test-workspace
# 这会创建 ~/.agentfs/test-workspace.db

# 创建一个 base 目录（模拟 Git 仓库）
mkdir -p base-repo
echo "Hello, World!" > base-repo/README.md
echo "fn main() { println!(\"Hello\"); }" > base-repo/main.rs
```

### 3. 创建持久化挂载点

**方案 A: 直接挂载 AgentFS（纯 AgentFS）**

```bash
# 创建挂载点目录
mkdir -p /tmp/agentfs-test/mountpoint

# 挂载 AgentFS（后台运行）
agentfs mount ~/.agentfs/test-workspace.db /tmp/agentfs-test/mountpoint --foreground &
MOUNT_PID=$!

# 等待挂载完成
sleep 2

# 验证挂载
ls -la /tmp/agentfs-test/mountpoint
```

**方案 B: 使用 OverlayFS（HostFS + AgentFS）**

```bash
# 创建 delta 层（AgentFS）
mkdir -p /tmp/agentfs-test/delta

# 注意：需要使用 agentfs CLI 创建 overlay，
# 或者手动使用 Linux 的 mount -t overlay 命令
# 在 macOS 上可能需要使用 NFS 方式
```

**注意**: macOS 上 `agentfs mount` 使用 NFS，不是 FUSE。需要验证 NFS 挂载点是否可以映射到 Docker 容器。

### 4. 启动测试容器

```bash
# 使用 Alpine 容器，映射挂载点
docker run -it --rm \
  -v /tmp/agentfs-test/mountpoint:/workspace \
  -v /tmp/agentfs-test/mountpoint/.agentfs:/workspace/.agentfs \
  alpine sh

# 在容器内，测试文件操作
echo "Container writes this" > /workspace/test.txt
echo "Another file" > /workspace/another.txt
ls -la /workspace/
cat /workspace/test.txt
```

### 5. 在容器内执行 Claude Code

```bash
# 在容器内（需要先安装必要工具）
apk add --no-cache curl bash

# 使用 Claude Code 执行一些任务
# 这里需要用户提供具体的 Claude Code 命令示例
# 例如：
# - 读取文件
# - 修改文件
# - 创建新文件
# - 执行 shell 命令（git, npm 等）
```

### 6. 宿主机验证 AgentFS 追踪

**查看文件系统状态：**

```bash
# 查看挂载的文件系统
agentfs fs ls --database ~/.agentfs/test-workspace.db /

# 查看文件内容
agentfs fs cat --database ~/.agentfs/test-workspace.db /test.txt

# 查看文件元数据
agentfs fs stat --database ~/.agentfs/test-workspace.db /test.txt
```

**查看 ToolCalls（如果支持）：**

```bash
# 查看 tool_calls 表
sqlite3 ~/.agentfs/test-workspace.db "SELECT * FROM tool_calls;"

# 或者使用 agentfs CLI（如果有此命令）
agentfs tools list --database ~/.agentfs/test-workspace.db
```

**查看 Key-Value Store：**

```bash
# 查看 kv_store 表
sqlite3 ~/.agentfs/test-workspace.db "SELECT * FROM kv_store;"
```

### 7. 清理

```bash
# 停止容器（Ctrl+C）

# 卸载挂载点
kill $MOUNT_PID

# 或者在 macOS 上
umount /tmp/agentfs-test/mountpoint

# 清理测试目录
rm -rf /tmp/agentfs-test
rm ~/.agentfs/test-workspace.db
```

## 预期结果

### 成功标准

1. ✅ AgentFS 成功安装并运行
2. ✅ 挂载点成功创建并可以访问
3. ✅ Docker 容器可以正常启动并映射挂载点
4. ✅ 容器内可以读写文件
5. ✅ 宿主机可以通过 `agentfs fs ls/cat` 查看到容器内的文件操作
6. ⚠️  ToolCalls 追踪（可能不支持，需要验证）

### 可能的问题和解决方案

| 问题 | 可能原因 | 解决方案 |
|------|---------|---------|
| `agentfs mount` 在 macOS 上无法映射到容器 | NFS 权限或配置问题 | 使用 Docker 的 NFS 卷映射，或改用 Linux 主机 |
| 容器内无法写入文件 | 挂载权限问题 | 检查挂载选项，添加读写权限 |
| 无法追踪文件操作 | AgentFS 不支持或配置错误 | 检查 AgentFS 版本和配置 |
| ToolCalls 没有记录 | `agentfs mount` 不支持 ToolCalls 追踪 | 需要使用 `agentfs run` 或额外的监控机制 |

## 测试脚本

创建一个自动化测试脚本 `/tmp/agentfs-test/test.sh`：

```bash
#!/bin/bash
set -e

echo "=== AgentFS Docker 集成测试 ==="

# 1. 安装检查
if ! command -v agentfs &> /dev/null; then
    echo "错误: agentfs 未安装"
    echo "运行: brew install agentfs/tap/agentfs"
    exit 1
fi

echo "AgentFS 版本: $(agentfs --version)"

# 2. 创建测试目录
TEST_DIR="/tmp/agentfs-test"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR/workspace"
mkdir -p "$TEST_DIR/base-repo"

cd "$TEST_DIR"

# 3. 初始化 AgentFS
agentfs init test-workspace
echo "✅ AgentFS 初始化完成"

# 4. 创建测试文件
echo "Base file" > base-repo/README.md

# 5. 挂载
mkdir -p mountpoint
agentfs mount ~/.agentfs/test-workspace.db mountpoint --foreground &
MOUNT_PID=$!
echo "挂载 PID: $MOUNT_PID"

sleep 3

# 6. 验证挂载
if [ -d mountpoint ]; then
    echo "✅ 挂载点创建成功"
else
    echo "❌ 挂载点创建失败"
    exit 1
fi

# 7. 启动容器
echo "=== 启动容器测试 ==="
docker run --rm \
  -v "$PWD/mountpoint:/workspace" \
  alpine sh -c "
    echo 'Container PID:' \$\$
    echo '=== 容器内文件操作 ==='
    echo 'Test from container' > /workspace/container-file.txt
    echo '✅ 容器内写入文件'
    ls -la /workspace/
"

# 8. 验证追踪
echo "=== 宿主机验证 ==="
agentfs fs ls --database ~/.agentfs/test-workspace.db /

echo "=== 清理 ==="
kill $MOUNT_PID 2>/dev/null || true
rm -rf "$TEST_DIR"

echo "✅ 测试完成"
```

## 注意事项

1. **macOS 特性**: macOS 使用 NFS 而不是 FUSE，可能与 Linux 有不同的行为
2. **Docker Desktop for Mac**: 文件系统映射可能需要特殊配置
3. **ToolCalls 追踪**: `agentfs mount` 可能不支持 ToolCalls 追踪，只有 `agentfs run` 支持
4. **OverlayFS**: macOS 上的 OverlayFS 支持可能有限，可能需要使用 Linux 主机测试

## 下一步

测试完成后，根据结果确定：
- 如果可行：设计完整的集成方案
- 如果有问题：调整方案或寻找替代方案
- ToolCalls 追踪需要额外机制：设计命令包装层或 eBPF 监控方案
