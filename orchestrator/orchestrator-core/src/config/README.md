# Toolman Configuration Files

This directory contains configuration files for the Toolman server that provides selective tool filtering for MCP (Model Context Protocol) servers.

## Overview

The Toolman server acts as a filtering proxy between AI agents and MCP servers, allowing you to expose only specific tools from full-featured servers. This provides fine-grained security control and prevents agents from accessing tools they don't need.

## Configuration Files

### `toolman_default.json`
A minimal default configuration showing the basic structure:
- **Single Server**: Only the filesystem server
- **Basic Tools**: Read, write, and list directory operations
- **Simple Policy**: Default agent with filesystem access
- **Use Case**: Starting point or simple development setups

### `toolman_example.json`
A comprehensive example showing real-world usage:
- **8 MCP Servers**: Official servers (filesystem, GitHub, Git, Brave Search, Memory, Puppeteer, PostgreSQL, SQLite)
- **50+ Tools**: Real tool names from actual MCP server implementations
- **5 Agent Personas**: Different tool sets for various roles
- **Advanced Features**: Task-based selection, security settings, performance tuning
- **Use Case**: Production-ready configuration template

## Key Concepts

### Selective Tool Filtering
Instead of exposing all 30+ tools from the GitHub server, you can expose only the 2-3 tools your agent actually needs:

```json
{
  "exposed_tools": {
    "github": [
      "get_file",           // Only expose these 2 tools
      "create_pull_request" // out of 30+ available
    ]
  }
}
```

### Agent Personas
Different agents get different tool sets based on their role:

```json
{
  "agent_policies": {
    "claude-developer": {
      "allowed_servers": ["filesystem", "github", "git"],
      "tool_overrides": {
        "filesystem": ["read_file", "write_file", "create_directory"],
        "github": ["get_file", "create_pull_request"],
        "git": ["git_log", "git_diff"]
      }
    },
    "claude-readonly": {
      "allowed_servers": ["filesystem", "git"],
      "tool_overrides": {
        "filesystem": ["read_file", "list_directory"],
        "git": ["git_log", "git_show"]
      }
    }
  }
}
```

### Task-Based Tool Selection
Automatically add tools based on task keywords:

```json
{
  "task_based_tool_selection": {
    "patterns": {
      "api_development": {
        "keywords": ["api", "service", "backend"],
        "additional_tools": {
          "filesystem": ["create_directory"],
          "github": ["create_repository"]
        }
      }
    }
  }
}
```

## Configuration Structure

### Servers Section
Defines available MCP servers and how to run them:

```json
{
  "servers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"],
      "env": {},
      "enabled": true,
      "description": "Official filesystem MCP server"
    }
  }
}
```

### Exposed Tools Section
Defines which tools are available (subset of server's full tool list):

```json
{
  "exposed_tools": {
    "filesystem": [
      "read_file",
      "write_file",
      "list_directory"
    ]
  }
}
```

### Agent Policies Section
Defines what each agent can access:

```json
{
  "agent_policies": {
    "agent-name": {
      "allowed_servers": ["filesystem"],
      "tool_overrides": {
        "filesystem": ["read_file", "write_file"]
      }
    }
  }
}
```

## Real MCP Servers

The example configuration uses official MCP servers with their actual tool names:

| Server | Tools Available | Example Exposed Tools |
|--------|-----------------|----------------------|
| **Filesystem** | `read_file`, `write_file`, `edit_file`, `create_directory`, `list_directory`, `move_file`, `search_files`, `get_file_info`, `list_allowed_directories` | `read_file`, `write_file`, `list_directory` |
| **GitHub** | `create_repository`, `get_file`, `create_pull_request`, `create_issue`, `search_repositories`, `list_commits`, etc. | `get_file`, `create_pull_request` |
| **Git** | `git_log`, `git_diff`, `git_show`, `search_files`, `read_file`, etc. | `git_log`, `git_diff` |
| **Brave Search** | `brave_web_search`, `brave_local_search` | `brave_web_search` |
| **Memory** | `create_memories`, `search_memories`, `get_memories` | `search_memories` |
| **Puppeteer** | `puppeteer_screenshot`, `puppeteer_navigate`, `puppeteer_click`, `puppeteer_type` | `puppeteer_screenshot` |
| **PostgreSQL** | `query`, `list_tables`, `describe_table`, `list_schemas` | `query`, `list_tables` |
| **SQLite** | `query`, `list_tables`, `describe_table`, `read_query` | `query`, `describe_table` |

## Usage

### Development
Start with `toolman_default.json` for simple setups:
```bash
toolman --config /config/toolman_default.json
```

### Production
Use `toolman_example.json` as a template and customize:
1. Copy `toolman_example.json`
2. Remove servers you don't need
3. Adjust `exposed_tools` for your use case
4. Configure `agent_policies` for your agents
5. Set appropriate security settings

### TaskRun Integration
The TaskRun controller automatically generates toolman configurations based on:
- Service type (API, frontend, etc.)
- Task description keywords
- Agent requirements

## Security Features

### Dangerous Tools
Mark tools that require extra caution:
```json
{
  "security_settings": {
    "dangerous_tools": [
      "filesystem:delete_file",
      "github:delete_repository"
    ]
  }
}
```

### Confirmation Required
Tools that need explicit approval:
```json
{
  "security_settings": {
    "require_confirmation": [
      "filesystem:write_file",
      "postgresql:query"
    ]
  }
}
```

### Audit Logging
Track all tool calls for security analysis:
```json
{
  "security_settings": {
    "audit_all_calls": true,
    "log_level": "info"
  }
}
```

## Benefits

1. **Security**: Agents only see tools they need
2. **Performance**: Smaller tool lists, faster discovery
3. **Maintainability**: Single source of truth for tool definitions
4. **Flexibility**: Task-specific and agent-specific tool sets
5. **Scalability**: Easy to add new servers and tools

## Getting Started

1. **Start Simple**: Use `toolman_default.json` for initial setup
2. **Add Servers**: Enable additional servers as needed from the example
3. **Customize Tools**: Adjust `exposed_tools` based on your requirements
4. **Create Policies**: Define agent-specific policies
5. **Add Security**: Configure security settings for production

For more details, see the comprehensive documentation in `docs/toolman-selective-filtering.md`.