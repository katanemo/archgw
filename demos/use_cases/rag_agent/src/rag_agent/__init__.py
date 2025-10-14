import click
from mcp.server.fastmcp import FastMCP

mcp = None


@click.command()
@click.option("--transport", "transport", default="stdio")
@click.option("--host", "host", default="localhost")
@click.option("--port", "port", default=10101)
@click.option("--agent", "agent", default=None)
@click.option(
    "--rest-server",
    "rest_server",
    is_flag=True,
    help="Start REST server instead of MCP server",
)
@click.option("--rest-port", "rest_port", default=8000, help="Port for REST server")
def main(host, port, agent, transport, rest_server, rest_port):
    if rest_server:
        print(f"Starting REST server on {host}:{rest_port} for agent: {agent}")

        if agent == "query_parser":
            from rag_agent.query_rewriter_agent import start_server

            start_server(host=host, port=rest_port)
            return
        elif agent == "context_builder":
            from rag_agent.context_builder_agent import (
                start_server,
            )

            start_server(host=host, port=rest_port)
            return
        elif agent == "response_generator":
            from rag_agent.response_generator_agent import start_server

            start_server(host=host, port=rest_port)
            return
        else:
            print("Please specify an agent to start with --agent option.")
            return

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
