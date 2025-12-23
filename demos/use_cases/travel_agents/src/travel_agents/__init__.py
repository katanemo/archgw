import click


@click.command()
@click.option("--host", "host", default="localhost", help="Host to bind server to")
@click.option("--port", "port", type=int, default=8000, help="Port for server")
@click.option(
    "--agent",
    "agent",
    required=True,
    help="Agent name: weather, flight, or currency",
)
def main(host, port, agent):
    """Start a travel agent REST server."""
    agent_map = {
        "weather": ("travel_agents.weather_agent", 10510),
        "flight": ("travel_agents.flight_agent", 10520),
        "currency": ("travel_agents.currency_agent", 10530),
    }

    if agent not in agent_map:
        print(f"Error: Unknown agent '{agent}'")
        print(f"Available agents: {', '.join(agent_map.keys())}")
        return

    module_name, default_port = agent_map[agent]

    if port == 8000:
        port = default_port

    print(f"Starting {agent} agent REST server on {host}:{port}")

    if agent == "weather":
        from travel_agents.weather_agent import start_server

        start_server(host=host, port=port)
    elif agent == "flight":
        from travel_agents.flight_agent import start_server

        start_server(host=host, port=port)
    elif agent == "currency":
        from travel_agents.currency_agent import start_server

        start_server(host=host, port=port)


if __name__ == "__main__":
    main()
