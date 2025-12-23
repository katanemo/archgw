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
FLIGHT_MODEL = "openai/gpt-4o"
FLIGHT_EXTRACTION_MODEL = "openai/gpt-4o-mini"

# FlightAware AeroAPI configuration
AEROAPI_BASE_URL = "https://aeroapi.flightaware.com/aeroapi"
AEROAPI_KEY = os.getenv("AEROAPI_KEY", "ESVFX7TJLxB7OTuayUv0zTQBryA3tOPr")

# HTTP client for API calls
http_client = httpx.AsyncClient(timeout=30.0)

# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",
)

# System prompt for flight agent
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


FLIGHT_EXTRACTION_PROMPT = """You are a flight information extraction assistant. Your ONLY job is to extract flight-related information from user messages and convert it to structured data.

CRITICAL RULES:
1. Extract origin city/airport and destination city/airport from the message AND conversation context
2. Extract any mentioned dates or time references
3. **CROSS-AGENT REFERENCE HANDLING - CRITICAL**: When extracting flight info, use cities mentioned in weather queries as context
   - If a weather query mentions a city (e.g., "weather in Seattle"), use that city to fill missing flight origin/destination
   - Example: "What is the weather in Seattle and what flight goes to New York direct?"
     → Weather mentions "Seattle" → Use Seattle as flight origin
     → Extract origin=Seattle, destination=New York
   - Example: "What is the weather in Atlanta and what flight goes from Detroit to Atlanta?"
     → Extract origin=Detroit, destination=Atlanta (both explicitly mentioned in flight part)
   - **ALWAYS check conversation history for cities mentioned in weather queries** - use them to infer missing flight origin/destination
4. **MULTI-PART QUERY HANDLING**: When the user asks about both weather/flights/currency in one query, extract ONLY the flight-related parts
   - Look for patterns like "flight from X to Y", "flights from X", "flights to Y", "flight goes from X to Y"
   - Example: "What is the weather in Atlanta and what flight goes from Detroit to Atlanta?" → Extract origin=Detroit, destination=Atlanta (ignore Atlanta weather part)
   - Example: "What's the weather in Seattle, and what is one flight that goes direct to Atlanta?" → Extract origin=Seattle (from weather context), destination=Atlanta
   - Focus on the flight route, but use weather context to fill missing parts
5. PAY ATTENTION TO CONVERSATION CONTEXT - THIS IS CRITICAL:
   - If previous messages mention cities/countries, use that context to resolve pronouns and incomplete queries
   - Example 1: Previous: "What's the weather in Istanbul?" → Current: "Do they fly out from Seattle?"
     → "they" refers to Istanbul → origin=Istanbul, destination=Seattle
   - Example 2: Previous: "What's the weather in London?" → Current: "What flights go from there to Seattle?"
     → "there" = London → origin=London, destination=Seattle
   - Example 3: Previous: "What's the exchange rate for Turkey?" → Current: "Do they have flights to Seattle?"
     → "they" refers to Turkey/Istanbul → origin=Istanbul, destination=Seattle
   - Example 4: Previous: "What is the weather in Seattle?" → Current: "What flight goes to New York direct?"
     → Seattle mentioned in weather query → Use Seattle as origin → origin=Seattle, destination=New York
6. For follow-up questions like "Do they fly out from X?" or "Do they have flights to Y?":
   - Look for previously mentioned cities/countries in the conversation
   - If a city was mentioned earlier, use it as the missing origin or destination
   - If the question mentions a city explicitly, use that city
   - Try to infer the complete route from context
7. Extract dates and time references:
   - "tomorrow", "today", "next week", specific dates
   - Convert relative dates to ISO format (YYYY-MM-DD) when possible
8. Determine the origin and destination based on context:
   - "from X to Y" → origin=X, destination=Y
   - "X to Y" → origin=X, destination=Y
   - "flight goes from X to Y" → origin=X, destination=Y
   - "flights from X" → origin=X, destination=null (UNLESS conversation context provides a previously mentioned city - use that as destination)
   - "flights to Y" → origin=null (UNLESS conversation context provides a previously mentioned city - use that as origin), destination=Y
   - "What flights go direct from X?" → origin=X, destination=from conversation context (if a city was mentioned earlier)
   - "Do they fly out from X?" → origin=X (or from context), destination=from context (check ALL previous messages for mentioned cities)
   - "Do they have flights to Y?" → origin=from context (check ALL previous messages), destination=Y
   - CRITICAL: When only one part (origin OR destination) is provided, ALWAYS check conversation history for the missing part
8. Return your response as a JSON object with the following structure:
   {
     "origin": "London" or null,
     "destination": "Seattle" or null,
     "date": "2025-12-20" or null,
     "origin_airport_code": "LHR" or null,
     "destination_airport_code": "SEA" or null
   }

9. If you cannot determine a value, use null for that field
10. Use city names (not airport codes) in origin/destination fields - airport codes will be resolved separately
11. Ignore error messages, HTML tags, and assistant responses
12. Extract from the most recent user message BUT use conversation context to resolve references
13. For dates: Use ISO format (YYYY-MM-DD). If relative date like "tomorrow", calculate the actual date
14. IMPORTANT: When a follow-up question mentions one city but context has another city, try to infer the complete route

Examples with context:
- "What is the weather in Atlanta and what flight goes from Detroit to Atlanta?" → {"origin": "Detroit", "destination": "Atlanta", "date": null, "origin_airport_code": null, "destination_airport_code": null}
- "What is the weather in Seattle and what flight goes to New York direct?" → {"origin": "Seattle", "destination": "New York", "date": null, "origin_airport_code": null, "destination_airport_code": null} (Seattle from weather context)
- Conversation: "What's the weather in Istanbul?" → Current: "Do they fly out from Seattle?" → {"origin": "Istanbul", "destination": "Seattle", "date": null, "origin_airport_code": null, "destination_airport_code": null}
- Conversation: "What's the weather in Istanbul?" → Current: "What flights go direct from Seattle?" → {"origin": "Seattle", "destination": "Istanbul", "date": null, "origin_airport_code": null, "destination_airport_code": null} (Istanbul from previous context)
- Conversation: "What's the weather in London?" → Current: "What flights go from there to Seattle?" → {"origin": "London", "destination": "Seattle", "date": null, "origin_airport_code": null, "destination_airport_code": null}
- Conversation: "Tell me about Istanbul" → Current: "Do they have flights to Seattle?" → {"origin": "Istanbul", "destination": "Seattle", "date": null, "origin_airport_code": null, "destination_airport_code": null}
- "What flights go from London to Seattle?" → {"origin": "London", "destination": "Seattle", "date": null, "origin_airport_code": null, "destination_airport_code": null}
- "Show me flights to New York tomorrow" → {"origin": null, "destination": "New York", "date": "2025-12-21", "origin_airport_code": null, "destination_airport_code": null}
- "Flights from LAX to JFK" → {"origin": "Los Angeles", "destination": "New York", "date": null, "origin_airport_code": "LAX", "destination_airport_code": "JFK"}

Now extract the flight information from this message, considering the conversation context:"""


async def extract_flight_info_from_messages(messages):
    """Extract flight information from user messages using LLM, considering conversation context."""
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

    user_messages = [msg for msg in messages if msg.role == "user"]

    if not user_messages:
        logger.warning("No user messages found")
        return {
            "origin": None,
            "destination": None,
            "date": None,
            "origin_airport_code": None,
            "destination_airport_code": None,
        }

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
        logger.warning("No valid user message found")
        return {
            "origin": None,
            "destination": None,
            "date": None,
            "origin_airport_code": None,
            "destination_airport_code": None,
        }

    try:
        logger.info(f"Extracting flight info from user message: {user_content[:200]}")
        logger.info(
            f"Using conversation context with {len(conversation_context)} messages"
        )

        llm_messages = [{"role": "system", "content": FLIGHT_EXTRACTION_PROMPT}]

        context_messages = (
            conversation_context[-10:]
            if len(conversation_context) > 10
            else conversation_context
        )
        for msg in context_messages:
            llm_messages.append({"role": msg["role"], "content": msg["content"]})

        response = await archgw_client.chat.completions.create(
            model=FLIGHT_EXTRACTION_MODEL,
            messages=llm_messages,
            temperature=0.1,
            max_tokens=300,
        )

        extracted_text = response.choices[0].message.content.strip()

        try:
            if "```json" in extracted_text:
                extracted_text = (
                    extracted_text.split("```json")[1].split("```")[0].strip()
                )
            elif "```" in extracted_text:
                extracted_text = extracted_text.split("```")[1].split("```")[0].strip()

            flight_info = json.loads(extracted_text)

            date = flight_info.get("date")
            if date:
                today = datetime.now().date()
                if date.lower() == "tomorrow":
                    date = (today + timedelta(days=1)).strftime("%Y-%m-%d")
                elif date.lower() == "today":
                    date = today.strftime("%Y-%m-%d")
                elif "next week" in date.lower():
                    date = (today + timedelta(days=7)).strftime("%Y-%m-%d")

            result = {
                "origin": flight_info.get("origin"),
                "destination": flight_info.get("destination"),
                "date": date,
                "origin_airport_code": flight_info.get("origin_airport_code"),
                "destination_airport_code": flight_info.get("destination_airport_code"),
            }

            # Fallback: If origin is missing but we have destination, infer from weather context
            if not result["origin"] and result["destination"]:
                # Look for cities mentioned in weather queries in conversation context
                for msg in reversed(conversation_context):
                    if msg["role"] == "user":
                        content = msg["content"]
                        # Look for weather queries mentioning cities
                        if (
                            "weather" in content.lower()
                            or "forecast" in content.lower()
                        ):
                            # Common patterns: "weather in [city]", "forecast for [city]", "weather [city]"
                            patterns = [
                                r"(?:weather|forecast).*?(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                                r"weather\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                            ]
                            for pattern in patterns:
                                city_match = re.search(pattern, content, re.IGNORECASE)
                                if city_match:
                                    potential_city = city_match.group(1).strip()
                                    # Don't use the same city as destination
                                    if (
                                        potential_city.lower()
                                        != result["destination"].lower()
                                    ):
                                        logger.info(
                                            f"Inferring origin from weather context in extraction: {potential_city}"
                                        )
                                        result["origin"] = potential_city
                                        break
                            if result["origin"]:
                                break

            # Fallback: If destination is missing but we have origin, try to infer from conversation context
            if result["origin"] and not result["destination"]:
                # Look for cities mentioned in previous messages
                for msg in reversed(conversation_context):
                    if msg["role"] == "user":
                        content = msg["content"]
                        # Look for weather queries mentioning cities
                        if (
                            "weather" in content.lower()
                            or "forecast" in content.lower()
                        ):
                            # Common patterns: "weather in [city]", "forecast for [city]", "weather [city]"
                            patterns = [
                                r"(?:weather|forecast).*?(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                                r"weather\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                            ]
                            for pattern in patterns:
                                city_match = re.search(pattern, content, re.IGNORECASE)
                                if city_match:
                                    potential_city = city_match.group(1).strip()
                                    # Don't use the same city as origin
                                    if (
                                        potential_city.lower()
                                        != result["origin"].lower()
                                    ):
                                        logger.info(
                                            f"Inferring destination from conversation context: {potential_city}"
                                        )
                                        result["destination"] = potential_city
                                        break
                            if result["destination"]:
                                break

            logger.info(f"LLM extracted flight info: {result}")
            return result

        except json.JSONDecodeError as e:
            logger.warning(
                f"Failed to parse JSON from LLM response: {extracted_text}, error: {e}"
            )
            return {
                "origin": None,
                "destination": None,
                "date": None,
                "origin_airport_code": None,
                "destination_airport_code": None,
            }

    except Exception as e:
        logger.error(f"Error extracting flight info with LLM: {e}, using defaults")
        return {
            "origin": None,
            "destination": None,
            "date": None,
            "origin_airport_code": None,
            "destination_airport_code": None,
        }


AIRPORT_CODE_RESOLUTION_PROMPT = """You are an airport code resolution assistant. Your ONLY job is to convert city names or locations to their primary airport IATA/ICAO codes.

CRITICAL RULES:
1. Convert city names, locations, or airport names to their primary airport code (prefer IATA 3-letter codes like JFK, LHR, LAX)
2. For cities with multiple airports, choose the PRIMARY/MOST COMMONLY USED airport:
   - London → LHR (Heathrow, not Gatwick or Stansted)
   - New York → JFK (not LGA or EWR)
   - Paris → CDG (not ORY)
   - Tokyo → NRT (Narita, not HND)
   - Beijing → PEK (not PKX)
   - Shanghai → PVG (not SHA)
3. If the input is already an airport code (3-letter IATA or 4-letter ICAO), return it as-is
4. Return ONLY the airport code, nothing else
5. Use standard IATA codes when available, ICAO codes as fallback
6. If you cannot determine the airport code, return "NOT_FOUND"

Examples:
- "London" → "LHR"
- "New York" → "JFK"
- "Los Angeles" → "LAX"
- "Seattle" → "SEA"
- "Paris" → "CDG"
- "Tokyo" → "NRT"
- "JFK" → "JFK"
- "LAX" → "LAX"
- "LHR" → "LHR"
- "Unknown City" → "NOT_FOUND"

Now convert this location to an airport code:"""


async def resolve_airport_code(city_name: str) -> Optional[str]:
    """Resolve city name to airport code using LLM and FlightAware API.

    Uses LLM to convert city names to airport codes, then validates via API.
    """
    if not city_name:
        return None

    try:
        logger.info(f"Resolving airport code for: {city_name}")

        response = await archgw_client.chat.completions.create(
            model=FLIGHT_EXTRACTION_MODEL,
            messages=[
                {"role": "system", "content": AIRPORT_CODE_RESOLUTION_PROMPT},
                {"role": "user", "content": city_name},
            ],
            temperature=0.1,
            max_tokens=50,
        )

        airport_code = response.choices[0].message.content.strip().upper()
        airport_code = airport_code.strip("\"'`.,!? \n\t")

        if airport_code == "NOT_FOUND" or not airport_code:
            logger.warning(f"LLM could not resolve airport code for {city_name}")
            return None

        logger.info(f"LLM resolved {city_name} to airport code: {airport_code}")

        try:
            url = f"{AEROAPI_BASE_URL}/airports/{airport_code}"
            headers = {"x-apikey": AEROAPI_KEY}

            validation_response = await http_client.get(url, headers=headers)

            if validation_response.status_code == 200:
                data = validation_response.json()
                validated_code = data.get("code_icao") or data.get("code_iata")
                if validated_code:
                    logger.info(
                        f"Validated airport code {airport_code} → {validated_code}"
                    )
                    return validated_code
                else:
                    return airport_code
            else:
                logger.warning(
                    f"API validation failed for {airport_code}, but using LLM result"
                )
                return airport_code

        except Exception as e:
            logger.warning(
                f"API validation error for {airport_code}: {e}, using LLM result"
            )
            return airport_code

    except Exception as e:
        logger.error(f"Error resolving airport code for {city_name} with LLM: {e}")
        return None


async def get_flights_between_airports(
    origin_code: str, dest_code: str, start_date: str = None, end_date: str = None
) -> Optional[dict]:
    """Get flights between two airports using FlightAware AeroAPI."""
    try:
        url = f"{AEROAPI_BASE_URL}/airports/{origin_code}/flights/to/{dest_code}"
        headers = {"x-apikey": AEROAPI_KEY}

        params = {}
        if start_date:
            params["start"] = start_date
        if end_date:
            params["end"] = end_date
        params["connection"] = "nonstop"
        params["max_pages"] = 1

        response = await http_client.get(url, headers=headers, params=params)

        if response.status_code != 200:
            logger.warning(
                f"FlightAware API returned status {response.status_code} for {origin_code} to {dest_code}"
            )
            return None

        data = response.json()

        flights = []
        for flight_group in data.get("flights", []):
            segments = flight_group.get("segments", [])
            if segments:
                segment = segments[0]
                flight_info = {
                    "ident": segment.get("ident"),
                    "ident_icao": segment.get("ident_icao"),
                    "ident_iata": segment.get("ident_iata"),
                    "operator": segment.get("operator"),
                    "operator_iata": segment.get("operator_iata"),
                    "flight_number": segment.get("flight_number"),
                    "origin": {
                        "code": segment.get("origin", {}).get("code"),
                        "code_iata": segment.get("origin", {}).get("code_iata"),
                        "name": segment.get("origin", {}).get("name"),
                        "city": segment.get("origin", {}).get("city"),
                    },
                    "destination": {
                        "code": segment.get("destination", {}).get("code"),
                        "code_iata": segment.get("destination", {}).get("code_iata"),
                        "name": segment.get("destination", {}).get("name"),
                        "city": segment.get("destination", {}).get("city"),
                    },
                    "scheduled_out": segment.get("scheduled_out"),
                    "estimated_out": segment.get("estimated_out"),
                    "actual_out": segment.get("actual_out"),
                    "scheduled_off": segment.get("scheduled_off"),
                    "estimated_off": segment.get("estimated_off"),
                    "actual_off": segment.get("actual_off"),
                    "scheduled_on": segment.get("scheduled_on"),
                    "estimated_on": segment.get("estimated_on"),
                    "actual_on": segment.get("actual_on"),
                    "scheduled_in": segment.get("scheduled_in"),
                    "estimated_in": segment.get("estimated_in"),
                    "actual_in": segment.get("actual_in"),
                    "status": segment.get("status"),
                    "aircraft_type": segment.get("aircraft_type"),
                    "departure_delay": segment.get("departure_delay"),
                    "arrival_delay": segment.get("arrival_delay"),
                    "gate_origin": segment.get("gate_origin"),
                    "gate_destination": segment.get("gate_destination"),
                    "terminal_origin": segment.get("terminal_origin"),
                    "terminal_destination": segment.get("terminal_destination"),
                    "cancelled": segment.get("cancelled"),
                    "diverted": segment.get("diverted"),
                }
                flights.append(flight_info)

        return {
            "origin_code": origin_code,
            "destination_code": dest_code,
            "flights": flights,
            "num_flights": len(flights),
        }

    except httpx.HTTPError as e:
        logger.error(
            f"HTTP error fetching flights from {origin_code} to {dest_code}: {e}"
        )
        return None
    except json.JSONDecodeError as e:
        logger.error(f"Failed to parse JSON response from FlightAware API: {e}")
        return None
    except Exception as e:
        logger.error(f"Unexpected error fetching flights: {e}")
        return None


app = FastAPI(title="Flight Information Agent", version="1.0.0")


async def prepare_flight_messages(request_body: ChatCompletionRequest):
    """Prepare messages with flight data."""
    flight_info = await extract_flight_info_from_messages(request_body.messages)

    origin = flight_info.get("origin")
    destination = flight_info.get("destination")
    date = flight_info.get("date")
    origin_code = flight_info.get("origin_airport_code")
    dest_code = flight_info.get("destination_airport_code")

    # Enhanced context extraction: Use weather queries to infer missing origin or destination
    # CRITICAL: When user asks "weather in X and flight to Y", use X as origin
    if not origin and destination:
        # Look through conversation history for cities mentioned in weather queries
        for msg in request_body.messages:
            if msg.role == "user":
                content = msg.content
                # Extract cities from weather queries: "weather in [city]", "forecast for [city]"
                weather_patterns = [
                    r"(?:weather|forecast).*?(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                    r"weather\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                ]
                for pattern in weather_patterns:
                    matches = re.findall(pattern, content, re.IGNORECASE)
                    for match in matches:
                        city = match.strip()
                        # Use weather city as origin if it's different from destination
                        if (
                            city.lower() != destination.lower()
                            and len(city.split()) <= 3
                        ):
                            origin = city
                            logger.info(
                                f"Inferred origin from weather context: {origin} (destination: {destination})"
                            )
                            flight_info["origin"] = origin
                            break
                    if origin:
                        break
                if origin:
                    break

    # If destination is missing but origin is present, try to infer from conversation
    if origin and not destination:
        # Look through conversation history for mentioned cities
        mentioned_cities = set()
        for msg in request_body.messages:
            if msg.role == "user":
                content = msg.content
                # Extract cities from weather queries: "weather in [city]", "forecast for [city]"
                weather_patterns = [
                    r"(?:weather|forecast).*?(?:in|for)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                    r"weather\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
                ]
                for pattern in weather_patterns:
                    matches = re.findall(pattern, content, re.IGNORECASE)
                    for match in matches:
                        city = match.strip()
                        # Don't use same city as origin, and validate it's a real city name
                        if city.lower() != origin.lower() and len(city.split()) <= 3:
                            mentioned_cities.add(city)

        # If we found cities in context, use the first one as destination
        if mentioned_cities:
            destination = list(mentioned_cities)[0]
            logger.info(
                f"Inferred destination from conversation context: {destination}"
            )
            flight_info["destination"] = destination

    if origin and not origin_code:
        origin_code = await resolve_airport_code(origin)
    if destination and not dest_code:
        dest_code = await resolve_airport_code(destination)

    if not date:
        date = (datetime.now() + timedelta(days=1)).strftime("%Y-%m-%d")

    start_date = f"{date}T00:00:00Z"
    end_date = f"{date}T23:59:59Z"

    flight_data = None
    if origin_code and dest_code:
        flight_data = await get_flights_between_airports(
            origin_code, dest_code, start_date, end_date
        )
    else:
        logger.warning(
            f"Cannot fetch flights: origin_code={origin_code}, dest_code={dest_code}"
        )

    # Build context message based on what we have
    if flight_data and flight_data.get("flights"):
        flight_context = f"""
Flight search results for {origin or origin_code} to {destination or dest_code} on {date}:

{json.dumps(flight_data, indent=2)}

Use this data to answer the user's flight query. Present the flights clearly with all relevant details.
"""
    elif origin_code and not dest_code:
        # We have origin but no destination - this is a follow-up question
        flight_context = f"""
The user is asking about flights from {origin or origin_code}, but no destination was specified.

From the conversation context, it appears the user may be asking about flights from {origin or origin_code} to a previously mentioned location, or they may need to specify a destination.

Check the conversation history to see if a destination was mentioned earlier. If so, you can mention that you'd be happy to search for flights from {origin or origin_code} to that destination. If not, politely ask the user to specify both origin and destination cities.

Example response: "I can help you find flights from {origin or origin_code}! Could you please tell me which city you'd like to fly to? For example, 'flights from {origin or origin_code} to Seattle' or 'flights from {origin or origin_code} to Istanbul'."
"""
    elif dest_code and not origin_code:
        # We have destination but no origin
        flight_context = f"""
The user is asking about flights to {destination or dest_code}, but no origin was specified.

From the conversation context, it appears the user may be asking about flights to {destination or dest_code} from a previously mentioned location, or they may need to specify an origin.

Check the conversation history to see if an origin was mentioned earlier. If so, you can mention that you'd be happy to search for flights from that origin to {destination or dest_code}. If not, politely ask the user to specify both origin and destination cities.
"""
    else:
        # Neither origin nor destination
        flight_context = f"""
Flight search attempted but both origin and destination are missing.

The user's query was incomplete. Check the conversation history to see if origin or destination cities were mentioned earlier. If so, use that context to help the user. If not, politely ask the user to specify both origin and destination cities for their flight search.

Example: "I'd be happy to help you find flights! Could you please tell me both the departure city and destination city? For example, 'flights from Seattle to Istanbul' or 'flights from Istanbul to Seattle'."
"""

    response_messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "assistant", "content": flight_context},
    ]

    # Add conversation history
    for msg in request_body.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    return response_messages


@app.post("/v1/chat/completions")
async def chat_completion_http(request: Request, request_body: ChatCompletionRequest):
    """HTTP endpoint for chat completions with streaming support."""
    logger.info(f"Received flight request with {len(request_body.messages)} messages")

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

    logger.info("Preparing flight messages for LLM")
    # Prepare messages with flight data
    response_messages = await prepare_flight_messages(request_body)

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

        full_response = "".join(collected_content)
        updated_history = [{"role": "assistant", "content": full_response}]

        logger.info(f"Full flight agent response: {full_response}")

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
                        "content": "I apologize, but I'm having trouble retrieving flight information right now. Please try again."
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
    return {"status": "healthy", "agent": "flight_information"}


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=10520)


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
