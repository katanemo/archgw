import json
import re
from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
from openai import AsyncOpenAI
import os
import logging
import time
import uuid
import uvicorn
from datetime import datetime, timedelta
import httpx
from typing import Optional
from urllib.parse import quote

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
WEATHER_MODEL = "openai/gpt-4o"
LOCATION_MODEL = "openai/gpt-4o-mini"

# HTTP client for API calls
http_client = httpx.AsyncClient(timeout=10.0)

# System prompt for weather agent
SYSTEM_PROMPT = """You are a professional travel planner assistant. Your role is to provide accurate, clear, and helpful information about weather and flights based on the structured data provided to you.

CRITICAL INSTRUCTIONS:

1. DATA STRUCTURE:

   WEATHER DATA:
   - You will receive weather data as JSON in a system message
   - The data contains a "location" field (string) and a "forecast" array
   - Each forecast entry has: date, day_name, temperature_c, temperature_f, temperature_max_c, temperature_min_c, condition, sunrise, sunset
   - Some fields may be null/None - handle these gracefully

   FLIGHT DATA:
   - You will receive flight information in a system message
   - Flight data includes: airline, flight number, departure time, arrival time, origin airport, destination airport, aircraft type, status, gate, terminal
   - Information may include both scheduled and estimated times
   - Some fields may be unavailable - handle these gracefully

2. WEATHER HANDLING:
   - For single-day queries: Use temperature_c/temperature_f (current/primary temperature)
   - For multi-day forecasts: Use temperature_max_c and temperature_min_c when available
   - Always provide temperatures in both Celsius and Fahrenheit when available
   - If temperature is null, say "temperature data unavailable" rather than making up numbers
   - Use exact condition descriptions provided (e.g., "Clear sky", "Rainy", "Partly Cloudy")
   - Add helpful context when appropriate (e.g., "perfect for outdoor activities" for clear skies)

3. FLIGHT HANDLING:
   - Present flight information clearly with airline name and flight number
   - Include departure and arrival times with time zones when provided
   - Mention origin and destination airports with their codes
   - Include gate and terminal information when available
   - Note aircraft type if relevant to the query
   - Highlight any status updates (delays, early arrivals, etc.)
   - For multiple flights, list them in chronological order by departure time
   - If specific details are missing, acknowledge this rather than inventing information

4. MULTI-PART QUERIES:
   - Users may ask about both weather and flights in one message
   - Answer ALL parts of the query that you have data for
   - Organize your response logically - typically weather first, then flights, or vice versa based on the query
   - Provide complete information for each topic without mentioning other agents
   - If you receive data for only one topic but the user asked about multiple, answer what you can with the provided data

5. ERROR HANDLING:
   - If weather forecast contains an "error" field, acknowledge the issue politely
   - If temperature or condition is null/None, mention that specific data is unavailable
   - If flight details are incomplete, state which information is unavailable
   - Never invent or guess weather or flight data - only use what's provided
   - If location couldn't be determined, acknowledge this but still provide available data

6. RESPONSE FORMAT:

   For Weather:
   - Single-day queries: Provide current conditions, temperature, and condition
   - Multi-day forecasts: List each day with date, day name, high/low temps, and condition
   - Include sunrise/sunset times when available and relevant

   For Flights:
   - List flights with clear numbering or bullet points
   - Include key details: airline, flight number, departure/arrival times, airports
   - Add gate, terminal, and status information when available
   - For multiple flights, organize chronologically

   General:
   - Use natural, conversational language
   - Be concise but complete
   - Format dates and times clearly
   - Use bullet points or numbered lists for clarity

7. LOCATION HANDLING:
   - Always mention location names from the data
   - For flights, clearly state origin and destination cities/airports
   - If locations differ from what the user asked, acknowledge this politely

8. RESPONSE STYLE:
   - Be friendly and professional
   - Use natural language, not technical jargon
   - Provide information in a logical, easy-to-read format
   - When answering multi-part queries, create a cohesive response that addresses all aspects

Remember: Only use the data provided. Never fabricate weather or flight information. If data is missing, clearly state what's unavailable. Answer all parts of the user's query that you have data for."""


async def geocode_city(city: str) -> Optional[dict]:
    """Geocode a city name to latitude and longitude using Open-Meteo API."""
    try:
        url = f"https://geocoding-api.open-meteo.com/v1/search?name={quote(city)}&count=1&language=en&format=json"
        response = await http_client.get(url)

        if response.status_code != 200:
            logger.warning(
                f"Geocoding API returned status {response.status_code} for city: {city}"
            )
            return None

        data = response.json()

        if not data.get("results") or len(data["results"]) == 0:
            logger.warning(f"No geocoding results found for city: {city}")
            return None

        result = data["results"][0]
        return {
            "latitude": result["latitude"],
            "longitude": result["longitude"],
            "name": result.get("name", city),
        }
    except Exception as e:
        logger.error(f"Error geocoding city {city}: {e}")
        return None


async def get_live_weather(
    latitude: float, longitude: float, days: int = 1
) -> Optional[dict]:
    """Get live weather data from Open-Meteo API."""
    try:
        forecast_days = min(days, 16)

        url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={latitude}&"
            f"longitude={longitude}&"
            f"current=temperature_2m&"
            f"hourly=temperature_2m&"
            f"daily=sunrise,sunset,temperature_2m_max,temperature_2m_min,weather_code&"
            f"forecast_days={forecast_days}&"
            f"timezone=auto"
        )

        response = await http_client.get(url)

        if response.status_code != 200:
            logger.warning(f"Weather API returned status {response.status_code}")
            return None

        return response.json()
    except Exception as e:
        logger.error(f"Error fetching weather data: {e}")
        return None


def weather_code_to_condition(weather_code: int) -> str:
    """Convert WMO weather code to human-readable condition."""
    # WMO Weather interpretation codes (WW)
    if weather_code == 0:
        return "Clear sky"
    elif weather_code in [1, 2, 3]:
        return "Partly Cloudy"
    elif weather_code in [45, 48]:
        return "Foggy"
    elif weather_code in [51, 53, 55, 56, 57]:
        return "Drizzle"
    elif weather_code in [61, 63, 65, 66, 67]:
        return "Rainy"
    elif weather_code in [71, 73, 75, 77]:
        return "Snowy"
    elif weather_code in [80, 81, 82]:
        return "Rainy"
    elif weather_code in [85, 86]:
        return "Snowy"
    elif weather_code in [95, 96, 99]:
        return "Stormy"
    else:
        return "Cloudy"


async def get_weather_data(location: str, days: int = 1):
    """Get live weather data for a location using Open-Meteo API."""
    geocode_result = await geocode_city(location)

    if not geocode_result:
        logger.warning(f"Could not geocode location: {location}, using fallback")
        geocode_result = await geocode_city("New York")
        if not geocode_result:
            return {
                "location": location,
                "forecast": [
                    {
                        "date": datetime.now().strftime("%Y-%m-%d"),
                        "day_name": datetime.now().strftime("%A"),
                        "temperature_c": None,
                        "temperature_f": None,
                        "condition": "Unknown",
                        "error": "Could not retrieve weather data",
                    }
                ],
            }

    location_name = geocode_result["name"]
    latitude = geocode_result["latitude"]
    longitude = geocode_result["longitude"]

    weather_data = await get_live_weather(latitude, longitude, days)

    if not weather_data:
        logger.warning("Could not fetch weather data for requested location")
        return {
            "location": location_name,
            "forecast": [
                {
                    "date": datetime.now().strftime("%Y-%m-%d"),
                    "day_name": datetime.now().strftime("%A"),
                    "temperature_c": None,
                    "temperature_f": None,
                    "condition": "Unknown",
                    "error": "Could not retrieve weather data",
                }
            ],
        }

    current_temp = weather_data.get("current", {}).get("temperature_2m")
    daily_data = weather_data.get("daily", {})

    forecast = []
    for i in range(min(days, len(daily_data.get("time", [])))):
        date_str = daily_data["time"][i]
        date_obj = datetime.fromisoformat(date_str.replace("Z", "+00:00"))

        temp_max = (
            daily_data.get("temperature_2m_max", [None])[i]
            if i < len(daily_data.get("temperature_2m_max", []))
            else None
        )
        temp_min = (
            daily_data.get("temperature_2m_min", [None])[i]
            if i < len(daily_data.get("temperature_2m_min", []))
            else None
        )
        weather_code = (
            daily_data.get("weather_code", [0])[i]
            if i < len(daily_data.get("weather_code", []))
            else 0
        )
        sunrise = (
            daily_data.get("sunrise", [None])[i]
            if i < len(daily_data.get("sunrise", []))
            else None
        )
        sunset = (
            daily_data.get("sunset", [None])[i]
            if i < len(daily_data.get("sunset", []))
            else None
        )

        temp_c = (
            temp_max if temp_max is not None else (current_temp if i == 0 else temp_min)
        )

        day_info = {
            "date": date_str.split("T")[0],
            "day_name": date_obj.strftime("%A"),
            "temperature_c": round(temp_c, 1) if temp_c is not None else None,
            "temperature_f": (
                round(temp_c * 9 / 5 + 32, 1) if temp_c is not None else None
            ),
            "temperature_max_c": round(temp_max, 1) if temp_max is not None else None,
            "temperature_min_c": round(temp_min, 1) if temp_min is not None else None,
            "condition": weather_code_to_condition(weather_code),
            "sunrise": sunrise.split("T")[1] if sunrise else None,
            "sunset": sunset.split("T")[1] if sunset else None,
        }
        forecast.append(day_info)

    return {"location": location_name, "forecast": forecast}


LOCATION_EXTRACTION_PROMPT = """You are a location extraction assistant for WEATHER queries. Your ONLY job is to extract the geographic location (city, state, country, etc.) that the user is asking about for WEATHER information.

CRITICAL RULES:
1. Extract ONLY the location name associated with WEATHER questions - nothing else
2. Return just the location name in plain text (e.g., "London", "New York", "Paris, France")
3. **MULTI-PART QUERY HANDLING**: If the user mentions multiple locations in a multi-part query, extract ONLY the location mentioned in the WEATHER part
   - Look for patterns like "weather in [location]", "forecast for [location]", "weather [location]"
   - The location that appears WITH "weather" keywords is the weather location
   - Example: "What's the weather in Seattle, and what is one flight that goes direct to Atlanta?" → Extract "Seattle" (appears with "weather in")
   - Example: "What is the weather in Atlanta and what flight goes from Detroit to Atlanta?" → Extract "Atlanta" (appears with "weather in", even though Atlanta also appears in flight part)
   - Example: "Weather in London and flights to Paris" → Extract "London" (weather location)
   - Example: "What flight goes from Detroit to Atlanta and what's the weather in Atlanta?" → Extract "Atlanta" (appears with "weather in")
4. Look for patterns like "weather in [location]", "forecast for [location]", "weather [location]", "temperature in [location]"
5. Ignore error messages, HTML tags, and assistant responses
6. If no clear weather-related location is found, return exactly: "NOT_FOUND"
7. Clean the location name - remove words like "about", "for", "in", "the weather in", etc.
8. Return the location in a format suitable for geocoding (city name, or "City, State", or "City, Country")

Examples:
- "What's the weather in London?" → "London"
- "Tell me about the weather for New York" → "New York"
- "Weather forecast for Paris, France" → "Paris, France"
- "What's the weather in Seattle, and what is one flight that goes direct to Atlanta?" → "Seattle" (appears with "weather in")
- "What is the weather in Atlanta and what flight goes from Detroit to Atlanta?" → "Atlanta" (appears with "weather in")
- "Weather in Istanbul and flights to Seattle" → "Istanbul" (weather location)
- "What flight goes from Detroit to Atlanta and what's the weather in Atlanta?" → "Atlanta" (appears with "weather in")
- "I'm going to Seattle" → "Seattle" (if context suggests weather query)
- "What's happening?" → "NOT_FOUND"

Now extract the WEATHER location from this message:"""


async def extract_location_from_messages(messages):
    """Extract location from user messages using LLM, focusing on weather-related locations."""
    user_messages = [msg for msg in messages if msg.role == "user"]

    if not user_messages:
        logger.warning("No user messages found, using default: New York")
        return "New York"

    # CRITICAL: Always preserve the FIRST user message (original query) for multi-agent scenarios
    # When Plano processes multiple agents, it may add assistant responses that get filtered out,
    # but we need to always use the original user query
    original_user_message = user_messages[0].content.strip() if user_messages else None

    # Try to find a valid recent user message first (for follow-up queries)
    user_content = None
    for msg in reversed(user_messages):
        content = msg.content.strip()
        content_lower = content.lower()

        # Skip messages that are clearly JSON-encoded assistant responses or errors
        # But be less aggressive - only skip if it's clearly not a user query
        if content.startswith("[{") or content.startswith("[{"):
            # Likely JSON-encoded assistant response
            continue
        if any(
            pattern in content_lower
            for pattern in [
                '"role": "assistant"',
                '"role":"assistant"',
                "error:",
            ]
        ):
            continue
        # Don't skip messages that just happen to contain these words naturally
        user_content = content
        break

    # Fallback to original user message if no valid recent message found
    if not user_content and original_user_message:
        # Check if original message is valid (not JSON-encoded)
        if not (
            original_user_message.startswith("[{")
            or original_user_message.startswith("[{")
        ):
            user_content = original_user_message
            logger.info(f"Using original user message: {user_content[:200]}")

    if not user_content:
        logger.warning("No valid user message found, using default: New York")
        return "New York"

    try:
        logger.info(
            f"Extracting weather location from user message: {user_content[:200]}"
        )

        # Build context from conversation history
        conversation_context = []
        for msg in messages:
            content = msg.content.strip()
            content_lower = content.lower()
            if any(
                pattern in content_lower
                for pattern in ["<", ">", "error:", "i apologize", "i'm having trouble"]
            ):
                continue
            conversation_context.append({"role": msg.role, "content": content})

        # Use last 5 messages for context
        context_messages = (
            conversation_context[-5:]
            if len(conversation_context) > 5
            else conversation_context
        )

        llm_messages = [{"role": "system", "content": LOCATION_EXTRACTION_PROMPT}]
        for msg in context_messages:
            llm_messages.append({"role": msg["role"], "content": msg["content"]})

        response = await archgw_client.chat.completions.create(
            model=LOCATION_MODEL,
            messages=llm_messages,
            temperature=0.1,
            max_tokens=50,
        )

        location = response.choices[0].message.content.strip()
        location = location.strip("\"'`.,!?")

        if not location or location.upper() == "NOT_FOUND":
            # Fallback: Try regex extraction for weather patterns
            weather_patterns = [
                r"weather\s+(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                r"forecast\s+(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                r"weather\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
            ]
            for msg in reversed(context_messages):
                if msg["role"] == "user":
                    content = msg["content"]
                    for pattern in weather_patterns:
                        match = re.search(pattern, content, re.IGNORECASE)
                        if match:
                            potential_location = match.group(1).strip()
                            logger.info(
                                f"Fallback regex extracted weather location: {potential_location}"
                            )
                            return potential_location

            logger.warning(
                f"LLM could not extract location from message, using default: New York"
            )
            return "New York"

        logger.info(f"LLM extracted weather location: {location}")
        return location

    except Exception as e:
        logger.error(f"Error extracting location with LLM: {e}, trying fallback regex")
        # Fallback regex extraction
        try:
            for msg in reversed(messages):
                if msg.role == "user":
                    content = msg.content
                    weather_patterns = [
                        r"weather\s+(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                        r"forecast\s+(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                    ]
                    for pattern in weather_patterns:
                        match = re.search(pattern, content, re.IGNORECASE)
                        if match:
                            potential_location = match.group(1).strip()
                            logger.info(
                                f"Fallback regex extracted weather location: {potential_location}"
                            )
                            return potential_location
        except:
            pass

        logger.error("All extraction methods failed, using default: New York")
        return "New York"


# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",
)

# FastAPI app for REST server
app = FastAPI(title="Weather Forecast Agent", version="1.0.0")


async def prepare_weather_messages(request_body: ChatCompletionRequest):
    """Prepare messages with weather data."""
    # Extract location from conversation using LLM
    location = await extract_location_from_messages(request_body.messages)

    # Determine if they want forecast (multi-day)
    last_user_msg = ""
    for msg in reversed(request_body.messages):
        if msg.role == "user":
            last_user_msg = msg.content.lower()
            break

    days = 5 if "forecast" in last_user_msg or "week" in last_user_msg else 1

    # Get live weather data
    weather_data = await get_weather_data(location, days)

    # Create system message with weather data
    weather_context = f"""
Current weather data for {weather_data['location']}:

{json.dumps(weather_data, indent=2)}

Use this data to answer the user's weather query.
"""

    response_messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "assistant", "content": weather_context},
    ]

    # Add conversation history
    for msg in request_body.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    return response_messages


@app.post("/v1/chat/completions")
async def chat_completion_http(request: Request, request_body: ChatCompletionRequest):
    """HTTP endpoint for chat completions with streaming support."""
    logger.info(f"Received weather request with {len(request_body.messages)} messages")
    logger.info(
        f"messages detail json dumps: {json.dumps([msg.model_dump() for msg in request_body.messages], indent=2)}"
    )

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
    response_messages = await prepare_weather_messages(request_body)

    try:
        logger.info(
            f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate weather response"
        )

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
