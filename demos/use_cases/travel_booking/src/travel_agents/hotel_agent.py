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
    format="%(asctime)s - [HOTEL_AGENT] - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)

# Configuration for archgw LLM gateway
LLM_GATEWAY_ENDPOINT = os.getenv("LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1")
HOTEL_MODEL = "gpt-4o-mini"

# Sample hotel data
HOTEL_CHAINS = [
    "Marriott",
    "Hilton",
    "Hyatt",
    "InterContinental",
    "Four Seasons",
    "Sheraton",
    "Ritz-Carlton",
    "Westin",
]
HOTEL_TYPES = [
    "Luxury Hotel",
    "Business Hotel",
    "Boutique Hotel",
    "Resort",
    "City Center Hotel",
]
AMENITIES = [
    ["Free WiFi", "Pool", "Gym", "Restaurant", "Bar"],
    ["Free WiFi", "Gym", "Business Center", "Room Service"],
    ["Free WiFi", "Spa", "Pool", "Restaurant", "Concierge"],
    ["Free WiFi", "Beach Access", "Pool", "Restaurant", "Water Sports"],
    ["Free WiFi", "Rooftop Bar", "Restaurant", "City Views"],
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

# System prompt for hotel agent
SYSTEM_PROMPT = """You are a helpful hotel reservation assistant.

Your role is to help users find and book hotels that match their needs and preferences.

When responding:
1. Parse the user's request to understand location, dates, number of guests, and preferences
2. Use the hotel search results provided in the conversation context
3. Present hotel options clearly with key details (name, rating, amenities, price per night)
4. Help users compare options based on their priorities (location, price, amenities, etc.)
5. If they want to book, confirm their selection and provide booking confirmation
6. Ask clarifying questions if needed information is missing (dates, number of rooms, guests, etc.)

Be professional, attentive to details, and help make the booking process smooth."""


def generate_hotel_data(
    location: str, check_in: str = None, check_out: str = None, num_results: int = 5
):
    """Generate mock hotel search results."""
    if not check_in:
        check_in = (datetime.now() + timedelta(days=7)).strftime("%Y-%m-%d")
    if not check_out:
        check_out = (datetime.now() + timedelta(days=10)).strftime("%Y-%m-%d")

    # Calculate number of nights
    check_in_date = datetime.strptime(check_in, "%Y-%m-%d")
    check_out_date = datetime.strptime(check_out, "%Y-%m-%d")
    nights = (check_out_date - check_in_date).days

    hotels = []
    for i in range(num_results):
        # Generate hotel details
        chain = random.choice(HOTEL_CHAINS)
        hotel_type = random.choice(HOTEL_TYPES)
        rating = round(random.uniform(3.5, 5.0), 1)

        # Generate price based on rating and location
        base_price = 100 + (rating - 3.5) * 100
        price_variation = random.randint(-50, 150)
        price_per_night = int(base_price + price_variation)

        # Distance from city center
        distance_km = round(random.uniform(0.5, 15.0), 1)

        hotel = {
            "name": f"{chain} {location} {random.choice(['Downtown', 'City Center', 'Waterfront', 'Airport', 'Marina'])}",
            "type": hotel_type,
            "rating": rating,
            "location": location,
            "distance_from_center_km": distance_km,
            "check_in": check_in,
            "check_out": check_out,
            "nights": nights,
            "price_per_night_usd": price_per_night,
            "total_price_usd": price_per_night * nights,
            "amenities": random.choice(AMENITIES),
            "available_rooms": random.randint(3, 50),
            "room_type": random.choice(
                ["Standard Room", "Deluxe Room", "Suite", "Executive Room"]
            ),
            "cancellation_policy": random.choice(
                [
                    "Free cancellation until 24h before",
                    "Free cancellation until 48h before",
                    "Non-refundable",
                ]
            ),
        }
        hotels.append(hotel)

    # Sort by rating (descending) then price
    hotels.sort(key=lambda x: (-x["rating"], x["price_per_night_usd"]))

    return hotels


def extract_hotel_params(messages):
    """Extract hotel search parameters from messages."""
    location = None
    check_in = None
    check_out = None

    # Look through messages for hotel details
    for msg in reversed(messages):
        if msg.role == "user":
            content = msg.content.lower()

            # Look for location - "in X" or "hotel in X"
            if " in " in content:
                parts = content.split(" in ")
                if len(parts) > 1:
                    # Get the word after "in"
                    location_words = parts[1].strip().split()
                    if location_words:
                        location = location_words[0].title()

            # Look for date mentions
            if "tomorrow" in content:
                check_in = (datetime.now() + timedelta(days=1)).strftime("%Y-%m-%d")
                check_out = (datetime.now() + timedelta(days=4)).strftime("%Y-%m-%d")
            elif "next week" in content:
                check_in = (datetime.now() + timedelta(days=7)).strftime("%Y-%m-%d")
                check_out = (datetime.now() + timedelta(days=10)).strftime("%Y-%m-%d")

    # Defaults
    if not location:
        location = "New York"
    if not check_in:
        check_in = (datetime.now() + timedelta(days=7)).strftime("%Y-%m-%d")
    if not check_out:
        check_out = (datetime.now() + timedelta(days=10)).strftime("%Y-%m-%d")

    return location, check_in, check_out


# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",
)

# FastAPI app for REST server
app = FastAPI(title="Hotel Reservation Agent", version="1.0.0")


def prepare_hotel_messages(request_body: ChatCompletionRequest):
    """Prepare messages with hotel data."""
    # Extract hotel parameters
    location, check_in, check_out = extract_hotel_params(request_body.messages)

    # Check if user wants to book (vs just search)
    last_user_msg = ""
    for msg in reversed(request_body.messages):
        if msg.role == "user":
            last_user_msg = msg.content.lower()
            break

    is_booking = any(word in last_user_msg for word in ["book", "reserve", "confirm"])

    # Generate hotel search results
    hotels = generate_hotel_data(location, check_in, check_out)

    hotel_context = f"""
Hotel search results for {location} from {check_in} to {check_out}:

{json.dumps(hotels, indent=2)}

{'User wants to book a hotel. Help them complete the reservation.' if is_booking else 'Present these options to the user clearly.'}
"""

    response_messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "system", "content": hotel_context},
    ]

    # Add conversation history
    for msg in request_body.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    return response_messages


@app.post("/v1/chat/completions")
async def chat_completion_http(request: Request, request_body: ChatCompletionRequest):
    """HTTP endpoint for chat completions with streaming support."""
    logger.info(
        f"Received hotel reservation request with {len(request_body.messages)} messages"
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
    # Prepare messages with hotel data
    response_messages = prepare_hotel_messages(request_body)

    try:
        logger.info(
            f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate hotel response"
        )

        # Prepare extra headers
        extra_headers = {"x-envoy-max-retries": "3"}
        if traceparent_header:
            extra_headers["traceparent"] = traceparent_header

        response_stream = await archgw_client.chat.completions.create(
            model=HOTEL_MODEL,
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
        logger.error(f"Error generating hotel response: {e}")

        error_chunk = ChatCompletionStreamResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "delta": {
                        "content": "I apologize, but I'm having trouble searching for hotels right now. Please try again."
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
    return {"status": "healthy", "agent": "hotel_reservation"}


def start_server(host: str = "localhost", port: int = 10530):
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
                    "format": "%(asctime)s - [HOTEL_AGENT] - %(levelname)s - %(message)s",
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
