FROM ubuntu:22.04

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

# Install basic tools
RUN apt-get update && apt-get install -y \
    # Version control
    git \
    # Network tools
    curl \
    wget \
    # Editors
    vim \
    nano \
    # Build tools
    build-essential \
    # SSL certificates
    ca-certificates \
    # Other common tools
    unzip \
    zip \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 20.x LTS (required by OpenCode)
# Using NodeSource repository for latest LTS version
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Install Bun runtime (fast JavaScript runtime for package management)
# Official installation from https://bun.sh
RUN curl -fsSL https://bun.sh/install | bash

# Set up Bun environment variables
ENV BUN_INSTALL="/root/.bun"
ENV PATH="$BUN_INSTALL/bin:$PATH"

# Install OpenCode (AI coding agent with ACP support)
# Using Bun for faster installation, but OpenCode runs on Node.js
RUN bun install -g opencode-ai

# Verify installations
RUN node --version && \
    bun --version && \
    opencode --version && \
    opencode acp --version

# Optional: Install Claude Code adapter (commented out by default)
# Uncomment the following lines to install Claude Code adapter:
# RUN curl -fsSL https://claude.ai/install.sh | bash
# ENV CLAUDE_CODE_PATH="/root/.claude-code/bin"
# ENV PATH="$CLAUDE_CODE_PATH:$PATH"

# Create workspace directory
WORKDIR /workspace

# Keep container running
CMD ["sleep", "infinity"]
