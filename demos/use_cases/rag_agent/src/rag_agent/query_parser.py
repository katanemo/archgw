from pydantic import BaseModel
from . import mcp

class Response(BaseModel):
    query: str
    metadata: dict

@mcp.tool()
def parse_query(query):
    """Parse the user query and returns metadata extracted from query."""
    return Response(query=query, metadata={
      "is_valid": True
    })
