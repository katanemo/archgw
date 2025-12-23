# Travel Booking Agent Demo

A production-ready multi-agent travel booking system demonstrating Plano's intelligent agent routing. This demo showcases three specialized agents working together to help users plan trips with weather information, flight searches, and currency exchange rates.

## Overview

This demo consists of three intelligent agents that work together seamlessly:

- **Weather Agent** - Real-time weather conditions and forecasts for any city worldwide
- **Flight Agent** - Live flight information between airports with real-time tracking
- **Currency Agent** - Real-time currency exchange rates and conversions

All agents use Plano's agent router to intelligently route user requests to the appropriate specialized agent based on conversation context and user intent.

## Features

- **Intelligent Routing**: Plano automatically routes requests to the right agent
- **Conversation Context**: Agents understand follow-up questions and references
- **Real-Time Data**: Live weather, flight, and currency data from public APIs
- **LLM-Powered**: Uses GPT-4o-mini for extraction and GPT-4o for responses
- **Streaming Responses**: Real-time streaming for better user experience

## Prerequisites

- Python 3.10 or higher
- [UV package manager](https://github.com/astral-sh/uv) (recommended) or pip
- OpenAI API key
- [Plano CLI](https://docs.planoai.dev) installed

## Quick Start

### 1. Install Dependencies

```bash
# Using UV (recommended)
uv sync

# Or using pip
pip install -e .
```

### 2. Set Environment Variables

Create a `.env` file or export environment variables:

```bash
export OPENAI_API_KEY="your-openai-api-key"
export AEROAPI_KEY="your-flightaware-api-key"  # Optional, demo key included
```

### 3. Start All Agents

```bash
chmod +x start_agents.sh
./start_agents.sh
```

This starts:
- Weather Agent on port 10510
- Flight Agent on port 10520
- Currency Agent on port 10530

### 4. Start Plano Orchestrator

In a new terminal:

```bash
cd /path/to/travel_booking
plano up arch_config.yaml
```

The gateway will start on port 8001 and route requests to the appropriate agents.

### 5. Test the System

Send requests to Plano Orchestrator:

```bash
curl -X POST http://localhost:8001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {"role": "user", "content": "What is the weather like in Paris?"}
    ]
  }'
```

## Example Conversations

### Weather Query
```
User: What's the weather in Istanbul?
Assistant: [Weather Agent provides current conditions and forecast]
```

### Flight Search
```
User: What flights go from London to Seattle?
Assistant: [Flight Agent shows available flights with schedules and status]
```

### Currency Exchange
```
User: What's the exchange rate for Turkish Lira to USD?
Assistant: [Currency Agent provides current exchange rate]
```

### Multi-Agent Conversation
```
User: What's the weather in Istanbul?
Assistant: [Weather information]

User: What's their exchange rate?
Assistant: [Currency rate for Turkey]

User: Do they fly out from Seattle?
Assistant: [Flight information from Istanbul to Seattle]
```

The system understands context and pronouns, automatically routing to the right agent.

### Multi-Intent Queries
```
User: What's the weather in Seattle, and do any flights go direct to New York?
Assistant: [Both weather_agent and flight_agent respond simultaneously]
  - Weather Agent: [Weather information for Seattle]
  - Flight Agent: [Flight information from Seattle to New York]
```

The orchestrator can select multiple agents simultaneously for queries containing multiple intents.

## Agent Details

### Weather Agent
- **Port**: 10510
- **API**: Open-Meteo (free, no API key)
- **Capabilities**: Current weather, multi-day forecasts, temperature, conditions, sunrise/sunset

### Flight Agent
- **Port**: 10520
- **API**: FlightAware AeroAPI
- **Capabilities**: Real-time flight status, schedules, delays, gates, terminals, live tracking

### Currency Agent
- **Port**: 10530
- **API**: Frankfurter (free, no API key)
- **Capabilities**: Exchange rates, currency conversions, historical rates

## Architecture

```
User Request → Plano Gateway (port 8001)
                ↓
         Agent Router (LLM-based)
                ↓
    ┌───────────┼───────────┐
    ↓           ↓           ↓
Weather      Flight     Currency
Agent        Agent       Agent
(10510)      (10520)     (10530)
```

Each agent:
1. Extracts intent using GPT-4o-mini
2. Fetches real-time data from APIs
3. Generates response using GPT-4o
4. Streams response back to user

## Configuration

### plano_config.yaml

Defines the three agents, their descriptions, and routing configuration. The agent router uses these descriptions to intelligently route requests.

### Environment Variables

- `OPENAI_API_KEY` - Required for LLM operations
- `AEROAPI_KEY` - Optional, FlightAware API key (demo key included)
- `LLM_GATEWAY_ENDPOINT` - Plano LLM gateway URL (default: http://localhost:12000/v1)

## Project Structure

```
travel_booking/
├── arch_config.yaml          # Plano configuration
├── start_agents.sh          # Start all agents script
├── pyproject.toml           # Python dependencies
└── src/
    └── travel_agents/
        ├── __init__.py      # CLI entry point
        ├── api.py           # Shared API models
        ├── weather_agent.py # Weather forecast agent
        ├── flight_agent.py  # Flight information agent
        └── currency_agent.py # Currency exchange agent
```

## Troubleshooting

**Agents won't start**
- Ensure Python 3.10+ is installed
- Check that UV is installed: `pip install uv`
- Verify ports 10510, 10520, 10530 are available

**Plano won't start**
- Verify Plano is installed: `plano --version`
- Check that `OPENAI_API_KEY` is set
- Ensure you're in the travel_booking directory

**No response from agents**
- Verify all agents are running (check start_agents.sh output)
- Check that Plano is running on port 8001
- Review agent logs for errors

## API Endpoints

All agents expose OpenAI-compatible chat completion endpoints:

- `POST /v1/chat/completions` - Chat completion endpoint
- `GET /health` - Health check endpoint
