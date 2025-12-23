import json
from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
from openai import AsyncOpenAI
import os
import logging
import time
import uuid
import uvicorn
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
    format="%(asctime)s - [CURRENCY_AGENT] - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)

# Configuration for archgw LLM gateway
LLM_GATEWAY_ENDPOINT = os.getenv("LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1")
CURRENCY_MODEL = "openai/gpt-4o"
CURRENCY_EXTRACTION_MODEL = "openai/gpt-4o-mini"

# HTTP client for API calls
http_client = httpx.AsyncClient(timeout=10.0)

# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",
)

# System prompt for currency agent
SYSTEM_PROMPT = """You are a professional travel planner assistant. Your role is to provide accurate, clear, and helpful information about weather, flights, and currency exchange based on the structured data provided to you.

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

   CURRENCY DATA:
   - You will receive currency exchange data as JSON in a system message
   - The data contains: from_currency, to_currency, rate, date, and optionally original_amount and converted_amount
   - Some fields may be null/None - handle these gracefully

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

4. CURRENCY HANDLING:
   - Present exchange rates clearly with both currency codes and names when helpful
   - Include the date of the exchange rate
   - If an amount was provided, show both the original and converted amounts
   - Use clear formatting (e.g., "100 USD = 92.50 EUR" or "1 USD = 0.925 EUR")
   - If rate data is unavailable, acknowledge this politely

5. MULTI-PART QUERIES:
   - Users may ask about weather, flights, and currency in one message
   - Answer ALL parts of the query that you have data for
   - Organize your response logically - typically weather first, then flights, then currency, or based on the query order
   - Provide complete information for each topic without mentioning other agents
   - If you receive data for only one topic but the user asked about multiple, answer what you can with the provided data

6. ERROR HANDLING:
   - If weather forecast contains an "error" field, acknowledge the issue politely
   - If temperature or condition is null/None, mention that specific data is unavailable
   - If flight details are incomplete, state which information is unavailable
   - If currency rate is unavailable, mention that specific data is unavailable
   - Never invent or guess weather, flight, or currency data - only use what's provided
   - If location couldn't be determined, acknowledge this but still provide available data

7. RESPONSE FORMAT:

   For Weather:
   - Single-day queries: Provide current conditions, temperature, and condition
   - Multi-day forecasts: List each day with date, day name, high/low temps, and condition
   - Include sunrise/sunset times when available and relevant

   For Flights:
   - List flights with clear numbering or bullet points
   - Include key details: airline, flight number, departure/arrival times, airports
   - Add gate, terminal, and status information when available
   - For multiple flights, organize chronologically

   For Currency:
   - Show exchange rate clearly: "1 [FROM] = [RATE] [TO]"
   - If amount provided: "[AMOUNT] [FROM] = [CONVERTED] [TO]"
   - Include the date of the exchange rate

   General:
   - Use natural, conversational language
   - Be concise but complete
   - Format dates and times clearly
   - Use bullet points or numbered lists for clarity

8. LOCATION HANDLING:
   - Always mention location names from the data
   - For flights, clearly state origin and destination cities/airports
   - For currency, use country/city context to resolve currency references
   - If locations differ from what the user asked, acknowledge this politely

9. RESPONSE STYLE:
   - Be friendly and professional
   - Use natural language, not technical jargon
   - Provide information in a logical, easy-to-read format
   - When answering multi-part queries, create a cohesive response that addresses all aspects

Remember: Only use the data provided. Never fabricate weather, flight, or currency information. If data is missing, clearly state what's unavailable. Answer all parts of the user's query that you have data for."""


CURRENCY_EXTRACTION_PROMPT = """You are a currency information extraction assistant. Your ONLY job is to extract currency-related information from user messages and convert it to standard 3-letter ISO currency codes.

CRITICAL RULES:
1. Extract currency codes (3-letter ISO codes like USD, EUR, GBP, JPY, PKR, etc.) from the message AND conversation context
2. Extract any mentioned amounts or numbers that might be currency amounts
3. PAY ATTENTION TO CONVERSATION CONTEXT:
   - If previous messages mention a country/city, use that context to resolve pronouns like "their", "that country", "there", etc.
   - Example: If previous message was "What's the weather in Lahore, Pakistan?" and current message is "What is their currency exchange rate with USD?", then "their" = Pakistan = PKR
   - Look for country names in the conversation history to infer currencies
4. If country names or regions are mentioned (in current message OR conversation context), convert them to their standard currency codes:
   - United States/USA/US → USD
   - Europe/Eurozone/France/Germany/Italy/Spain/etc. → EUR
   - United Kingdom/UK/Britain → GBP
   - Japan → JPY
   - China → CNY
   - India → INR
   - Pakistan → PKR
   - Australia → AUD
   - Canada → CAD
   - Switzerland → CHF
   - South Korea → KRW
   - Singapore → SGD
   - Hong Kong → HKD
   - Brazil → BRL
   - Mexico → MXN
   - And any other countries you know the currency for
5. Determine the FROM currency (source) and TO currency (target) based on context:
   - "from X to Y" → from_currency=X, to_currency=Y
   - "X to Y" → from_currency=X, to_currency=Y
   - "convert X to Y" → from_currency=X, to_currency=Y
   - "X in Y" → from_currency=X, to_currency=Y
   - "rate for X" or "X rate" → to_currency=X (assume USD as base)
   - "their currency with USD" or "their currency to USD" → from_currency=country_from_context, to_currency=USD
   - "X dollars/euros/pounds/etc." → from_currency=X
6. If only one currency is mentioned, determine if it's the source or target based on context
7. ALWAYS return currency codes, never country names in the currency fields
8. Return your response as a JSON object with the following structure:
   {
     "from_currency": "USD" or null,
     "to_currency": "EUR" or null,
     "amount": 100.0 or null
   }

9. If you cannot determine a currency, use null for that field
10. Use standard 3-letter ISO currency codes ONLY
11. Ignore error messages, HTML tags, and assistant responses
12. Extract from the most recent user message BUT use conversation context to resolve references
13. Default behavior: If only one currency is mentioned without context, assume it's the target currency and use USD as the source

Examples with context:
- Conversation: "What's the weather in Lahore, Pakistan?" → Current: "What is their currency exchange rate with USD?" → {"from_currency": "PKR", "to_currency": "USD", "amount": null}
- Conversation: "Tell me about Tokyo" → Current: "What's their currency rate?" → {"from_currency": "JPY", "to_currency": "USD", "amount": null}
- "What's the exchange rate from USD to EUR?" → {"from_currency": "USD", "to_currency": "EUR", "amount": null}
- "Convert 100 dollars to euros" → {"from_currency": "USD", "to_currency": "EUR", "amount": 100.0}
- "How much is 50 GBP in Japanese yen?" → {"from_currency": "GBP", "to_currency": "JPY", "amount": 50.0}
- "What's the rate for euros?" → {"from_currency": "USD", "to_currency": "EUR", "amount": null}
- "Convert money from United States to France" → {"from_currency": "USD", "to_currency": "EUR", "amount": null}
- "100 pounds to dollars" → {"from_currency": "GBP", "to_currency": "USD", "amount": 100.0}

Now extract the currency information from this message, considering the conversation context:"""


async def extract_currency_info_from_messages(messages):
    """Extract currency information from user messages using LLM, considering conversation context."""
    # Get all messages for context (both user and assistant)
    conversation_context = []
    for msg in messages:
        # Skip error messages and HTML tags
        content = msg.content.strip()
        content_lower = content.lower()
        if any(
            pattern in content_lower
            for pattern in ["<", ">", "error:", "i apologize", "i'm having trouble"]
        ):
            continue
        conversation_context.append({"role": msg.role, "content": content})

    # Get the most recent user message
    user_messages = [msg for msg in messages if msg.role == "user"]

    if not user_messages:
        logger.warning("No user messages found")
        return {"from_currency": "USD", "to_currency": "EUR", "amount": None}

    # Get the most recent user message (skip error messages and HTML tags)
    user_content = None
    for msg in reversed(user_messages):
        content = msg.content.strip()
        # Skip messages with error patterns or HTML tags
        content_lower = content.lower()
        if any(
            pattern in content_lower
            for pattern in [
                "<",
                ">",
                "assistant:",
                "error:",
                "i apologize",
                "i'm having trouble",
            ]
        ):
            continue
        user_content = content
        break

    if not user_content:
        logger.warning("No valid user message found")
        return {"from_currency": "USD", "to_currency": "EUR", "amount": None}

    try:
        logger.info(f"Extracting currency info from user message: {user_content[:200]}")
        logger.info(
            f"Using conversation context with {len(conversation_context)} messages"
        )

        llm_messages = [{"role": "system", "content": CURRENCY_EXTRACTION_PROMPT}]

        context_messages = (
            conversation_context[-10:]
            if len(conversation_context) > 10
            else conversation_context
        )
        for msg in context_messages:
            llm_messages.append({"role": msg["role"], "content": msg["content"]})

        response = await archgw_client.chat.completions.create(
            model=CURRENCY_EXTRACTION_MODEL,
            messages=llm_messages,
            temperature=0.1,
            max_tokens=200,
        )

        extracted_text = response.choices[0].message.content.strip()

        try:
            if "```json" in extracted_text:
                extracted_text = (
                    extracted_text.split("```json")[1].split("```")[0].strip()
                )
            elif "```" in extracted_text:
                extracted_text = extracted_text.split("```")[1].split("```")[0].strip()

            currency_info = json.loads(extracted_text)

            from_currency = currency_info.get("from_currency")
            to_currency = currency_info.get("to_currency")
            amount = currency_info.get("amount")

            if not from_currency:
                from_currency = "USD"
            if not to_currency:
                to_currency = "EUR"

            result = {
                "from_currency": from_currency,
                "to_currency": to_currency,
                "amount": amount,
            }

            logger.info(f"LLM extracted currency info: {result}")
            return result

        except json.JSONDecodeError as e:
            logger.warning(
                f"Failed to parse JSON from LLM response: {extracted_text}, error: {e}"
            )
            return {"from_currency": "USD", "to_currency": "EUR", "amount": None}

    except Exception as e:
        logger.error(f"Error extracting currency info with LLM: {e}, using defaults")
        return {"from_currency": "USD", "to_currency": "EUR", "amount": None}


async def get_currency_exchange_rate(
    from_currency: str, to_currency: str
) -> Optional[dict]:
    """Get currency exchange rate between two currencies using Frankfurter API.

    Uses the Frankfurter API (api.frankfurter.dev) which provides free, open-source
    currency data tracking reference exchange rates published by institutional sources.
    No API keys required.

    Args:
        from_currency: Base currency code (e.g., "USD", "EUR")
        to_currency: Target currency code (e.g., "EUR", "GBP")

    Returns:
        Dictionary with exchange rate data or None if error occurs
    """
    try:
        url = f"https://api.frankfurter.dev/v1/latest?base={from_currency}&symbols={to_currency}"
        response = await http_client.get(url)

        if response.status_code != 200:
            logger.warning(
                f"Currency API returned status {response.status_code} for {from_currency} to {to_currency}"
            )
            return None

        data = response.json()

        if "rates" not in data:
            logger.warning(f"Invalid API response structure: missing 'rates' field")
            return None

        if to_currency not in data["rates"]:
            logger.warning(
                f"Currency {to_currency} not found in API response for base {from_currency}"
            )
            return None

        return {
            "from_currency": from_currency,
            "to_currency": to_currency,
            "rate": data["rates"][to_currency],
            "date": data.get("date"),
            "base": data.get("base"),
        }
    except httpx.HTTPError as e:
        logger.error(
            f"HTTP error fetching currency rate from {from_currency} to {to_currency}: {e}"
        )
        return None
    except json.JSONDecodeError as e:
        logger.error(f"Failed to parse JSON response from currency API: {e}")
        return None
    except Exception as e:
        logger.error(f"Unexpected error fetching currency rate: {e}")
        return None


# FastAPI app for REST server
app = FastAPI(title="Currency Exchange Agent", version="1.0.0")


async def prepare_currency_messages(request_body: ChatCompletionRequest):
    """Prepare messages with currency exchange data."""
    # Extract currency information from conversation using LLM
    currency_info = await extract_currency_info_from_messages(request_body.messages)

    from_currency = currency_info["from_currency"]
    to_currency = currency_info["to_currency"]
    amount = currency_info.get("amount")

    # Get currency exchange rate
    rate_data = await get_currency_exchange_rate(from_currency, to_currency)

    if rate_data:
        currency_data = {
            "from_currency": rate_data["from_currency"],
            "to_currency": rate_data["to_currency"],
            "rate": rate_data["rate"],
            "date": rate_data.get("date"),
        }

        # If an amount was mentioned, calculate the conversion
        if amount is not None:
            converted_amount = amount * rate_data["rate"]
            currency_data["original_amount"] = amount
            currency_data["converted_amount"] = round(converted_amount, 2)
    else:
        logger.warning(
            f"Could not fetch currency rate for {from_currency} to {to_currency}"
        )
        currency_data = {
            "from_currency": from_currency,
            "to_currency": to_currency,
            "rate": None,
            "error": "Could not retrieve exchange rate",
        }

    # Create system message with currency data
    currency_context = f"""
Current currency exchange data:

{json.dumps(currency_data, indent=2)}

Use this data to answer the user's currency exchange query.
"""

    response_messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "assistant", "content": currency_context},
    ]

    # Add conversation history
    for msg in request_body.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    return response_messages


@app.post("/v1/chat/completions")
async def chat_completion_http(request: Request, request_body: ChatCompletionRequest):
    """HTTP endpoint for chat completions with streaming support."""
    logger.info(f"Received currency request with {len(request_body.messages)} messages")

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
    # Prepare messages with currency exchange data
    response_messages = await prepare_currency_messages(request_body)

    try:
        logger.info(
            f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate currency response"
        )

        # Prepare extra headers
        extra_headers = {"x-envoy-max-retries": "3"}
        if traceparent_header:
            extra_headers["traceparent"] = traceparent_header

        response_stream = await archgw_client.chat.completions.create(
            model=CURRENCY_MODEL,
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
        logger.error(f"Error generating currency response: {e}")

        error_chunk = ChatCompletionStreamResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "delta": {
                        "content": "I apologize, but I'm having trouble generating a currency exchange response right now. Please try again."
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
    return {"status": "healthy", "agent": "currency_exchange"}


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=10530)


def start_server(host: str = "localhost", port: int = 10530):
    """Start the currency agent server."""
    uvicorn.run(
        app,
        host=host,
        port=port,
        log_config={
            "version": 1,
            "disable_existing_loggers": False,
            "formatters": {
                "default": {
                    "format": "%(asctime)s - [CURRENCY_AGENT] - %(levelname)s - %(message)s",
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
