# Travel Booking Demo

A multi-agent travel booking system demonstrating archgw's agent router with specialized agents for weather, flights, and hotels.

## Architecture

This demo consists of three intelligent agents:

1. **Weather Agent** (REST) - Provides current weather and forecasts for destinations worldwide
2. **Flight Agent** (REST) - Searches and books flights between cities with pricing and availability
3. **Hotel Agent** (REST) - Searches and reserves hotel rooms with preferences and pricing

All agents use archgw's agent router to intelligently route user requests to the appropriate specialized agent.

## Components

### Weather Forecast Agent
- **Port**: 10510
- **Endpoint**: `/v1/chat/completions`
- Provides weather information and forecasts for any location
- Returns temperature, conditions, humidity, and wind speed
- Supports multi-day forecasts

### Flight Booking Agent
- **Port**: 10520
- **Endpoint**: `/v1/chat/completions`
- Searches for flights between cities
- Returns flight options with airlines, times, prices, and durations
- Supports booking confirmations

### Hotel Reservation Agent
- **Port**: 10530
- **Endpoint**: `/v1/chat/completions`
- Searches for hotels in any city
- Returns hotel options with ratings, amenities, prices, and locations
- Supports reservation confirmations

## Quick Start

### Prerequisites
- Python 3.10 or higher
- UV package manager (recommended) or pip
- OpenAI API key
- archgw installed and configured

### 1. Set up environment
```bash
# Copy and edit the .env file with your OpenAI API key
cp .env.example .env
# Edit .env and add your OPENAI_API_KEY
```

### 2. Install dependencies
```bash
# Using UV (recommended)
uv sync

# Or using pip
pip install -e .
```

### 3. Start all agents
```bash
chmod +x start_agents.sh
./start_agents.sh
```

This starts:
- Weather Agent on port 10510
- Flight Agent on port 10520
- Hotel Agent on port 10530

### 4. Start archgw
In a new terminal:
```bash
cd /path/to/travel_booking
archgw up --foreground
```

### 5. Test the system

#### Weather Query
```bash
curl -X POST http://localhost:8001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "What is the weather like in Paris?"}
    ]
  }'
```

#### Flight Search
```bash
curl -X POST http://localhost:8001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "Find me flights from New York to London next week"}
    ]
  }'
```

#### Hotel Search
```bash
curl -X POST http://localhost:8001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [
      {"role": "user", "content": "I need a hotel in Tokyo for 3 nights"}
    ]
  }'
```

### 6. Use with Open WebUI (Optional)

Start the docker compose services:
```bash
docker-compose up -d
```

Then open http://localhost:8080 in your browser. The Open WebUI is pre-configured to use the archgw endpoint at http://host.docker.internal:8001/v1.

## Example Conversations

### Multi-Agent Conversation
The system can handle complex travel planning that involves multiple agents:

```
User: I'm planning a trip to Tokyo next month. What's the weather like?
Assistant: [Weather Agent provides Tokyo weather forecast]

User: Great! Can you find me flights from San Francisco to Tokyo?
Assistant: [Flight Agent shows flight options]

User: I'll take the United flight. Now I need a hotel near the city center.
Assistant: [Hotel Agent shows hotel options in Tokyo]
```

The archgw agent router automatically routes each request to the appropriate agent based on the content.

## Agent Capabilities

### Weather Agent
- Current weather conditions
- 5-day forecasts
- Temperature (Celsius and Fahrenheit)
- Humidity and wind speed
- Weather conditions (sunny, cloudy, rainy, etc.)

### Flight Agent
- Flight search between any two cities
- Multiple airline options
- Flight times and durations
- Pricing information
- Direct and connecting flights
- Seat availability
- Booking confirmations

### Hotel Agent
- Hotel search by city
- Check-in/check-out date support
- Hotel ratings and reviews
- Amenities listing
- Distance from city center
- Pricing per night and total
- Room availability
- Reservation confirmations

## Architecture Details

### Agent Routing
archgw's agent router analyzes incoming requests and automatically routes them to the most appropriate agent based on:
- Request content and intent
- Agent descriptions in arch_config.yaml
- Conversation context

### Request Flow
1. User sends a request to archgw (port 8001)
2. archgw's agent router analyzes the request
3. Router selects the appropriate agent (weather, flight, or hotel)
4. Agent processes the request using archgw's LLM gateway
5. Response streams back to the user

### Tracing
The demo includes Jaeger for distributed tracing:
- View traces at http://localhost:16686
- Trace sampling set to 100% for demo purposes
- Track requests across archgw and agents

## Development

### Running Individual Agents
You can start agents individually for development:

```bash
# Weather agent
uv run python -m travel_agents --agent weather --port 10510

# Flight agent
uv run python -m travel_agents --agent flight --port 10520

# Hotel agent
uv run python -m travel_agents --agent hotel --port 10530
```

### Project Structure
```
travel_booking/
├── arch_config.yaml          # archgw configuration
├── docker-compose.yaml       # Optional services (Jaeger, Open WebUI)
├── pyproject.toml           # Python dependencies
├── start_agents.sh          # Start all agents script
├── .env                     # Environment variables
└── src/
    └── travel_agents/
        ├── __init__.py      # CLI entry point
        ├── __main__.py      # Module runner
        ├── api.py           # Shared API models
        ├── weather_agent.py # Weather forecast agent
        ├── flight_agent.py  # Flight booking agent
        └── hotel_agent.py   # Hotel reservation agent
```

## Configuration

### arch_config.yaml
The configuration defines:
- Three agents with their URLs and descriptions
- Model providers (OpenAI)
- Model aliases for easy reference
- Agent router on port 8001
- Tracing configuration

### Environment Variables
- `OPENAI_API_KEY`: Your OpenAI API key (required)
- `LLM_GATEWAY_ENDPOINT`: archgw LLM gateway URL (default: http://localhost:12000/v1)

## Troubleshooting

### Agents won't start
- Ensure Python 3.10+ is installed
- Check that UV is installed: `pip install uv`
- Verify ports 10510, 10520, 10530 are available

### archgw won't start
- Make sure you're in the travel_booking directory
- Check that OPENAI_API_KEY is set in .env
- Verify archgw is installed: `archgw --version`

### No response from agents
- Check that all agents are running (check start_agents.sh output)
- Verify archgw is running on port 8001
- Check logs for errors

### Wrong agent responds
- The agent router uses LLM-based routing
- If routing is incorrect, try being more explicit in your request
- Check agent descriptions in arch_config.yaml

## Notes

- This is a demo with mock data - flights and hotels are simulated
- Real implementations would integrate with actual APIs (Amadeus, Booking.com, etc.)
- Weather data is generated randomly based on typical patterns for each city
- All agents use streaming responses for better user experience

## License

This demo is part of the archgw project.
