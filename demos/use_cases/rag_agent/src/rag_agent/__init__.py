import click
from mcp.server.fastmcp import FastMCP

mcp = None


@click.command()
@click.option("--transport", "transport", default="stdio")
@click.option("--host", "host", default="localhost")
@click.option("--port", "port", default=10101)
@click.option("--agent", "agent", default=None)
def main(host, port, agent, transport):
    print(f"Starting agent(s): {agent if agent else 'all'}")
    global mcp
    mcp = FastMCP("RAG Agent Demo", host=host, port=port)

    if agent == "query_parser":
        import rag_agent.query_parser
    elif agent == "document_store":
        import rag_agent.document_store
    elif agent == "response_generator":
        import rag_agent.response_generator
    else:
        import rag_agent.query_parser
        import rag_agent.document_store
        import rag_agent.response_generator
    print("All agents loaded.")
    mcp.run(transport=transport)


if __name__ == "__main__":
    main()
