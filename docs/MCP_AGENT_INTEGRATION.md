# MCP Agent Description Integration

## Overview

This implementation adds support for fetching agent tool descriptions from MCP (Model Context Protocol) endpoints during agent selection. This allows the system to use the actual tool descriptions from MCP servers for intelligent agent routing instead of relying solely on static descriptions in the configuration file.

## Architecture

### Components Modified

1. **MCP Client Module** (`brightstaff/src/utils/mcp_client.rs`)
   - New module that handles communication with MCP servers via SSE
   - Fetches tool lists and descriptions from MCP endpoints
   - Parses MCP URLs in multiple formats

2. **Configuration Structs** (`common/src/configuration.rs`)
   - Added optional `tool` field to `Agent` struct
   - Added optional `tool` field to `AgentFilter` struct
   - Supports specifying which MCP tool to invoke

3. **Agent Selector** (`brightstaff/src/handlers/agent_selector.rs`)
   - Enhanced to fetch tool descriptions from MCP endpoints
   - Uses MCP descriptions for agent routing when available
   - Falls back to configuration descriptions if MCP fetch fails

4. **Agent Chat Completions** (`brightstaff/src/handlers/agent_chat_completions.rs`)
   - Updated to pass agent_map to agent selector
   - Ensures agent information is available during selection

## How It Works

### 1. Configuration

Agents can now be configured with MCP URLs and optional tool names:

```yaml
agents:
  - id: rag_agent
    url: mcp://host.docker.internal:10501
    tool: invoke  # Optional: defaults to agent id
  
  - id: travel_agent
    url: mcp://host.docker.internal:10502

agent_filters:
  - id: query_rewriter
    url: mcp://host.docker.internal:10500
    tool: rewrite_query_with_archgw  # Optional
```

### 2. MCP URL Parsing

The MCP client supports three URL formats:

```
mcp://host:port                      # Basic format
mcp://host:port/tool_name           # Tool in path
mcp://host:port?tool=tool_name      # Tool as query param
```

### 3. Description Fetching

During agent selection:

1. Agent selector checks if agent URL starts with `mcp://`
2. If yes, calls MCP client to fetch tool description from endpoint
3. MCP client makes GET request to `http://host:port/sse/tools/list`
4. Parses SSE response to extract tool descriptions
5. Returns description for specified tool (from config or URL)
6. Falls back to config description if MCP fetch fails

### 4. Agent Routing

The fetched MCP tool descriptions are used in routing preferences:

```rust
ModelUsagePreference {
    model: agent_id,
    routing_preferences: vec![RoutingPreference {
        name: agent_id,
        description: mcp_description,  // From MCP endpoint
    }],
}
```

The LLM router uses these descriptions to select the appropriate agent based on the user's request.

## Key Features

1. **Automatic Description Fetching**: Tool descriptions are automatically fetched from MCP servers when agents have `mcp://` URLs

2. **Graceful Fallback**: If MCP endpoint is unavailable or doesn't return a description, falls back to the description in arch_config.yaml

3. **Multiple URL Formats**: Supports flexible MCP URL specification for tool names

4. **Async Operation**: All MCP fetching is done asynchronously to avoid blocking

5. **Comprehensive Logging**: Debug and warning logs track MCP interactions

## Example Usage

### Configuration (arch_config.yaml)

```yaml
version: v0.3.0

agents:
  - id: rag_agent
    url: mcp://host.docker.internal:10501
    # Description will be fetched from MCP endpoint

agent_filters:
  - id: query_rewriter
    url: mcp://host.docker.internal:10500
    tool: rewrite_query_with_archgw

listeners:
  - type: agent
    port: 8001
    router: arch_agent_router
    agents:
      - id: rag_agent
        description: fallback description if MCP unavailable
        filter_chain:
          - query_rewriter
```

### Flow

1. User sends request to listener
2. Agent selector creates agent map from configuration
3. For each agent in listener:
   - Checks if agent URL is MCP (`mcp://`)
   - Fetches tool description from MCP endpoint
   - Uses fetched description for routing
4. LLM router selects best agent based on descriptions
5. Filter chain processes request using selected agent

## Testing

Unit tests cover:
- MCP URL parsing (basic, path, query param formats)
- Agent selection with MCP descriptions
- Fallback to config descriptions
- Agent map creation

Integration tests verify:
- End-to-end agent selection flow
- Pipeline processing with MCP agents

## Error Handling

The implementation handles:
- Invalid MCP URLs (returns error)
- Unreachable MCP endpoints (logs warning, uses config description)
- Missing tool in MCP response (returns error)
- Empty descriptions (logs warning, uses config description)
- Network errors (falls back to config description)

## Backward Compatibility

The changes are fully backward compatible:
- `tool` field is optional on Agent and AgentFilter
- Non-MCP agents work as before
- Config descriptions still work when MCP is not used

## Future Enhancements

Possible improvements:
1. Cache MCP tool descriptions to reduce network calls
2. Support for MCP stdio transport in addition to SSE
3. Periodic refresh of MCP descriptions
4. Support for fetching tool schemas along with descriptions
5. Metrics for MCP endpoint availability and response times
