# AgentFS 容器集成测试结果报告

## 测试信息

- **执行日期**: 2026-01-19
- **测试方案**: 宿主机 init + 容器内 run
- **参考文档**: `docs/plans/2026-01-19-agentfs-container-integration-test-design.md`

## 测试环境

| 项目 | 配置 |
|------|------|
| 宿主机系统 | macOS (Darwin 25.2.0) |
| 容器镜像 | docker.1ms.run/ubuntu:latest |
| AgentFS 版本 | v0.5.3 |
| Session ID | task-001 |

## 测试执行情况

### ✅ 成功的步骤

#### 步骤 1: 宿主机准备
**状态**: ✅ 成功

```bash
# 创建目录和初始化
mkdir -p /tmp/agentfs-test/task-001
cd /tmp/agentfs-test/task-001
agentfs init --base /tmp/agentfs-test/task-001 task-001
```

**结果**:
- AgentFS 成功初始化
- 创建了 `.agentfs/task-001.db` 和 WAL 文件
- 符号链接创建成功：`delta.db -> task-001.db`

#### 步骤 2: 容器启动和 AgentFS 安装
**状态**: ✅ 成功

**挂载配置**:
```bash
-v /tmp/agentfs-test/task-001/.agentfs:/root/.agentfs/run/task-001
-v /tmp/agentfs-test/task-001:/workspace
```

**结果**:
- 容器成功启动
- 符号链接在容器内可见
- AgentFS v0.5.3 成功安装到容器内（`~/.cargo/bin/agentfs`）
- AgentFS 版本验证成功

### ❌ 失败的步骤

#### 步骤 3: 容器内执行 agentfs run
**状态**: ❌ 失败

**错误信息**:
```
Error: FUSE mount did not become ready within 10s
```

**失败原因**: Docker 容器默认没有 FUSE 权限

## 核心问题分析

### 问题：容器内 FUSE 不可用

**技术原因**:
1. **FUSE 需要特权访问**: FUSE 文件系统需要访问 `/dev/fuse` 设备
2. **Docker 安全限制**: 默认容器没有设备访问权限
3. **Capability 缺失**: 需要 `CAP_SYS_ADMIN` capability

**AgentFS 的 FUSE 依赖**:
- AgentFS 使用 FUSE 实现 overlay 文件系统
- `agentfs run` 命令需要挂载 FUSE 文件系统
- 没有 FUSE，无法创建隔离的 copy-on-write 环境

### 可能的解决方案

#### 方案 1: 使用特权模式（不推荐）
```bash
docker run --privileged \
  -v /tmp/agentfs-test/task-001/.agentfs:/root/.agentfs/run/task-001 \
  ...
```

**优点**: 可能解决 FUSE 问题
**缺点**:
- 严重的安全风险
- 违背容器隔离原则
- 生产环境不可接受

#### 方案 2: 添加设备和 Capability
```bash
docker run \
  --device /dev/fuse \
  --cap-add SYS_ADMIN \
  --security-opt apparmor=unconfined \
  -v /tmp/agentfs-test/task-001/.agentfs:/root/.agentfs/run/task-001 \
  ...
```

**优点**: 比 --privileged 更精确的权限控制
**缺点**:
- 仍然需要提升权限
- 可能在某些环境（如 Kubernetes）中受限
- 增加安全风险

#### 方案 3: 使用 AgentFS 的实验性沙箱模式
```bash
agentfs run --experimental-sandbox --session task-001 <command>
```

**说明**: AgentFS 提供了基于 ptrace 的实验性沙箱，不依赖 FUSE

**状态**: 需要进一步测试验证

#### 方案 4: 回归方案 A（推荐）
使用已验证的宿主机运行方案：
```bash
# 宿主机
agentfs run --session task-001 docker run \
  -v /path/to/workspace:/workspace \
  ...
```

**优点**:
- 已完整验证可行
- 无需容器特权
- 跨平台兼容
- 简单可靠

## 测试结论

### 核心发现

1. **✅ 符号链接方案可行**:
   - `delta.db -> task-001.db` 在 Docker 挂载中正常工作
   - 容器内可以正确访问符号链接

2. **✅ AgentFS 安装成功**:
   - Linux 版本的 agentfs 可以在 ubuntu 容器中正常安装
   - 二进制兼容性没有问题（glibc 环境）

3. **❌ FUSE 是阻塞问题**:
   - 容器内运行 `agentfs run` 需要 FUSE 支持
   - 标准 Docker 容器无法提供 FUSE 访问
   - 需要特权模式或特殊配置

4. **⚠️ 架构限制**:
   - 本方案（容器内 run）比方案 A（宿主机 run）复杂得多
   - 需要额外的权限和配置
   - 安全性和可移植性较差

### 方案可行性评估

| 方案 | 可行性 | 复杂度 | 安全性 | 推荐度 |
|------|--------|--------|--------|--------|
| **方案 A**: 宿主机 run | ✅ 已验证 | 低 | 高 | ⭐⭐⭐⭐⭐ |
| **方案 B**: 容器内 run (特权) | ⚠️ 可能 | 高 | 低 | ⭐ |
| **方案 B**: 容器内 run (实验性) | ❓ 未知 | 中 | 中 | ⭐⭐ |

## 最终建议

### 对于 VibeRepo 项目

**推荐采用方案 A（宿主机运行 agentfs run）**

**理由**:
1. **已完整验证**: 无遗留技术风险
2. **安全性高**: 无需容器特权
3. **实现简单**: 代码复杂度低
4. **跨平台**: macOS 和 Linux 都支持
5. **维护性好**: 依赖少，问题少

**实施方式**:
```rust
// WorkspaceService
pub async fn execute_task(workspace: &Workspace, task: &Task) -> Result<()> {
    let session_id = &workspace.agentfs_session_id;
    let work_dir = &workspace.work_dir;

    // 构建 docker 命令
    let docker_cmd = format!(
        "docker run --rm -v {}:/workspace -w /workspace {} {}",
        work_dir, workspace.image, task.command
    );

    // 在 agentfs 环境中执行
    let status = Command::new("agentfs")
        .args(["run", "--session", session_id, "sh", "-c", &docker_cmd])
        .status()?;

    Ok(())
}
```

### 如果必须使用容器内方案

如果有特殊需求必须在容器内运行 agentfs，建议：

1. **测试实验性沙箱模式**:
   ```bash
   agentfs run --experimental-sandbox --session task-001 <command>
   ```

2. **评估安全风险**:
   - 如果使用 `--privileged` 或 `--cap-add SYS_ADMIN`
   - 需要安全团队审批

3. **考虑替代方案**:
   - 使用其他文件追踪机制（如 inotify）
   - 直接查询 SQLite 数据库而不依赖 FUSE

## 后续行动

### 立即行动

1. ✅ **更新设计文档**: 添加 FUSE 限制说明
2. ✅ **记录测试结果**: 本报告
3. ⏭️ **确认方案选择**: 与团队讨论最终方案

### 可选探索

1. **测试实验性沙箱**:
   - 修改测试脚本使用 `--experimental-sandbox`
   - 验证是否可以绕过 FUSE 限制

2. **测试特权模式**:
   - 仅用于技术验证
   - 不推荐用于生产

3. **研究替代方案**:
   - 探索不依赖 FUSE 的文件追踪方法
   - 评估性能和功能差异

## 附录

### 测试脚本位置
- `/tmp/agentfs-container-integration-test.sh`

### 相关文档
- [测试设计](./2026-01-19-agentfs-container-integration-test-design.md)
- [AgentFS 研究总结](./2025-01-18-agentfs-research-summary.md)
- [AgentFS 架构对比](./2025-01-18-agentfs-architecture-comparison.md)

### 测试数据
- 测试目录: `/tmp/agentfs-test/` (已清理)
- Session ID: `task-001`
- 数据库文件: `task-001.db` (约 4KB + 181KB WAL)

## 结论

经过完整测试，**容器内运行 agentfs run 的方案在标准 Docker 环境中不可行**，主要受限于 FUSE 权限问题。

**推荐继续使用方案 A（宿主机运行 agentfs run）**，这是经过验证的、安全的、可靠的解决方案。

本次测试虽然未能验证容器内方案的完全可行性，但成功验证了：
- 符号链接方案的有效性
- Docker 挂载的兼容性
- AgentFS 在 Linux 容器中的安装和基本功能

这些发现为未来的优化和替代方案提供了宝贵的技术基础。
