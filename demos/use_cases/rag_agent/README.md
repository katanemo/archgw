# RAG Agent with MCP Protocol

A multi-agent RAG system using the Model Context Protocol (MCP) for agent communication.

## Architecture

This demo consists of three MCP agents:
1. **Query Rewriter** - Rewrites user queries for better retrieval
2. **Context Builder** - Retrieves relevant context from knowledge base
3. **Response Generator** - Generates final responses with context

Each agent runs as an independent MCP server and exposes tools that can be called via the MCP protocol.

## MCP Tools

### Query Rewriter Agent
- **Tool**: `rewrite_query_with_archgw`
- **Description**: Rewrites user queries using LLM for better retrieval
- **Port**: 10500

### Context Builder Agent
- **Tool**: `chat_completions`
- **Description**: Augments queries with relevant context from knowledge base
- **Port**: 10501

### Response Generator Agent
- **Port**: 10502

## Setup and Running

### 1. Start archgw
```bash
archgw up --foreground
```

### 2. Start Individual Agents

**Query Rewriter:**
```bash
uv run python -m rag_agent \
  --agent query_rewriter \
  --host 0.0.0.0 \
  --port 10500 \
  --transport sse
```

**Context Builder:**
```bash
uv run python -m rag_agent \
  --agent context_builder \
  --host 0.0.0.0 \
  --port 10501 \
  --transport sse
```

**Response Generator:**
```bash
uv run python -m rag_agent \
  --agent response_generator \
  --host 0.0.0.0 \
  --port 10502 \
  --transport sse
```

### 3. Start All Agents at Once
```bash
./start_agents.sh
```

## Configuration

The `arch_config.yaml` defines how agents are connected:

```yaml
agent_filters:
  - id: query_rewriter
    url: mcp://host.docker.internal:10500
    tool: rewrite_query_with_archgw  # MCP tool name

  - id: context_builder
    url: mcp://host.docker.internal:10501
    tool: chat_completions
```

### MCP Tool Invocation Patterns

The config supports different ways to specify MCP tools:

**1. Separate tool field (recommended):**
```yaml
- id: query_rewriter
  url: mcp://host.docker.internal:10500
  tool: rewrite_query_with_archgw
```

**2. Tool in URL path:**
```yaml
- id: query_rewriter
  url: mcp://host.docker.internal:10500/rewrite_query_with_archgw
```

**3. Tool as query parameter:**
```yaml
- id: query_rewriter
  url: mcp://host.docker.internal:10500?tool=rewrite_query_with_archgw
```

## CLI Options

```bash
uv run python -m rag_agent --help

Options:
  --transport TEXT   Transport type: stdio or sse (default: sse)
  --host TEXT       Host to bind MCP server to (default: localhost)
  --port INTEGER    Port for MCP server (default: 10500)
  --agent TEXT      Agent name: query_rewriter, context_builder, or response_generator (required)
  --name TEXT       Custom MCP server name (optional)
```

## Environment Variables

```bash
# archgw LLM Gateway base URL (default: http://localhost:12000/v1)
export LLM_GATEWAY_ENDPOINT="http://localhost:12000/v1"

# OpenAI API Key for model providers
export OPENAI_API_KEY="your-key-here"
```

## Testing

See `sample_queries.md` for example queries to test the RAG system.

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
