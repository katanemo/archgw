# MCP Agent Description - Quick Start

## What This Feature Does

When processing agent filter chains in Brightstaff, the system can now automatically fetch tool descriptions from MCP (Model Context Protocol) endpoints. These descriptions are used by the LLM router to intelligently select the appropriate agent for handling user requests.

## Configuration

### Basic Setup

Add agents with MCP URLs to your `arch_config.yaml`:

```yaml
agents:
  - id: rag_agent
    url: mcp://host.docker.internal:10501
  
  - id: query_rewriter  
    url: mcp://host.docker.internal:10500
    tool: rewrite_query_with_archgw  # Optional: specify tool name

listeners:
  - type: agent
    port: 8001
    router: arch_agent_router
    agents:
      - id: rag_agent
        description: "RAG agent for document retrieval"  # Fallback if MCP fails
        filter_chain:
          - query_rewriter
```

### MCP URL Formats

Three formats are supported:

```yaml
# 1. Basic - uses agent id as tool name
url: mcp://localhost:10500

# 2. Tool in path
url: mcp://localhost:10500/my_tool_name

# 3. Tool as query parameter
url: mcp://localhost:10500?tool=my_tool_name
```

## How It Works

1. **Request arrives** at agent listener
2. **Agent selector** needs to choose which agent to handle request
3. **For MCP agents**, description is fetched from endpoint:
   ```
   GET http://host:port/sse/tools/list
   Accept: text/event-stream
   ```
4. **Tool description extracted** from SSE response
5. **LLM router uses descriptions** to select best agent
6. **Selected agent processes** the request through its filter chain

## Example MCP Response

Your MCP server should respond to `/sse/tools/list` with:

```
data: {"tools": [{"name": "rewrite_query_with_archgw", "description": "Rewrites user queries using LLM for better retrieval", "inputSchema": {...}}]}
```

## Fallback Behavior

If MCP endpoint fails or returns empty description:
- System logs a warning
- Falls back to `description` field from arch_config.yaml
- Processing continues normally

## Logging

Enable debug logging to see MCP interactions:

```bash
RUST_LOG=debug cargo run
```

Look for logs like:
```
Agent rag_agent is an MCP agent, fetching tool description from: mcp://host.docker.internal:10501
Fetched MCP description for agent rag_agent: Rewrites user queries...
```

## Testing

Test your MCP endpoint manually:

```bash
# Check if endpoint is accessible
curl -H "Accept: text/event-stream" http://localhost:10500/sse/tools/list

# Expected response format
data: {"tools": [{"name": "my_tool", "description": "My tool description"}]}
```

## Troubleshooting

### "Failed to fetch MCP description"
- Check if MCP server is running
- Verify URL format is correct
- Ensure `/sse/tools/list` endpoint exists
- Check network connectivity

### "MCP tool description is empty"
- Verify MCP server returns tool in response
- Check tool name matches configuration
- Ensure `description` field is populated in MCP response

### "Tool not found"
- Verify tool name in config matches MCP server
- Check if tool is listed in `/sse/tools/list` response
- Try without explicit tool name (uses agent id)

## Best Practices

1. **Always provide fallback descriptions** in arch_config.yaml
2. **Use descriptive tool names** that match your config
3. **Keep MCP servers running** before starting Brightstaff
4. **Monitor logs** for MCP fetch failures
5. **Test MCP endpoints** independently before integration

## See Also

- [MCP Agent Integration Documentation](./MCP_AGENT_INTEGRATION.md)
- [RAG Agent Demo](../demos/use_cases/rag_agent/README.md)
- [Agent Configuration Reference](../demos/use_cases/rag_agent/arch_config.yaml)
