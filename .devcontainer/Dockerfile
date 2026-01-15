# 使用官方 Rust 镜像
FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# [可选] 安装额外的系统依赖
# 例如：libssl-dev 是很多 Rust 项目（如使用 openssl 的项目）需要的
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends libssl-dev pkg-config

# [可选] 安装常用 Rust 工具
USER vscode
RUN rustup component add rustfmt clippy