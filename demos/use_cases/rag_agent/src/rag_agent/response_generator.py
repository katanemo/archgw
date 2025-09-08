from . import mcp

@mcp.tool()
def generate_response(query, context):
    """Generate a response based on the user query and context."""
    return {"query": query, "context": context, "response": "This is a generated response."}
