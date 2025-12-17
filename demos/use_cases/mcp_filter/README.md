# RAG Agent Demo

A multi-agent RAG system demonstrating archgw's agent filter chain with MCP protocol.

## Architecture

This demo consists of three components:
1. **Query Rewriter** (MCP filter) - Rewrites user queries for better retrieval
2. **Context Builder** (MCP filter) - Retrieves relevant context from knowledge base
3. **RAG Agent** (REST) - Generates final responses based on augmented context

## Components

### Query Rewriter Filter (MCP)
- **Port**: 10501
- **Tool**: `query_rewriter`
- Improves queries using LLM before retrieval

### Context Builder Filter (MCP)
- **Port**: 10502
- **Tool**: `context_builder`
- Augments queries with relevant passages from knowledge base

### RAG Agent (REST/OpenAI)
- **Port**: 10505
- **Endpoint**: `/v1/chat/completions`
- Generates responses using OpenAI-compatible API

## Quick Start

### 1. Start all agents
```bash
./start_agents.sh
```

This starts:
- Query Rewriter MCP server on port 10501
- Context Builder MCP server on port 10502
- RAG Agent REST server on port 10505

### 2. Start archgw
```bash
archgw up --foreground
```

### 3. Test the system
```bash
curl -X POST http://localhost:8001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "What is the guaranteed uptime for TechCorp?"}]
  }'
```

## Configuration

The `arch_config.yaml` defines how agents are connected:

```yaml
filters:
  - id: query_rewriter
    url: mcp://host.docker.internal:10500
    tool: rewrite_query_with_archgw  # MCP tool name

  - id: context_builder
    url: mcp://host.docker.internal:10501
    tool: chat_completions
```
How It Works

1. User sends request to archgw listener on port 8001
2. Request passes through MCP filter chain:
   - **Query Rewriter** rewrites the query for better retrieval
   - **Context Builder** augments query with relevant knowledge base passages
3. Augmented request is forwarded to **RAG Agent** REST endpoint
4. RAG Agent generates final response using LLM

## Configuration

See `arch_config.yaml` for the complete filter chain setup. The MCP filters use default settings:
- `type: mcp` (default)
- `transport: streamable-http` (default)
- Tool name defaults to filter ID `sample_queries.md` for example queries to test the RAG system.

Example request:
```bash
curl -X POST http://localhost:8001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": "What is the guaranteed uptime for TechCorp?"
      }
    ]
  }'
```
- `LLM_GATEWAY_ENDPOINT` - archgw endpoint (default: `http://localhost:12000/v1`)
- `OPENAI_API_KEY` - OpenAI API key for model providers

## Additional Resources

- See `sample_queries.md` for more example queries
- See `arch_config.yaml` for complete configuration details
