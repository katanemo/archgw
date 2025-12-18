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
    format="%(asctime)s - [FLIGHT_AGENT] - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)

# Configuration for archgw LLM gateway
LLM_GATEWAY_ENDPOINT = os.getenv("LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1")
FLIGHT_MODEL = "gpt-4o-mini"

# Sample flight data
AIRLINES = [
    "United Airlines",
    "Delta",
    "American Airlines",
    "British Airways",
    "Emirates",
    "Lufthansa",
    "Air France",
    "Singapore Airlines",
]
AIRCRAFT_TYPES = [
    "Boeing 737",
    "Airbus A320",
    "Boeing 777",
    "Airbus A350",
    "Boeing 787",
]

CITIES = [
    "New York",
    "London",
    "Tokyo",
    "Paris",
    "Sydney",
    "Dubai",
    "Singapore",
    "San Francisco",
    "Los Angeles",
    "Chicago",
    "Miami",
    "Seattle",
    "Boston",
    "Hong Kong",
    "Bangkok",
    "Rome",
]

# System prompt for flight agent
SYSTEM_PROMPT = """You are a helpful flight booking assistant.

Your role is to help users search for and book flights based on their travel needs.

When responding:
1. Parse the user's request to understand departure city, destination, dates, and preferences
2. Use the flight search results provided in the conversation context
3. Present flight options clearly with key details (airline, times, price, duration)
4. Help users compare options and make informed decisions
5. If they want to book, confirm their selection and provide booking confirmation
6. Ask clarifying questions if needed information is missing

Be professional, helpful, and make the booking process smooth and easy."""


def generate_flight_data(
    origin: str, destination: str, date: str = None, num_results: int = 5
):
    """Generate mock flight search results."""
    if not date:
        date = (datetime.now() + timedelta(days=7)).strftime("%Y-%m-%d")

    flights = []
    for i in range(num_results):
        # Generate departure and arrival times
        departure_hour = random.randint(6, 22)
        departure_min = random.choice([0, 15, 30, 45])
        flight_duration_hours = random.randint(2, 14)
        flight_duration_mins = random.choice([0, 15, 30, 45])

        departure_time = f"{departure_hour:02d}:{departure_min:02d}"
        arrival_hour = (departure_hour + flight_duration_hours) % 24
        arrival_min = (departure_min + flight_duration_mins) % 60
        arrival_time = f"{arrival_hour:02d}:{arrival_min:02d}"

        # Generate price based on duration
        base_price = 200 + (flight_duration_hours * 50)
        price_variation = random.randint(-100, 300)
        price = base_price + price_variation

        # Determine if it's direct or has stops
        stops = random.choice([0, 0, 0, 1, 2])  # Bias towards direct flights

        flight = {
            "flight_number": f"{random.choice(['UA', 'DL', 'AA', 'BA', 'EK'])}{random.randint(100, 999)}",
            "airline": random.choice(AIRLINES),
            "aircraft": random.choice(AIRCRAFT_TYPES),
            "origin": origin,
            "destination": destination,
            "date": date,
            "departure_time": departure_time,
            "arrival_time": arrival_time,
            "duration": f"{flight_duration_hours}h {flight_duration_mins}m",
            "stops": stops,
            "price_usd": price,
            "available_seats": random.randint(5, 150),
            "class": random.choice(
                ["Economy", "Economy", "Premium Economy", "Business"]
            ),
        }
        flights.append(flight)

    # Sort by price
    flights.sort(key=lambda x: x["price_usd"])

    return flights


def extract_flight_params(messages):
    """Extract flight search parameters from messages."""
    origin = None
    destination = None
    date = None

    # Look through messages for flight details
    for msg in reversed(messages):
        if msg.role == "user":
            content = msg.content.lower()

            # Look for "from X to Y" pattern
            if " from " in content and " to " in content:
                parts = content.split(" from ")
                if len(parts) > 1:
                    remaining = parts[1]
                    if " to " in remaining:
                        city_parts = remaining.split(" to ")
                        origin = city_parts[0].strip().title()
                        # Extract destination (may have more text after)
                        dest_words = city_parts[1].strip().split()
                        if dest_words:
                            destination = dest_words[0].title()

            # Look for date mentions
            if "tomorrow" in content:
                date = (datetime.now() + timedelta(days=1)).strftime("%Y-%m-%d")
            elif "next week" in content:
                date = (datetime.now() + timedelta(days=7)).strftime("%Y-%m-%d")

    # Defaults
    if not origin:
        origin = "New York"
    if not destination:
        destination = "London"
    if not date:
        date = (datetime.now() + timedelta(days=7)).strftime("%Y-%m-%d")

    return origin, destination, date


# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",
)

# FastAPI app for REST server
app = FastAPI(title="Flight Booking Agent", version="1.0.0")


def prepare_flight_messages(request_body: ChatCompletionRequest):
    """Prepare messages with flight data."""
    # Extract flight parameters
    origin, destination, date = extract_flight_params(request_body.messages)

    # Check if user wants to book (vs just search)
    last_user_msg = ""
    for msg in reversed(request_body.messages):
        if msg.role == "user":
            last_user_msg = msg.content.lower()
            break

    is_booking = any(
        word in last_user_msg for word in ["book", "reserve", "purchase", "buy"]
    )

    # Generate flight search results
    flights = generate_flight_data(origin, destination, date)

    flight_context = f"""
Flight search results for {origin} to {destination} on {date}:

{json.dumps(flights, indent=2)}

{'User wants to book a flight. Help them complete the booking.' if is_booking else 'Present these options to the user clearly.'}
"""

    response_messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "system", "content": flight_context},
    ]

    # Add conversation history
    for msg in request_body.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    return response_messages


@app.post("/v1/chat/completions")
async def chat_completion_http(request: Request, request_body: ChatCompletionRequest):
    """HTTP endpoint for chat completions with streaming support."""
    logger.info(
        f"Received flight booking request with {len(request_body.messages)} messages"
    )

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
    # Prepare messages with flight data
    response_messages = prepare_flight_messages(request_body)

    try:
        logger.info(
            f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate flight response"
        )

        # Prepare extra headers
        extra_headers = {"x-envoy-max-retries": "3"}
        if traceparent_header:
            extra_headers["traceparent"] = traceparent_header

        response_stream = await archgw_client.chat.completions.create(
            model=FLIGHT_MODEL,
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
        logger.error(f"Error generating flight response: {e}")

        error_chunk = ChatCompletionStreamResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "delta": {
                        "content": "I apologize, but I'm having trouble searching for flights right now. Please try again."
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
    return {"status": "healthy", "agent": "flight_booking"}


def start_server(host: str = "localhost", port: int = 10520):
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
                    "format": "%(asctime)s - [FLIGHT_AGENT] - %(levelname)s - %(message)s",
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
