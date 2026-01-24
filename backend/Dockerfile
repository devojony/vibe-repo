# ============================================
# Build Stage - 构建 Rust 应用
# ============================================
FROM rust:1.83-bookworm AS builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制依赖文件并构建依赖层（利用 Docker 缓存）
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 复制源代码
COPY src ./src

# 构建应用程序（如果依赖没变，这一步会很快）
RUN cargo build --release

# ============================================
# Runtime Stage - 运行时环境
# ============================================
FROM debian:bookworm-slim

# 安装运行时依赖
# - ca-certificates: HTTPS 连接
# - git: Git 操作
# - curl: 健康检查和 HTTP 请求
# - docker.io: Docker 客户端（用于管理容器）
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    curl \
    docker.io \
    && rm -rf /var/lib/apt/lists/*

# 创建应用用户（非 root）
RUN useradd -m -u 1001 -s /bin/bash app

# 创建数据目录
RUN mkdir -p /data/vibe-repo/db \
             /data/vibe-repo/storage \
             /data/vibe-repo/task-logs \
             /data/vibe-repo/workspaces && \
    chown -R app:app /data/vibe-repo

# 创建工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/vibe-repo /app/vibe-repo

# 更改所有权
RUN chown -R app:app /app

# 切换到非 root 用户
USER app

# 暴露端口
EXPOSE 3000

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# 启动应用
CMD ["./vibe-repo"]