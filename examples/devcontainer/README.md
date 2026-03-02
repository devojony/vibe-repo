# DevContainer Configuration Examples

This directory contains example `devcontainer.json` configurations for common project types.

## Available Examples

### 1. Minimal Configuration
**File:** `minimal.json`

The simplest possible configuration using Ubuntu base image.

```bash
cp minimal.json .devcontainer/devcontainer.json
```

### 2. Node.js Project
**File:** `nodejs.json`

Configuration for Node.js 20 projects with npm support.

**Features:**
- Node.js 20 LTS
- Git
- Automatic `npm install` on container creation
- Port 3000 forwarded

```bash
cp nodejs.json .devcontainer/devcontainer.json
```

### 3. Python Project
**File:** `python.json`

Configuration for Python 3.11 projects.

**Features:**
- Python 3.11
- Git
- Automatic `pip install -r requirements.txt`
- Python environment variables

```bash
cp python.json .devcontainer/devcontainer.json
```

### 4. Rust Project
**File:** `rust.json`

Configuration for Rust projects.

**Features:**
- Latest Rust toolchain
- Git
- Automatic `cargo build` on container creation
- Rust backtrace enabled

```bash
cp rust.json .devcontainer/devcontainer.json
```

### 5. Full-Stack Project
**File:** `fullstack.json`

Configuration for full-stack projects with Node.js, Python, and Docker.

**Features:**
- Node.js 20
- Python 3.11
- Docker-in-Docker
- Git
- Automatic dependency installation
- Multiple ports forwarded (3000, 8000)

```bash
cp fullstack.json .devcontainer/devcontainer.json
```

## Usage

1. Choose an example that matches your project type
2. Copy it to your repository's `.devcontainer/` directory
3. Rename it to `devcontainer.json`
4. Customize as needed
5. Add your repository to VibeRepo

```bash
# Example: Node.js project
mkdir -p .devcontainer
cp examples/devcontainer/nodejs.json .devcontainer/devcontainer.json

# Customize if needed
vim .devcontainer/devcontainer.json

# Commit to your repository
git add .devcontainer/
git commit -m "Add devcontainer configuration"
git push
```

## Customization Tips

### Change Node.js Version

```json
{
  "features": {
    "ghcr.io/devcontainers/features/node:1": {
      "version": "18"  // Change to 18, 20, or "lts"
    }
  }
}
```

### Add More Features

Browse available features at [containers.dev/features](https://containers.dev/features)

```json
{
  "features": {
    "ghcr.io/devcontainers/features/node:1": {},
    "ghcr.io/devcontainers/features/docker-in-docker:2": {},
    "ghcr.io/devcontainers/features/github-cli:1": {}
  }
}
```

### Add Environment Variables

```json
{
  "containerEnv": {
    "API_URL": "https://api.example.com",
    "DEBUG": "true"
  }
}
```

### Run Commands on Container Creation

```json
{
  "postCreateCommand": "npm install && npm run build"
}
```

## Resources

- **[DevContainer Guide](../../docs/api/devcontainer-guide.md)** - Complete guide
- **[Dev Containers Specification](https://containers.dev/)** - Official docs
- **[Dev Container Features](https://containers.dev/features)** - Feature catalog

## Need Help?

- Check the [Troubleshooting Guide](../../docs/api/troubleshooting.md)
- See the [DevContainer Guide](../../docs/api/devcontainer-guide.md) for detailed documentation
- Open an issue on GitHub
