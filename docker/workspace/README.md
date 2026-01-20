# VibeRepo Workspace Container Image

This directory contains the Dockerfile for the standard VibeRepo workspace container.

## Overview

The workspace container provides a clean, isolated environment for automated development tasks. It's based on Ubuntu 22.04 LTS and includes essential development tools.

## Base Image

- **OS**: Ubuntu 22.04 LTS
- **Size**: ~200MB (compressed)
- **Architecture**: amd64

## Pre-installed Tools

### Version Control
- `git` - Git version control system

### Network Tools
- `curl` - Command-line HTTP client
- `wget` - File downloader

### Editors
- `vim` - Vi IMproved text editor
- `nano` - Simple text editor

### Build Tools
- `build-essential` - GCC, G++, make, and other build tools

### Utilities
- `ca-certificates` - SSL/TLS certificates
- `unzip` / `zip` - Archive utilities
- `jq` - JSON processor

## Usage

### Building the Image

From the project root directory:

```bash
docker build -f docker/workspace/Dockerfile -t vibe-repo-workspace:latest .
```

### Running a Container

```bash
docker run -d --name my-workspace vibe-repo-workspace:latest
```

### Executing Commands

```bash
docker exec my-workspace git --version
```

## Customization

The base image is intentionally minimal. Users can install additional tools using init scripts:

```bash
# Example: Install Node.js
apt-get update && apt-get install -y nodejs npm

# Example: Install Python
apt-get update && apt-get install -y python3 python3-pip

# Example: Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

## Design Philosophy

1. **Minimal Base**: Only essential tools are pre-installed
2. **Flexibility**: Users customize via init scripts
3. **Stability**: Based on Ubuntu LTS for long-term support
4. **Security**: Regular security updates from Ubuntu

## Image Management

The VibeRepo backend automatically manages this image:

- **Auto-build**: Built automatically when creating the first workspace
- **Reuse**: All workspaces share the same image
- **Update**: Can be rebuilt via API endpoint

See the main documentation for API details.

### Automatic Management

VibeRepo's Container Lifecycle Management system handles this image automatically:

1. **First Workspace Creation**: Image is built automatically if it doesn't exist
2. **Subsequent Workspaces**: Reuse the existing image (fast startup)
3. **Image Updates**: Rebuild via API when Dockerfile changes
4. **Conflict Detection**: Prevents deletion if workspaces are using the image

### API Integration

Query image information:
```bash
curl http://localhost:3000/api/settings/workspace/image
```

Response:
```json
{
  "exists": true,
  "image_name": "vibe-repo-workspace:latest",
  "image_id": "sha256:abc123...",
  "size_mb": 215.4,
  "created_at": "2026-01-20T09:00:00Z",
  "in_use_by_workspaces": 3,
  "workspace_ids": [1, 2, 3]
}
```

Rebuild image after Dockerfile changes:
```bash
# Normal rebuild (fails if workspaces are using it)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": false}'

# Force rebuild (rebuilds even if in use)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": true}'
```

Delete image (only if no workspaces are using it):
```bash
curl -X DELETE http://localhost:3000/api/settings/workspace/image
```

### Monitoring Image Usage

Check which workspaces are using this image:
```bash
# Get image info (includes workspace IDs)
curl http://localhost:3000/api/settings/workspace/image

# Response includes:
# - in_use_by_workspaces: Number of workspaces
# - workspace_ids: Array of workspace IDs [1, 2, 3]
```

Before deleting or rebuilding:
1. Check image usage via API
2. Stop or delete workspaces if needed
3. Proceed with image operation

### Updating the Image

When you modify the Dockerfile:

1. **Update Dockerfile**: Edit `docker/workspace/Dockerfile`
2. **Rebuild Image**: Use API with `force=true` if workspaces are running
3. **Restart Workspaces**: Restart workspaces to use the new image

```bash
# 1. Edit Dockerfile
vim docker/workspace/Dockerfile

# 2. Rebuild image (force if needed)
curl -X POST http://localhost:3000/api/settings/workspace/image/rebuild \
  -H "Content-Type: application/json" \
  -d '{"force": true}'

# 3. Restart workspaces to use new image
curl -X POST http://localhost:3000/api/workspaces/1/restart
curl -X POST http://localhost:3000/api/workspaces/2/restart
```

**Note**: Workspaces using the old image will continue to work, but won't get the updates until restarted.

## Troubleshooting

### Build Fails

If the build fails, check:
1. Docker daemon is running
2. Internet connection is available (for apt-get)
3. Sufficient disk space

### Container Exits Immediately

The container uses `sleep infinity` to stay running. If it exits:
1. Check Docker logs: `docker logs <container-id>`
2. Verify the image was built correctly
3. Check for resource constraints

## Version History

- **v0.3.0** (2026-01-20): Initial workspace image
  - Ubuntu 22.04 base
  - Essential development tools
  - Minimal footprint (~200MB)
