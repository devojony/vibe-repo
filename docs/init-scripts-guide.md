# Init Scripts 使用指南

## 概述

Init Scripts 是 VibeRepo 提供的一个强大功能，允许你在 workspace 容器启动后自动执行自定义的 shell 脚本。这个功能取代了之前的 `custom_dockerfile_path` 方法，提供了更灵活和易于管理的容器配置方案。

## 核心特性

### 1. 自动执行
- 脚本在容器启动后自动运行
- 无需手动干预
- 支持后台执行

### 2. 混合存储策略
- **小输出 (≤4KB)**: 直接存储在数据库中
- **大输出 (>4KB)**: 存储在文件系统，数据库保存最后 4KB 作为摘要
- 自动选择最优存储方式

### 3. 超时控制
- 默认超时: 300 秒（5 分钟）
- 可配置: 1-3600 秒
- 超时后自动终止脚本

### 4. 状态跟踪
脚本执行有 6 个状态：
- `Pending`: 等待执行
- `Running`: 正在执行
- `Success`: 执行成功（退出码 0）
- `Failed`: 执行失败（非零退出码）
- `Timeout`: 执行超时
- `Cancelled`: 已取消

### 5. 并发控制
- 同一 workspace 的脚本不能同时执行
- 尝试并发执行会返回 409 Conflict
- 使用数据库锁防止竞态条件

### 6. 日志管理
- 自动记录 stdout 和 stderr
- 30 天自动清理旧日志
- 支持下载完整日志文件

## API 使用

### 创建带 Init Script 的 Workspace

```bash
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 1,
    "init_script": "#!/bin/bash\necho \"Initializing...\"\napt-get update\napt-get install -y git curl",
    "script_timeout_seconds": 600
  }'
```

**响应示例：**
```json
{
  "id": 1,
  "repository_id": 1,
  "workspace_status": "Initializing",
  "init_script": {
    "id": 1,
    "workspace_id": 1,
    "script_content": "#!/bin/bash\necho \"Initializing...\"\n...",
    "timeout_seconds": 600,
    "status": "Pending",
    "output_summary": null,
    "has_full_log": false,
    "executed_at": null,
    "created_at": "2026-01-19T14:20:54Z",
    "updated_at": "2026-01-19T14:20:54Z"
  }
}
```

### 更新 Init Script

```bash
curl -X PUT http://localhost:3000/api/workspaces/1/init-script \
  -H "Content-Type: application/json" \
  -d '{
    "script_content": "#!/bin/bash\necho \"Updated script\"\ndate",
    "timeout_seconds": 300,
    "execute_immediately": false
  }'
```

**参数说明：**
- `script_content`: 脚本内容（必需）
- `timeout_seconds`: 超时时间（可选，默认 300）
- `execute_immediately`: 是否立即执行（可选，默认 false）

### 手动执行 Init Script

```bash
curl -X POST http://localhost:3000/api/workspaces/1/init-script/execute \
  -H "Content-Type: application/json" \
  -d '{
    "force": false
  }'
```

**参数说明：**
- `force`: 是否强制执行（可选，默认 false）
  - `false`: 如果脚本正在运行，返回 409 Conflict
  - `true`: 等待当前执行完成后再执行（未来功能）

### 查看执行日志

```bash
curl http://localhost:3000/api/workspaces/1/init-script/logs
```

**响应示例：**
```json
{
  "status": "Success",
  "output_summary": "Initializing...\nReading package lists...\nDone\n",
  "has_full_log": false,
  "executed_at": "2026-01-19T14:25:30Z"
}
```

### 下载完整日志

```bash
curl http://localhost:3000/api/workspaces/1/init-script/logs/full -o script.log
```

仅当 `has_full_log` 为 `true` 时可用（输出 >4KB）。

## 常见用例

### 1. 安装开发工具

```bash
#!/bin/bash
set -e  # 遇到错误立即退出

echo "Installing development tools..."

# 更新包列表
apt-get update

# 安装常用工具
apt-get install -y \
  git \
  curl \
  wget \
  vim \
  build-essential

echo "Development tools installed successfully!"
```

### 2. 配置 Node.js 环境

```bash
#!/bin/bash
set -e

echo "Setting up Node.js environment..."

# 安装 Node.js
curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
apt-get install -y nodejs

# 验证安装
node --version
npm --version

# 安装全局包
npm install -g yarn pnpm

echo "Node.js environment ready!"
```

### 3. 克隆私有仓库

```bash
#!/bin/bash
set -e

echo "Cloning private repositories..."

# 配置 Git
git config --global user.name "Bot User"
git config --global user.email "bot@example.com"

# 克隆仓库（使用环境变量中的 token）
git clone https://${GIT_TOKEN}@github.com/org/private-repo.git /workspace/repo

echo "Repository cloned successfully!"
```

### 4. 数据库初始化

```bash
#!/bin/bash
set -e

echo "Initializing database..."

# 等待数据库就绪
until pg_isready -h db -p 5432; do
  echo "Waiting for database..."
  sleep 2
done

# 运行迁移
cd /workspace/app
npm run migrate

echo "Database initialized!"
```

### 5. 下载和解压数据集

```bash
#!/bin/bash
set -e

echo "Downloading dataset..."

# 创建数据目录
mkdir -p /workspace/data

# 下载数据集
wget -O /workspace/data/dataset.tar.gz \
  https://example.com/dataset.tar.gz

# 解压
cd /workspace/data
tar -xzf dataset.tar.gz
rm dataset.tar.gz

echo "Dataset ready at /workspace/data"
```

## 最佳实践

### 1. 使用 set -e

始终在脚本开头添加 `set -e`，这样任何命令失败都会立即终止脚本：

```bash
#!/bin/bash
set -e

# 你的脚本内容
```

### 2. 添加日志输出

使用 `echo` 输出进度信息，便于调试：

```bash
echo "Step 1: Updating packages..."
apt-get update

echo "Step 2: Installing tools..."
apt-get install -y git curl
```

### 3. 设置合理的超时

根据脚本复杂度设置超时：
- 简单脚本: 60-300 秒
- 安装软件包: 300-600 秒
- 下载大文件: 600-1800 秒

### 4. 处理错误

添加错误处理逻辑：

```bash
#!/bin/bash

# 错误处理函数
handle_error() {
  echo "Error occurred at line $1"
  exit 1
}

trap 'handle_error $LINENO' ERR

# 你的脚本内容
```

### 5. 使用环境变量

避免在脚本中硬编码敏感信息：

```bash
#!/bin/bash

# 使用环境变量
DB_HOST=${DB_HOST:-localhost}
DB_PORT=${DB_PORT:-5432}

echo "Connecting to $DB_HOST:$DB_PORT"
```

### 6. 幂等性

确保脚本可以多次执行而不会出错：

```bash
#!/bin/bash

# 检查是否已安装
if ! command -v git &> /dev/null; then
  echo "Installing git..."
  apt-get install -y git
else
  echo "Git already installed"
fi
```

### 7. 清理临时文件

脚本结束前清理临时文件：

```bash
#!/bin/bash
set -e

# 创建临时目录
TEMP_DIR=$(mktemp -d)

# 使用 trap 确保清理
trap "rm -rf $TEMP_DIR" EXIT

# 你的脚本内容
cd $TEMP_DIR
# ...
```

## 故障排查

### 脚本执行失败

1. **查看日志**
   ```bash
   curl http://localhost:3000/api/workspaces/1/init-script/logs
   ```

2. **检查退出码**
   - 非零退出码表示失败
   - 查看 stderr 输出

3. **常见问题**
   - 权限不足: 使用 `sudo` 或以 root 运行
   - 包不存在: 先运行 `apt-get update`
   - 网络问题: 检查容器网络配置

### 脚本超时

1. **增加超时时间**
   ```bash
   curl -X PUT http://localhost:3000/api/workspaces/1/init-script \
     -H "Content-Type: application/json" \
     -d '{
       "script_content": "...",
       "timeout_seconds": 1800
     }'
   ```

2. **优化脚本**
   - 移除不必要的操作
   - 使用并行下载
   - 缓存常用包

### 并发执行冲突

如果收到 409 Conflict 错误：

1. **等待当前执行完成**
   ```bash
   # 检查状态
   curl http://localhost:3000/api/workspaces/1/init-script/logs
   ```

2. **取消当前执行**（未来功能）
   ```bash
   curl -X POST http://localhost:3000/api/workspaces/1/init-script/cancel
   ```

## 性能优化

### 1. 使用包缓存

```bash
#!/bin/bash
set -e

# 使用本地镜像
echo "deb http://mirrors.aliyun.com/ubuntu/ focal main" > /etc/apt/sources.list

apt-get update
apt-get install -y git curl
```

### 2. 并行下载

```bash
#!/bin/bash
set -e

# 并行下载多个文件
wget -P /workspace/data \
  https://example.com/file1.tar.gz \
  https://example.com/file2.tar.gz \
  https://example.com/file3.tar.gz &

wait  # 等待所有下载完成
```

### 3. 减少日志输出

对于大量输出的命令，重定向到文件：

```bash
#!/bin/bash
set -e

# 减少日志输出
apt-get update > /dev/null 2>&1
apt-get install -y git curl > /dev/null 2>&1

echo "Installation complete!"
```

## 安全考虑

### 1. 避免硬编码密钥

❌ **不要这样做：**
```bash
git clone https://user:password@github.com/org/repo.git
```

✅ **应该这样做：**
```bash
git clone https://${GIT_TOKEN}@github.com/org/repo.git
```

### 2. 验证下载内容

```bash
#!/bin/bash
set -e

# 下载文件
wget -O /tmp/file.tar.gz https://example.com/file.tar.gz

# 验证校验和
echo "expected_sha256  /tmp/file.tar.gz" | sha256sum -c -

# 解压
tar -xzf /tmp/file.tar.gz
```

### 3. 限制网络访问

```bash
#!/bin/bash
set -e

# 只允许访问特定域名
iptables -A OUTPUT -d example.com -j ACCEPT
iptables -A OUTPUT -j DROP
```

## 迁移指南

如果你之前使用 `custom_dockerfile_path`，请参考 [migration-guide-init-scripts.md](./migration-guide-init-scripts.md) 获取详细的迁移步骤。

## 相关资源

- [API 文档](http://localhost:3000/swagger-ui)
- [迁移指南](./migration-guide-init-scripts.md)
- [README](../README.md)

## 支持

如有问题或建议，请：
1. 查看 [故障排查](#故障排查) 部分
2. 查看 API 文档
3. 提交 Issue 到 GitHub 仓库
