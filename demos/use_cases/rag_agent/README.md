# RAG Agent Query Parser

A FastAPI service that rewrites user queries using archgw and gpt-4o-mini for better retrieval accuracy.

## How it Works

1. Receives a chat completion request with conversation history
2. Calls archgw's LLM gateway with gpt-4o-mini to rewrite the last user query
3. Returns the rewritten query as the assistant response

## Setup and Running

1. **Start archgw**:
   ```bash
   archgw up --foreground
   ```

2. **Start the query parser service**:
   ```bash
   uv run python -m rag_agent.query_parser
   ```

## Configuration

```bash
# archgw LLM Gateway base URL (default: http://localhost:12000/v1)
export LLM_GATEWAY_ENDPOINT="http://localhost:12000/v1"

```
