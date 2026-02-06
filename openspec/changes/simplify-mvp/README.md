# 简化 MVP 变更

## 📋 文档索引

### 🚨 必读文档（实施前）

1. **[CRITICAL-WARNINGS.md](./CRITICAL-WARNINGS.md)** ⚠️
   - **必须先读这个！**
   - 说明 Git Provider 核心模块必须保留
   - 区分 API 层和核心模块的差异
   - 包含详细的影响分析和检查清单

### 📄 核心文档

2. **[proposal.md](./proposal.md)**
   - 变更提案：为什么要简化
   - 变更内容：删除什么、保留什么
   - 影响分析：代码、数据库、API

3. **[design.md](./design.md)**
   - 技术设计：9 个关键决策
   - 风险评估：5 个主要风险
   - 迁移计划：7 个阶段

4. **[tasks.md](./tasks.md)**
   - 实施任务：18 个任务组，200+ 详细任务
   - 每个任务都有评估步骤
   - 包含验证脚本和检查清单

### 📐 能力规范

5. **[specs/](./specs/)**
   - `simplified-task-execution.md` - 简化的任务执行
   - `minimal-api-surface.md` - 最小化 API（8 个端点）
   - `single-agent-workspace.md` - 单 Agent 模式
   - `task-state-machine.md` - 简化的状态机

## 🎯 快速开始

### 实施前检查

```bash
# 1. 阅读关键警告
cat openspec/changes/simplify-mvp/CRITICAL-WARNINGS.md

# 2. 确认理解以下要点：
# - 只删除 src/api/settings/providers/ (API 层)
# - 保留 src/git_provider/ (核心模块)
# - Git Provider 是 PR 创建的核心依赖
```

### 开始实施

```bash
# 1. 设置 worktree（Task 0）
mkdir -p .worktrees
git worktree add .worktrees/simplify-mvp mvp-simplified
cd .worktrees/simplify-mvp

# 2. 按照 tasks.md 顺序执行
# 从 Task 1 开始，严格按顺序执行
# 每个任务都有评估步骤，不要跳过
```

## 📊 变更概览

### 代码量变化

| 指标 | 当前 | 目标 | 变化 |
|------|------|------|------|
| 代码行数 | 15,000+ | ~6,000 | -60% |
| 数据库表 | 10 | 4 | -60% |
| 后台服务 | 6 | 1 | -83% |
| API 端点 | 50+ | 8 | -84% |

### 保留的核心功能

✅ Webhook 接收和处理  
✅ Task 创建和执行  
✅ Docker 容器运行  
✅ **PR 创建** (依赖 `src/git_provider/`)  
✅ **Issue 关闭** (依赖 `src/git_provider/`)  
✅ 日志查询  

### 删除的功能

❌ Issue 轮询（保留 Webhook）  
❌ WebSocket 实时日志（改用 REST 轮询）  
❌ Init Scripts（使用预构建镜像）  
❌ 失败分析系统（简化为错误日志）  
❌ 执行历史表（只保留最近日志）  
❌ 多 Agent 支持（单 Agent 模式）  
❌ Provider 管理 API（改用环境变量）  
❌ Webhook 重试和清理服务  

## ⚠️ 关键注意事项

### 1. Git Provider 模块

**删除**:
- `src/api/settings/providers/` (727 行) - API 端点

**保留**:
- `src/git_provider/` (4,111 行) - 核心模块
- 原因：PR 创建和 Issue 关闭完全依赖此模块
- 删除将导致系统失去核心价值

### 2. 配置方式变更

**旧方式**: REST API 管理 Provider 配置  
**新方式**: 环境变量配置

```bash
# 新的配置方式
export GITHUB_TOKEN="ghp_xxx"
export GITHUB_BASE_URL="https://github.com"
export GITEA_TOKEN="xxx"
export GITEA_BASE_URL="https://gitea.example.com"
```

### 3. Breaking Changes

这是一个 **BREAKING** 变更：
- 不支持从完整版自动迁移
- 建议全新部署
- 配置方式完全改变
- API 端点大幅减少

## 📝 实施顺序

1. **Task 0**: 设置 Git Worktree
2. **Task 1**: 创建分支和备份
3. **Task 2**: 删除后台服务（8 个服务）
4. **Task 3**: 删除 WebSocket（8 个阶段）
5. **Task 4**: 简化数据库（10 个子任务）
6. **Task 5-18**: 继续其他任务

每个任务都包含：
- 评估步骤（搜索引用、检查依赖）
- 删除步骤（删除文件、更新代码）
- 验证步骤（编译、测试）

## 🔍 验证清单

实施完成后，验证以下内容：

- [ ] 代码行数减少到 ~6,000 行
- [ ] 数据库表减少到 4 个
- [ ] 只有 1 个后台服务运行
- [ ] 只有 8 个 API 端点
- [ ] PR 创建功能正常工作
- [ ] Issue 关闭功能正常工作
- [ ] 所有核心测试通过（至少 200 个）
- [ ] `cargo check` 无错误
- [ ] `cargo clippy` 无警告

## 📚 相关资源

- [VibeRepo 主文档](../../../docs/README.md)
- [开发指南](../../../AGENTS.md)
- [API 参考](../../../docs/api/api-reference.md)
- [数据库架构](../../../docs/database/schema.md)

## 🆘 遇到问题？

1. **误删了 git_provider 模块？**
   ```bash
   git checkout HEAD -- backend/src/git_provider/
   ```

2. **编译错误？**
   - 检查是否误删了核心模块
   - 查看 CRITICAL-WARNINGS.md
   - 运行 `cargo check --verbose`

3. **测试失败？**
   - 确认只删除了 API 层
   - 确认核心服务文件未被修改
   - 查看具体的测试错误信息

---

**最后提醒**: 开始实施前，请务必阅读 [CRITICAL-WARNINGS.md](./CRITICAL-WARNINGS.md)！
