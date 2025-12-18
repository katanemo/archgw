import json
from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
from openai import AsyncOpenAI
import os
import logging
import time
import uuid
import uvicorn
from datetime import datetime, timedelta
import random

from .api import (
    ChatCompletionRequest,
    ChatCompletionResponse,
    ChatCompletionStreamResponse,
)

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - [WEATHER_AGENT] - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)

# Configuration for archgw LLM gateway
LLM_GATEWAY_ENDPOINT = os.getenv("LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1")
WEATHER_MODEL = "gpt-4o-mini"

# Sample weather data
WEATHER_CONDITIONS = ["Sunny", "Partly Cloudy", "Cloudy", "Rainy", "Stormy", "Snowy"]
CITIES_DATA = {
    "new york": {"temp_base": 15, "condition_bias": "Cloudy"},
    "london": {"temp_base": 12, "condition_bias": "Rainy"},
    "tokyo": {"temp_base": 18, "condition_bias": "Partly Cloudy"},
    "paris": {"temp_base": 14, "condition_bias": "Cloudy"},
    "sydney": {"temp_base": 22, "condition_bias": "Sunny"},
    "dubai": {"temp_base": 32, "condition_bias": "Sunny"},
    "singapore": {"temp_base": 28, "condition_bias": "Rainy"},
    "san francisco": {"temp_base": 16, "condition_bias": "Partly Cloudy"},
}

# System prompt for weather agent
SYSTEM_PROMPT = """You are a helpful weather information assistant.

Your role is to provide accurate and helpful weather information based on the weather data provided.

When responding:
1. Parse the user's request to understand the location they're asking about
2. Use the weather data provided in the conversation context
3. Provide clear, concise weather information
4. Include temperature, conditions, and any relevant details
5. If asked about forecast, provide multi-day information
6. Be conversational and helpful

Format your responses in a user-friendly way."""


def get_weather_data(location: str, days: int = 1):
    """Generate mock weather data for a location."""
    location_lower = location.lower()

    # Find matching city
    city_data = None
    for city, data in CITIES_DATA.items():
        if city in location_lower or location_lower in city:
            city_data = data
            location = city.title()
            break

    if not city_data:
        # Default for unknown cities
        city_data = {"temp_base": 20, "condition_bias": "Partly Cloudy"}

    weather_info = []
    for day in range(days):
        date = datetime.now() + timedelta(days=day)
        temp_variation = random.randint(-5, 5)
        temp = city_data["temp_base"] + temp_variation

        # Bias towards the city's typical condition
        if random.random() < 0.6:
            condition = city_data["condition_bias"]
        else:
            condition = random.choice(WEATHER_CONDITIONS)

        day_info = {
            "date": date.strftime("%Y-%m-%d"),
            "day_name": date.strftime("%A"),
            "temperature_c": temp,
            "temperature_f": int(temp * 9 / 5 + 32),
            "condition": condition,
            "humidity": random.randint(40, 80),
            "wind_speed_kmh": random.randint(5, 30),
        }
        weather_info.append(day_info)

    return {"location": location, "forecast": weather_info}


def extract_location_from_messages(messages):
    """Extract location from user messages."""
    # Look through messages for location mentions
    for msg in reversed(messages):
        if msg.role == "user":
            content = msg.content.lower()
            # Check for known cities
            for city in CITIES_DATA.keys():
                if city in content:
                    return city.title()
            # Basic extraction for "in [location]" or "weather [location]"
            words = content.split()
            if "in" in words:
                idx = words.index("in")
                if idx + 1 < len(words):
                    return words[idx + 1].title()
    return "New York"  # Default location


# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",
)

# FastAPI app for REST server
app = FastAPI(title="Weather Forecast Agent", version="1.0.0")


def prepare_weather_messages(request_body: ChatCompletionRequest):
    """Prepare messages with weather data."""
    # Extract location from conversation
    location = extract_location_from_messages(request_body.messages)

    # Determine if they want forecast (multi-day)
    last_user_msg = ""
    for msg in reversed(request_body.messages):
        if msg.role == "user":
            last_user_msg = msg.content.lower()
            break

    days = 5 if "forecast" in last_user_msg or "week" in last_user_msg else 1

    # Get weather data
    weather_data = get_weather_data(location, days)

    # Create system message with weather data
    weather_context = f"""
Current weather data for {weather_data['location']}:

{json.dumps(weather_data, indent=2)}

Use this data to answer the user's weather query.
"""

    response_messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "system", "content": weather_context},
    ]

    # Add conversation history
    for msg in request_body.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    return response_messages


@app.post("/v1/chat/completions")
async def chat_completion_http(request: Request, request_body: ChatCompletionRequest):
    """HTTP endpoint for chat completions with streaming support."""
    logger.info(f"Received weather request with {len(request_body.messages)} messages")

    # Get traceparent header from HTTP request
    traceparent_header = request.headers.get("traceparent")

    if traceparent_header:
        logger.info(f"Received traceparent header: {traceparent_header}")

    return StreamingResponse(
        stream_chat_completions(request_body, traceparent_header),
        media_type="text/plain",
        headers={
            "content-type": "text/event-stream",
        },
    )


async def stream_chat_completions(
    request_body: ChatCompletionRequest, traceparent_header: str = None
):
    """Generate streaming chat completions."""
    # Prepare messages with weather data
    response_messages = prepare_weather_messages(request_body)

    try:
        logger.info(
            f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate weather response"
        )

        # Prepare extra headers
        extra_headers = {"x-envoy-max-retries": "3"}
        if traceparent_header:
            extra_headers["traceparent"] = traceparent_header

        response_stream = await archgw_client.chat.completions.create(
            model=WEATHER_MODEL,
            messages=response_messages,
            temperature=request_body.temperature or 0.7,
            max_tokens=request_body.max_tokens or 1000,
            stream=True,
            extra_headers=extra_headers,
        )

        completion_id = f"chatcmpl-{uuid.uuid4().hex[:8]}"
        created_time = int(time.time())
        collected_content = []

        async for chunk in response_stream:
            if chunk.choices and chunk.choices[0].delta.content:
                content = chunk.choices[0].delta.content
                collected_content.append(content)

                stream_chunk = ChatCompletionStreamResponse(
                    id=completion_id,
                    created=created_time,
                    model=request_body.model,
                    choices=[
                        {
                            "index": 0,
                            "delta": {"content": content},
                            "finish_reason": None,
                        }
                    ],
                )

                yield f"data: {stream_chunk.model_dump_json()}\n\n"

        # Send final chunk
        full_response = "".join(collected_content)
        updated_history = [{"role": "assistant", "content": full_response}]

        final_chunk = ChatCompletionStreamResponse(
            id=completion_id,
            created=created_time,
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop",
                    "message": {
                        "role": "assistant",
                        "content": json.dumps(updated_history),
                    },
                }
            ],
        )

        yield f"data: {final_chunk.model_dump_json()}\n\n"
        yield "data: [DONE]\n\n"

    except Exception as e:
        logger.error(f"Error generating weather response: {e}")

        error_chunk = ChatCompletionStreamResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "delta": {
                        "content": "I apologize, but I'm having trouble retrieving weather information right now. Please try again."
                    },
                    "finish_reason": "stop",
                }
            ],
        )

        yield f"data: {error_chunk.model_dump_json()}\n\n"
        yield "data: [DONE]\n\n"


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy", "agent": "weather_forecast"}


def start_server(host: str = "localhost", port: int = 10510):
    """Start the REST server."""
    uvicorn.run(
        app,
        host=host,
        port=port,
        log_config={
            "version": 1,
            "disable_existing_loggers": False,
            "formatters": {
                "default": {
                    "format": "%(asctime)s - [WEATHER_AGENT] - %(levelname)s - %(message)s",
                },
            },
            "handlers": {
                "default": {
                    "formatter": "default",
                    "class": "logging.StreamHandler",
                    "stream": "ext://sys.stdout",
                },
            },
            "root": {
                "level": "INFO",
                "handlers": ["default"],
            },
        },
    )
