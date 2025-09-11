from pydantic import BaseModel
from . import mcp


class QueryRequest(BaseModel):
    query: str
    metadata: dict | None = None


class QueryResponse(BaseModel):
    query: str
    results: list


@mcp.tool()
def query_rag_store(request: QueryRequest):
    """Query the RAG document store."""
    return {"query": request.query, "results": []}
