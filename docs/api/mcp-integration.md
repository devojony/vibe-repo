# MCP (Model Context Protocol) Integration

## Overview

VibeRepo supports MCP (Model Context Protocol) servers to extend agent capabilities with external tools and data sources. MCP servers can be configured at two levels:

1. **Repository Level** (Priority): `{workspace_dir}/.vibe-repo/mcp-servers.json`
2. **Global Level** (Fallback): `./data/vibe-repo/config/mcp-servers.json`

## Configuration Format

### Basic Structure

```json
{
  "version": "1.0",
  "servers": [
    {
      "name": "server-name",
      "command": "command-to-execute",
      "args": ["arg1", "arg2"],
      "env": [
        {
          "name": "ENV_VAR_NAME",
          "value": "${ENV_VAR_VALUE}"
        }
      ],
      "disabled": false
    }
  ],
  "metadata": {
    "description": "Optional description",
    "updated_at": "2026-02-09T10:00:00Z"
  }
}
```

### Field Descriptions

- **version**: Configuration version (currently "1.0")
- **servers**: Array of MCP server configurations
  - **name**: Unique identifier for the server (required)
  - **command**: Executable command (required)
  - **args**: Command-line arguments (optional, default: [])
  - **env**: Environment variables (optional, default: [])
    - **name**: Variable name
    - **value**: Variable value (supports `${VAR}` placeholders)
  - **disabled**: Whether to skip this server (optional, default: false)
- **metadata**: Optional metadata for documentation

## Environment Variable Substitution

Environment variables can be referenced using `${VAR_NAME}` syntax:

```json
{
  "name": "GITHUB_TOKEN",
  "value": "${GITHUB_TOKEN}"
}
```

The system will:
1. Look up the environment variable at runtime
2. Replace the placeholder with the actual value
3. Return an error if the variable is not found

## Configuration Priority

When an agent is spawned, the system loads MCP configuration in this order:

1. **Repository-level configuration** (if exists)
   - Path: `{workspace_dir}/.vibe-repo/mcp-servers.json`
   - Use case: Project-specific tools and data sources

2. **Global configuration** (if repository config doesn't exist)
   - Path: `./data/vibe-repo/config/mcp-servers.json`
   - Use case: Organization-wide tools

3. **Default configuration** (if no config files exist)
   - Empty configuration (no MCP servers)

## Example Configurations

### Global Configuration

Create `./data/vibe-repo/config/mcp-servers.json`:

```json
{
  "version": "1.0",
  "servers": [
    {
      "name": "github",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": [
        {
          "name": "GITHUB_TOKEN",
          "value": "${GITHUB_TOKEN}"
        }
      ],
      "disabled": false
    },
    {
      "name": "filesystem",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/allowed/path"],
      "env": [],
      "disabled": false
    }
  ],
  "metadata": {
    "description": "Global MCP servers for all repositories",
    "updated_at": "2026-02-09T10:00:00Z"
  }
}
```

### Repository-Level Configuration

Create `{workspace_dir}/.vibe-repo/mcp-servers.json`:

```json
{
  "version": "1.0",
  "servers": [
    {
      "name": "postgres",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres"],
      "env": [
        {
          "name": "POSTGRES_CONNECTION_STRING",
          "value": "${DATABASE_URL}"
        }
      ],
      "disabled": false
    },
    {
      "name": "custom-api",
      "command": "/usr/local/bin/my-mcp-server",
      "args": ["--config", "./config.json"],
      "env": [
        {
          "name": "API_KEY",
          "value": "${MY_API_KEY}"
        }
      ],
      "disabled": false
    }
  ],
  "metadata": {
    "description": "Project-specific MCP servers",
    "updated_at": "2026-02-09T10:00:00Z"
  }
}
```

## Available MCP Servers

### Official MCP Servers

1. **GitHub** - `@modelcontextprotocol/server-github`
   - Access GitHub repositories, issues, PRs
   - Requires: `GITHUB_TOKEN`

2. **Filesystem** - `@modelcontextprotocol/server-filesystem`
   - Read/write files in allowed directories
   - Requires: Directory path as argument

3. **PostgreSQL** - `@modelcontextprotocol/server-postgres`
   - Query PostgreSQL databases
   - Requires: `POSTGRES_CONNECTION_STRING`

4. **Brave Search** - `@modelcontextprotocol/server-brave-search`
   - Web search capabilities
   - Requires: `BRAVE_API_KEY`

5. **Google Drive** - `@modelcontextprotocol/server-gdrive`
   - Access Google Drive files
   - Requires: Google OAuth credentials

### Custom MCP Servers

You can create custom MCP servers following the [MCP specification](https://modelcontextprotocol.io/):

```json
{
  "name": "my-custom-server",
  "command": "/path/to/my-server",
  "args": ["--port", "8080"],
  "env": [
    {
      "name": "CONFIG_PATH",
      "value": "/path/to/config"
    }
  ]
}
```

## Validation Rules

The system validates MCP configurations:

1. **Unique Names**: Each server must have a unique name
2. **Required Fields**: `name` and `command` are required
3. **Environment Variables**: All `${VAR}` placeholders must resolve
4. **Disabled Servers**: Servers with `disabled: true` are filtered out

## Error Handling

### Configuration Errors

If configuration loading fails:
- **Repository config error**: Falls back to global config
- **Global config error**: Falls back to empty config
- **Validation error**: Agent spawning fails with error message

### Runtime Errors

If environment variable substitution fails:
- Error message: `"Missing environment variable(s): VAR1, VAR2"`
- Agent spawning fails

## Best Practices

### 1. Use Environment Variables for Secrets

❌ **Bad** - Hardcoded secrets:
```json
{
  "name": "GITHUB_TOKEN",
  "value": "ghp_xxxxxxxxxxxx"
}
```

✅ **Good** - Environment variable reference:
```json
{
  "name": "GITHUB_TOKEN",
  "value": "${GITHUB_TOKEN}"
}
```

### 2. Disable Unused Servers

Instead of deleting server configurations, disable them:

```json
{
  "name": "postgres",
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-postgres"],
  "disabled": true
}
```

### 3. Document Your Configuration

Use the metadata field:

```json
{
  "metadata": {
    "description": "Production MCP servers - updated after security review",
    "updated_at": "2026-02-09T10:00:00Z",
    "owner": "devops-team"
  }
}
```

### 4. Repository-Specific Overrides

Use repository-level config to override global settings:

- **Global**: Common tools (GitHub, filesystem)
- **Repository**: Project-specific tools (database, APIs)

## Troubleshooting

### MCP Servers Not Loading

Check logs for:
```
INFO Loading repository-level MCP configuration from: ...
INFO Loaded 2 MCP server(s) from repository configuration
```

Or:
```
WARN Failed to load repository-level MCP configuration: ...
INFO Loading global MCP configuration from: ...
```

### Environment Variable Not Found

Error message:
```
ERROR Failed to load MCP configuration: Missing environment variable(s): GITHUB_TOKEN
```

Solution:
1. Set the environment variable: `export GITHUB_TOKEN=ghp_xxx`
2. Or update `.env` file
3. Restart the VibeRepo service

### Duplicate Server Names

Error message:
```
ERROR Failed to load MCP configuration: Duplicate MCP server name: github
```

Solution: Ensure all server names are unique in the configuration file.

## Security Considerations

1. **File Permissions**: Restrict access to configuration files containing sensitive data
2. **Environment Variables**: Use secure methods to manage secrets (e.g., HashiCorp Vault)
3. **Allowed Directories**: For filesystem MCP servers, specify minimal required paths
4. **Network Access**: MCP servers may make external network requests

## API Integration

MCP configuration is loaded automatically when spawning agents. No API changes are required.

### Agent Spawning Flow

```
1. Agent Manager receives spawn request
2. Load MCP configuration for workspace
   - Try repository-level config
   - Fall back to global config
   - Fall back to empty config
3. Validate and process configuration
   - Check for duplicates
   - Filter disabled servers
   - Substitute environment variables
4. Create ACP client with MCP servers
5. Initialize agent with MCP capabilities
```

## Future Enhancements

Planned features:

1. **MCP Server Management API**: CRUD operations for MCP configurations
2. **Server Health Monitoring**: Track MCP server availability
3. **Usage Analytics**: Monitor which MCP servers are used most
4. **Dynamic Loading**: Hot-reload MCP configuration without restarting agents
5. **Server Marketplace**: Discover and install community MCP servers

## References

- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [Official MCP Servers](https://github.com/modelcontextprotocol/servers)
- [Agent Client Protocol (ACP)](https://agentclientprotocol.com/)
- [VibeRepo Documentation](../README.md)
