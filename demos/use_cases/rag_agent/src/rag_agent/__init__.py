import click
from mcp.server.fastmcp import FastMCP

mcp = None


@click.command()
@click.option("--transport", "transport", default="sse", help="Transport type: stdio or sse")
@click.option("--host", "host", default="localhost", help="Host to bind MCP server to")
@click.option("--port", "port", type=int, default=10500, help="Port for MCP server")
@click.option("--agent", "agent", required=True, help="Agent name: query_rewriter, context_builder, or response_generator")
@click.option("--name", "agent_name", default=None, help="Custom MCP server name (defaults to agent type)")
def main(host, port, agent, transport, agent_name):
    """Start a RAG agent as an MCP server."""
    
    # Map friendly names to agent modules
    agent_map = {
        "query_rewriter": ("rag_agent.query_rewriter", "Query Rewriter Agent"),
        "context_builder": ("rag_agent.context_builder_agent", "Context Builder Agent"),
        "response_generator": ("rag_agent.response_generator", "Response Generator Agent"),
    }
    
    if agent not in agent_map:
        print(f"Error: Unknown agent '{agent}'")
        print(f"Available agents: {', '.join(agent_map.keys())}")
        return
    
    module_name, default_name = agent_map[agent]
    mcp_name = agent_name or default_name
    
    print(f"Starting MCP server: {mcp_name}")
    print(f"  Agent: {agent}")
    print(f"  Transport: {transport}")
    print(f"  Host: {host}")
    print(f"  Port: {port}")
    
    global mcp
    mcp = FastMCP(mcp_name, host=host, port=port)
    
    # Import the agent module to register its tools
    import importlib
    importlib.import_module(module_name)
    
    print(f"Agent '{agent}' loaded successfully")
    print(f"MCP server ready on {transport}://{host}:{port}")
    
    mcp.run(transport=transport)


if __name__ == "__main__":
    main()
