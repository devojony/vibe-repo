# DevContainer Configuration Guide

**Version:** 0.4.0-mvp

This guide explains how to use `devcontainer.json` to customize your VibeRepo workspace environments.

## Overview

VibeRepo uses the [Development Containers specification](https://containers.dev/) to create isolated, reproducible development environments. Each repository can have its own `devcontainer.json` configuration file that defines:

- Base Docker image
- Development tools and features
- Environment variables
- Lifecycle hooks
- Port forwarding
- And more...

## Quick Start

### 1. Create devcontainer.json

Create a `.devcontainer/devcontainer.json` file in your repository root:

```json
{
  "name": "My Project",
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu",
  "features": {
    "ghcr.io/devcontainers/features/node:1": {
      "version": "20"
    }
  }
}
```

### 2. Add Repository to VibeRepo

```bash
curl -X POST http://localhost:3000/api/repositories \
  -H "Content-Type: application/json" \
  -d '{
    "provider_type": "github",
    "provider_base_url": "https://api.github.com",
    "access_token": "ghp_xxxxxxxxxxxx",
    "full_name": "owner/my-repo",
    "branch_name": "vibe-dev"
  }'
```

### 3. Trigger Task

When you create a task, VibeRepo will:
1. Clone your repository
2. Detect `.devcontainer/devcontainer.json`
3. Create a container using your configuration
4. Install the agent (OpenCode/Claude Code)
5. Execute the task

## Configuration Options

### Base Image

Specify the Docker image to use:

```json
{
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu"
}
```

Or use a Dockerfile:

```json
{
  "build": {
    "dockerfile": "Dockerfile",
    "context": ".."
  }
}
```

### Features

Add pre-built development tools using [Dev Container Features](https://containers.dev/features):

```json
{
  "features": {
    "ghcr.io/devcontainers/features/node:1": {
      "version": "20"
    },
    "ghcr.io/devcontainers/features/python:1": {
      "version": "3.11"
    },
    "ghcr.io/devcontainers/features/git:1": {}
  }
}
```

### Environment Variables

Set environment variables for your workspace:

```json
{
  "containerEnv": {
    "NODE_ENV": "development",
    "API_URL": "https://api.example.com"
  }
}
```

### Lifecycle Hooks

Run commands at different stages:

```json
{
  "postCreateCommand": "npm install",
  "postStartCommand": "npm run dev",
  "postAttachCommand": "echo 'Container ready!'"
}
```

### Port Forwarding

Forward ports from the container:

```json
{
  "forwardPorts": [3000, 8080],
  "portsAttributes": {
    "3000": {
      "label": "Application",
      "onAutoForward": "notify"
    }
  }
}
```

### Mounts

Mount volumes into the container:

```json
{
  "mounts": [
    {
      "source": "${localWorkspaceFolder}/.cache",
      "target": "/workspace/.cache",
      "type": "bind"
    }
  ]
}
```

## Common Use Cases

### Node.js Project

```json
{
  "name": "Node.js Project",
  "image": "mcr.microsoft.com/devcontainers/javascript-node:20",
  "features": {
    "ghcr.io/devcontainers/features/node:1": {
      "version": "20"
    }
  },
  "postCreateCommand": "npm install",
  "forwardPorts": [3000]
}
```

### Python Project

```json
{
  "name": "Python Project",
  "image": "mcr.microsoft.com/devcontainers/python:3.11",
  "features": {
    "ghcr.io/devcontainers/features/python:1": {
      "version": "3.11"
    }
  },
  "postCreateCommand": "pip install -r requirements.txt",
  "containerEnv": {
    "PYTHONUNBUFFERED": "1"
  }
}
```

### Rust Project

```json
{
  "name": "Rust Project",
  "image": "mcr.microsoft.com/devcontainers/rust:latest",
  "features": {
    "ghcr.io/devcontainers/features/rust:1": {}
  },
  "postCreateCommand": "cargo build"
}
```

### Full-Stack Project

```json
{
  "name": "Full-Stack Project",
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu",
  "features": {
    "ghcr.io/devcontainers/features/node:1": {
      "version": "20"
    },
    "ghcr.io/devcontainers/features/python:1": {
      "version": "3.11"
    },
    "ghcr.io/devcontainers/features/docker-in-docker:2": {}
  },
  "postCreateCommand": "npm install && pip install -r requirements.txt",
  "forwardPorts": [3000, 8000]
}
```

## Default Configuration

If your repository doesn't have a `devcontainer.json`, VibeRepo uses this default configuration:

```json
{
  "name": "VibeRepo Default",
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu",
  "features": {
    "ghcr.io/devcontainers/features/common-utils:2": {
      "installZsh": true,
      "installOhMyZsh": true,
      "upgradePackages": true
    },
    "ghcr.io/devcontainers/features/git:1": {
      "version": "latest"
    }
  },
  "remoteUser": "vscode"
}
```

## Validation

VibeRepo validates your `devcontainer.json` before creating the workspace. Common validation errors:

### Missing Required Fields

```json
{
  "name": "My Project"
  // ❌ Error: Missing 'image' or 'build' field
}
```

Fix:
```json
{
  "name": "My Project",
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu"
}
```

### Invalid JSON

```json
{
  "name": "My Project",
  "image": "ubuntu:22.04",  // ❌ Trailing comma
}
```

Fix:
```json
{
  "name": "My Project",
  "image": "ubuntu:22.04"
}
```

## Troubleshooting

### Container Creation Fails

**Problem:** Container fails to start

**Solution:**
1. Check Docker daemon is running
2. Verify image exists: `docker pull <image-name>`
3. Check logs in VibeRepo for detailed error messages

### Feature Installation Fails

**Problem:** Dev Container Feature fails to install

**Solution:**
1. Verify feature name and version
2. Check feature documentation at [containers.dev/features](https://containers.dev/features)
3. Try using a different version or alternative feature

### Agent Installation Fails

**Problem:** Agent (OpenCode/Claude Code) fails to install

**Solution:**
1. Ensure Bun is available in the container
2. Check network connectivity
3. Verify AGENT_API_KEY is set correctly

## Best Practices

### 1. Use Official Images

Prefer official Microsoft Dev Container images:

```json
{
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu"
}
```

### 2. Pin Versions

Always specify versions for reproducibility:

```json
{
  "features": {
    "ghcr.io/devcontainers/features/node:1": {
      "version": "20.10.0"  // ✅ Specific version
    }
  }
}
```

### 3. Minimize Image Size

Only install what you need:

```json
{
  "features": {
    "ghcr.io/devcontainers/features/node:1": {
      "version": "20",
      "installYarnUsingApt": false  // Skip if not needed
    }
  }
}
```

### 4. Use Lifecycle Hooks Wisely

Avoid long-running commands in `postCreateCommand`:

```json
{
  "postCreateCommand": "npm install",  // ✅ Fast
  "postStartCommand": "npm run dev"    // ❌ Long-running, will block
}
```

### 5. Document Your Configuration

Add comments (JSON5 format supported):

```jsonc
{
  "name": "My Project",
  // Use Ubuntu base image for compatibility
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu",
  "features": {
    // Node.js 20 LTS for production parity
    "ghcr.io/devcontainers/features/node:1": {
      "version": "20"
    }
  }
}
```

## Resources

- **[Dev Containers Specification](https://containers.dev/)** - Official specification
- **[Dev Container Features](https://containers.dev/features)** - Pre-built features catalog
- **[Dev Container Images](https://github.com/devcontainers/images)** - Official images
- **[VibeRepo Examples](../../examples/devcontainer/)** - Example configurations

## Next Steps

- **[Configuration Examples](../../examples/devcontainer/)** - More example configurations
- **[Troubleshooting Guide](./troubleshooting.md)** - Common issues and solutions
- **[API Reference](./api-reference.md)** - Repository API documentation
