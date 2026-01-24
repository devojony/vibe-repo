# Docker 部署指南

本文档介绍如何使用 Docker 构建和运行 VibeRepo。

## 📋 目录

- [快速开始](#快速开始)
- [构建镜像](#构建镜像)
- [运行容器](#运行容器)
- [使用 Docker Compose](#使用-docker-compose)
- [配置说明](#配置说明)
- [数据持久化](#数据持久化)
- [故障排查](#故障排查)

## 🚀 快速开始

### 前置要求

- Docker 20.10+
- Docker Compose 2.0+ (可选)
- 至少 2GB 可用内存
- 至少 5GB 可用磁盘空间

### 使用 Docker Compose（推荐）

1. **复制环境配置文件**

```bash
cp .env.docker .env
```

2. **编辑 `.env` 文件，修改必要的配置**

```bash
# 至少需要修改以下配置：
# - WEBHOOK_SECRET_KEY: 生成一个安全的密钥
# - WEBHOOK_DOMAIN: 设置为你的公网域名
```

3. **启动服务**

```bash
docker-compose up -d
```

4. **查看日志**

```bash
docker-compose logs -f vibe-repo-api
```

5. **访问服务**

- API: http://localhost:3000
- Swagger UI: http://localhost:3000/swagger-ui
- 健康检查: http://localhost:3000/health

## 🔨 构建镜像

### 方法 1: 使用 Docker Compose

```bash
docker-compose build
```

### 方法 2: 手动构建

```bash
cd backend
docker build -t vibe-repo:latest .
```

### 构建优化

Dockerfile 使用了多阶段构建和依赖缓存优化：

- **阶段 1 (builder)**: 编译 Rust 应用
  - 先构建依赖层（利用 Docker 缓存）
  - 再构建应用代码
- **阶段 2 (runtime)**: 运行时环境
  - 基于 Debian Bookworm Slim
  - 只包含必要的运行时依赖
  - 镜像大小约 200MB

## 🏃 运行容器

### 使用 Docker Compose（推荐）

```bash
# 启动服务
docker-compose up -d

# 停止服务
docker-compose down

# 重启服务
docker-compose restart

# 查看状态
docker-compose ps

# 查看日志
docker-compose logs -f
```

### 手动运行容器

```bash
docker run -d \
  --name vibe-repo-api \
  -p 3000:3000 \
  -v /var/run/docker.sock:/var/run/docker.sock:rw \
  -v vibe_repo_data:/data/vibe-repo \
  -e DATABASE_URL=sqlite:/data/vibe-repo/db/vibe-repo.db?mode=rwc \
  -e SERVER_HOST=0.0.0.0 \
  -e SERVER_PORT=3000 \
  -e RUST_LOG=info,vibe_repo=debug \
  --privileged \
  vibe-repo:latest
```

## ⚙️ 配置说明

### 环境变量

详细的环境变量配置请参考 `.env.docker` 文件。主要配置项：

#### 数据库配置

```bash
# SQLite（默认）
DATABASE_URL=sqlite:/data/vibe-repo/db/vibe-repo.db?mode=rwc

# PostgreSQL（生产环境推荐）
DATABASE_URL=postgresql://user:password@postgres:5432/vibe_repo
```

#### 服务器配置

```bash
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
```

#### Webhook 配置

```bash
# 必须是公网可访问的地址
WEBHOOK_DOMAIN=https://your-domain.com

# 生产环境必须修改！
WEBHOOK_SECRET_KEY=your-secret-key-here
```

#### Workspace 配置

```bash
WORKSPACE_BASE_DIR=/data/vibe-repo/workspaces
WORKSPACE_MAX_CONCURRENT_TASKS=3
```

#### 日志配置

```bash
# 日志级别
RUST_LOG=info,vibe_repo=debug

# 启用 backtrace
RUST_BACKTRACE=1
```

### 使用 PostgreSQL

如果需要使用 PostgreSQL 而不是 SQLite：

1. 在 `docker-compose.yml` 中取消注释 PostgreSQL 服务
2. 修改 `.env` 文件中的 `DATABASE_URL`
3. 重启服务

```bash
docker-compose down
docker-compose up -d
```

## 💾 数据持久化

### 数据卷

Docker Compose 会自动创建以下数据卷：

- `vibe_repo_data`: 存储应用数据
  - `/data/vibe-repo/db/`: SQLite 数据库
  - `/data/vibe-repo/storage/`: 文件存储
  - `/data/vibe-repo/task-logs/`: 任务日志
  - `/data/vibe-repo/workspaces/`: 工作空间

- `postgres_data`: PostgreSQL 数据（如果使用）

### 备份数据

```bash
# 备份 SQLite 数据库
docker-compose exec vibe-repo-api sqlite3 /data/vibe-repo/db/vibe-repo.db ".backup /data/vibe-repo/backup.db"

# 导出数据卷
docker run --rm -v vibe_repo_data:/data -v $(pwd):/backup alpine tar czf /backup/vibe-repo-backup.tar.gz /data
```

### 恢复数据

```bash
# 恢复数据卷
docker run --rm -v vibe_repo_data:/data -v $(pwd):/backup alpine tar xzf /backup/vibe-repo-backup.tar.gz -C /
```

## 🔍 故障排查

### 查看日志

```bash
# 查看所有日志
docker-compose logs

# 查看特定服务日志
docker-compose logs vibe-repo-api

# 实时跟踪日志
docker-compose logs -f vibe-repo-api

# 查看最近 100 行日志
docker-compose logs --tail=100 vibe-repo-api
```

### 进入容器

```bash
# 使用 Docker Compose
docker-compose exec vibe-repo-api /bin/bash

# 手动运行
docker exec -it vibe-repo-api /bin/bash
```

### 检查健康状态

```bash
# 使用 Docker
docker ps

# 使用 curl
curl http://localhost:3000/health
```

### 常见问题

#### 1. 容器无法启动

**问题**: 容器启动后立即退出

**解决方法**:
```bash
# 查看容器日志
docker-compose logs vibe-repo-api

# 检查配置
docker-compose config
```

#### 2. 无法连接到 Docker socket

**问题**: 错误信息包含 "permission denied" 或 "cannot connect to Docker daemon"

**解决方法**:
```bash
# 确保 Docker socket 已挂载
docker-compose down
docker-compose up -d

# 检查权限
ls -la /var/run/docker.sock
```

#### 3. 数据库连接失败

**问题**: 无法连接到数据库

**解决方法**:
```bash
# 检查数据库 URL 配置
docker-compose exec vibe-repo-api env | grep DATABASE_URL

# 如果使用 PostgreSQL，检查 PostgreSQL 容器状态
docker-compose ps postgres
docker-compose logs postgres
```

#### 4. 端口冲突

**问题**: 端口 3000 已被占用

**解决方法**:
```bash
# 修改 .env 文件中的 SERVER_PORT
SERVER_PORT=3001

# 或在 docker-compose.yml 中修改端口映射
ports:
  - "3001:3000"
```

#### 5. 内存不足

**问题**: 容器因内存不足被杀死

**解决方法**:
```bash
# 在 docker-compose.yml 中添加内存限制
services:
  vibe-repo-api:
    deploy:
      resources:
        limits:
          memory: 2G
        reservations:
          memory: 1G
```

## 🔐 安全建议

### 生产环境部署

1. **修改默认密钥**
   ```bash
   # 生成安全的 webhook 密钥
   openssl rand -hex 32
   ```

2. **使用 HTTPS**
   - 配置反向代理（Nginx/Caddy）
   - 使用 Let's Encrypt 证书

3. **限制 Docker socket 访问**
   - 考虑使用 Docker API 而不是直接挂载 socket
   - 使用 Docker socket 代理（如 tecnativa/docker-socket-proxy）

4. **定期更新**
   ```bash
   # 更新镜像
   docker-compose pull
   docker-compose up -d
   ```

5. **监控和日志**
   - 配置日志收集（ELK/Loki）
   - 设置监控告警（Prometheus/Grafana）

## 📚 更多资源

- [VibeRepo 文档](../docs/README.md)
- [API 参考](../docs/api/api-reference.md)
- [开发指南](../docs/development/README.md)
- [Docker 官方文档](https://docs.docker.com/)
- [Docker Compose 文档](https://docs.docker.com/compose/)

## 🆘 获取帮助

如果遇到问题：

1. 查看 [故障排查](#故障排查) 部分
2. 搜索 [GitHub Issues](https://github.com/your-org/vibe-repo/issues)
3. 创建新的 Issue 并提供：
   - 错误日志
   - 环境信息（Docker 版本、操作系统等）
   - 复现步骤
