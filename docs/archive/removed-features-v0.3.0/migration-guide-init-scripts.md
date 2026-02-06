# Migration Guide: custom_dockerfile_path to Init Scripts

This guide explains how to migrate from the deprecated `custom_dockerfile_path` field to the new init script feature.

## Overview

The `custom_dockerfile_path` field has been removed from the `workspaces` table and replaced with a more flexible init script system. Instead of building custom Docker images, you can now provide shell scripts that run automatically after container startup.

## Key Differences

| Feature | custom_dockerfile_path (Old) | Init Scripts (New) |
|---------|------------------------------|-------------------|
| **Approach** | Custom Dockerfile build | Shell script execution in running container |
| **Flexibility** | Requires image rebuild | Can be updated without rebuild |
| **Execution Time** | During image build | After container starts |
| **Storage** | File path only | Hybrid (DB + filesystem) |
| **Logs** | Not tracked | Full execution logs with status |
| **Timeout** | Build timeout | Configurable script timeout |
| **Updates** | Requires rebuild | Instant updates |

## Migration Steps

### Step 1: Identify Existing Dockerfiles

If you were using `custom_dockerfile_path`, you likely have custom Dockerfiles that install dependencies or configure the environment.

Example old Dockerfile:
```dockerfile
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    git \
    curl \
    python3 \
    python3-pip \
    nodejs \
    npm

RUN pip3 install requests flask
RUN npm install -g typescript

WORKDIR /workspace
```

### Step 2: Convert to Init Script

Convert your Dockerfile commands to a shell script:

```bash
#!/bin/bash
set -e  # Exit on error

# Update package lists
apt-get update

# Install system packages
apt-get install -y \
    git \
    curl \
    python3 \
    python3-pip \
    nodejs \
    npm

# Install Python packages
pip3 install requests flask

# Install Node packages
npm install -g typescript

echo "Init script completed successfully"
```

### Step 3: Create Workspace with Init Script

Use the new API to create a workspace with an init script:

```bash
curl -X POST http://localhost:3000/api/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "repository_id": 1,
    "init_script": "#!/bin/bash\nset -e\napt-get update\napt-get install -y git curl python3 python3-pip nodejs npm\npip3 install requests flask\nnpm install -g typescript\necho \"Init script completed successfully\"",
    "script_timeout_seconds": 600,
    "image_source": "default",
    "max_concurrent_tasks": 3,
    "cpu_limit": 2.0,
    "memory_limit": "4GB",
    "disk_limit": "10GB"
  }'
```

### Step 4: Update Existing Workspaces

For existing workspaces, add an init script:

```bash
curl -X PUT http://localhost:3000/api/workspaces/1/init-script \
  -H "Content-Type: application/json" \
  -d '{
    "script_content": "#!/bin/bash\nset -e\napt-get update\napt-get install -y git curl\necho \"Setup complete\"",
    "timeout_seconds": 300,
    "execute_immediately": true
  }'
```

## Best Practices

### 1. Use Error Handling

Always include `set -e` at the beginning of your script to exit on errors:

```bash
#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status
```

### 2. Set Appropriate Timeouts

Consider the complexity of your script when setting timeouts:

- Simple package installations: 300 seconds (default)
- Complex builds or large downloads: 600-1800 seconds
- Very long operations: Consider breaking into multiple scripts

### 3. Add Logging

Include echo statements to track progress:

```bash
#!/bin/bash
set -e

echo "Starting package installation..."
apt-get update
apt-get install -y git curl

echo "Installing Python dependencies..."
pip3 install -r requirements.txt

echo "Init script completed successfully"
```

### 4. Handle Idempotency

Make your scripts idempotent (safe to run multiple times):

```bash
#!/bin/bash
set -e

# Check if already installed
if ! command -v node &> /dev/null; then
    echo "Installing Node.js..."
    apt-get update
    apt-get install -y nodejs npm
else
    echo "Node.js already installed"
fi
```

### 5. Use Conditional Installation

Only install what's needed:

```bash
#!/bin/bash
set -e

# Install only if package.json exists
if [ -f "/workspace/package.json" ]; then
    echo "Installing npm dependencies..."
    cd /workspace
    npm install
fi

# Install only if requirements.txt exists
if [ -f "/workspace/requirements.txt" ]; then
    echo "Installing Python dependencies..."
    pip3 install -r /workspace/requirements.txt
fi
```

## Common Migration Patterns

### Pattern 1: System Package Installation

**Old Dockerfile:**
```dockerfile
RUN apt-get update && apt-get install -y package1 package2
```

**New Init Script:**
```bash
#!/bin/bash
set -e
apt-get update
apt-get install -y package1 package2
```

### Pattern 2: Language Runtime Installation

**Old Dockerfile:**
```dockerfile
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
RUN apt-get install -y nodejs
```

**New Init Script:**
```bash
#!/bin/bash
set -e
curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
apt-get install -y nodejs
```

### Pattern 3: Environment Configuration

**Old Dockerfile:**
```dockerfile
ENV PATH="/opt/custom/bin:${PATH}"
RUN echo 'export PATH="/opt/custom/bin:$PATH"' >> /etc/profile
```

**New Init Script:**
```bash
#!/bin/bash
set -e
echo 'export PATH="/opt/custom/bin:$PATH"' >> /etc/profile
echo 'export PATH="/opt/custom/bin:$PATH"' >> ~/.bashrc
```

### Pattern 4: File Downloads

**Old Dockerfile:**
```dockerfile
RUN wget https://example.com/tool.tar.gz && \
    tar -xzf tool.tar.gz && \
    mv tool /usr/local/bin/
```

**New Init Script:**
```bash
#!/bin/bash
set -e
wget https://example.com/tool.tar.gz
tar -xzf tool.tar.gz
mv tool /usr/local/bin/
rm tool.tar.gz
```

## Monitoring and Debugging

### Check Script Status

```bash
curl http://localhost:3000/api/workspaces/1/init-script/logs
```

Response:
```json
{
  "status": "Success",
  "output_summary": "=== STDOUT ===\nStarting package installation...\nPackage installed successfully\n\n=== STDERR ===\n",
  "has_full_log": false,
  "executed_at": "2026-01-19T12:34:56Z"
}
```

### Download Full Logs

For scripts with large output:

```bash
curl http://localhost:3000/api/workspaces/1/init-script/logs/full -o init-script.log
```

### Re-execute Script

If a script fails, fix it and re-execute:

```bash
curl -X PUT http://localhost:3000/api/workspaces/1/init-script \
  -H "Content-Type: application/json" \
  -d '{
    "script_content": "#!/bin/bash\nset -e\n# Fixed script here",
    "timeout_seconds": 300,
    "execute_immediately": true
  }'
```

## Troubleshooting

### Script Times Out

**Problem:** Script execution exceeds timeout.

**Solution:** Increase timeout or optimize script:

```bash
curl -X PUT http://localhost:3000/api/workspaces/1/init-script \
  -H "Content-Type: application/json" \
  -d '{
    "script_content": "...",
    "timeout_seconds": 1800,
    "execute_immediately": false
  }'
```

### Script Fails Silently

**Problem:** Script doesn't exit with error code.

**Solution:** Add `set -e` and explicit error checking:

```bash
#!/bin/bash
set -e  # Exit on any error

apt-get update || { echo "apt-get update failed"; exit 1; }
apt-get install -y git || { echo "git installation failed"; exit 1; }
```

### Permission Denied

**Problem:** Script can't write to certain directories.

**Solution:** Use appropriate directories or run with sudo:

```bash
#!/bin/bash
set -e

# Use /tmp for temporary files
cd /tmp
wget https://example.com/file.tar.gz

# Install to user directory
mkdir -p ~/.local/bin
mv tool ~/.local/bin/
```

### Package Not Found

**Problem:** Package installation fails.

**Solution:** Update package lists first:

```bash
#!/bin/bash
set -e

apt-get update
apt-get install -y package-name
```

## Database Migration

The database migration is handled automatically when you run:

```bash
cd backend
cargo run --bin migration up
```

This will:
1. Create the `init_scripts` table
2. Remove the `custom_dockerfile_path` column from `workspaces`
3. Add necessary indexes and foreign keys

**Note:** Any existing `custom_dockerfile_path` values will be lost. Make sure to convert them to init scripts before running the migration.

## Rollback

If you need to rollback the migration:

```bash
cd backend
cargo run --bin migration down
```

This will:
1. Drop the `init_scripts` table
2. Add back the `custom_dockerfile_path` column to `workspaces`

## Benefits of Init Scripts

1. **Faster Updates**: No need to rebuild Docker images
2. **Better Visibility**: Full execution logs and status tracking
3. **Easier Debugging**: View logs and re-execute scripts
4. **More Flexible**: Update scripts without affecting running containers
5. **Automatic Cleanup**: Old logs are automatically deleted after 30 days

## Support

If you encounter issues during migration:

1. Check the API documentation at `http://localhost:3000/swagger-ui`
2. Review script logs at `/api/workspaces/:id/init-script/logs`
3. Consult the main README for API examples
4. Report issues on GitHub

## Example: Complete Migration

**Before (using custom_dockerfile_path):**

```json
{
  "repository_id": 1,
  "custom_dockerfile_path": "/path/to/custom/Dockerfile",
  "image_source": "custom",
  "max_concurrent_tasks": 3,
  "cpu_limit": 2.0,
  "memory_limit": "4GB",
  "disk_limit": "10GB"
}
```

**After (using init_script):**

```json
{
  "repository_id": 1,
  "init_script": "#!/bin/bash\nset -e\napt-get update\napt-get install -y git curl python3\npip3 install requests\necho 'Setup complete'",
  "script_timeout_seconds": 600,
  "image_source": "default",
  "max_concurrent_tasks": 3,
  "cpu_limit": 2.0,
  "memory_limit": "4GB",
  "disk_limit": "10GB"
}
```

The init script approach provides the same functionality with better visibility and easier maintenance.
