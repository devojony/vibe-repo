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

# Create workspace directory
WORKDIR /workspace

# Keep container running
CMD ["sleep", "infinity"]
