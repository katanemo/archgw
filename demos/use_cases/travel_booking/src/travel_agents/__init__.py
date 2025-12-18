import click


@click.command()
@click.option("--host", "host", default="localhost", help="Host to bind server to")
@click.option("--port", "port", type=int, default=8000, help="Port for server")
@click.option(
    "--agent",
    "agent",
    required=True,
    help="Agent name: weather, flight, or hotel",
)
def main(host, port, agent):
    """Start a travel agent REST server."""

    # Map friendly names to agent modules
    agent_map = {
        "weather": ("travel_agents.weather_agent", 10510),
        "flight": ("travel_agents.flight_agent", 10520),
        "hotel": ("travel_agents.hotel_agent", 10530),
    }

    if agent not in agent_map:
        print(f"Error: Unknown agent '{agent}'")
        print(f"Available agents: {', '.join(agent_map.keys())}")
        return

    module_name, default_port = agent_map[agent]

    # Use default port if not specified
    if port == 8000:
        port = default_port

    print(f"Starting {agent} agent REST server on {host}:{port}")

    # Import the agent module and start server
    if agent == "weather":
        from travel_agents.weather_agent import start_server

        start_server(host=host, port=port)
    elif agent == "flight":
        from travel_agents.flight_agent import start_server

        start_server(host=host, port=port)
    elif agent == "hotel":
        from travel_agents.hotel_agent import start_server

        start_server(host=host, port=port)


if __name__ == "__main__":
    main()
