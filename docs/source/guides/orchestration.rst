.. _agent_routing:

Orchestration
==============

Building multi-agent systems allow you to route requests across multiple specialized agents, each designed to handle specific types of tasks.
Plano makes it easy to build and scale these systems by managing the orchestration layer—deciding which agent(s) should handle each request—while you focus on implementing individual agent logic.

This guide shows you how to configure and implement multi-agent orchestration in Plano using a real-world example: a **Travel Booking Assistant** that routes queries to specialized agents for weather, flights, and currency exchange.

How It Works
------------

Plano's orchestration layer analyzes incoming prompts and routes them to the most appropriate agent based on user intent and conversation context. The workflow is:

1. **User submits a prompt**: The request arrives at Plano's agent listener.
2. **Agent selection**: Plano analyzes the prompt to determine user intent and complexity, and routes the request to the most suitable agent configured in your system—such as a weather agent, flight agent, or currency agent.
3. **Agent handles request**: The selected agent processes the prompt using its specialized logic and tools.
4. **Seamless handoffs**: For multi-turn conversations, Plano repeats the intent analysis for each follow-up query, enabling smooth handoffs between agents as the conversation evolves.

Example: Travel Booking Assistant
----------------------------------

Let's walk through a complete multi-agent system: a Travel Booking Assistant that helps users plan trips by providing weather forecasts, flight information, and currency exchange rates. This system uses three specialized agents:

* **Weather Agent**: Provides real-time weather conditions and forecasts
* **Flight Agent**: Searches for flights between airports
* **Currency Agent**: Provides currency exchange rates and conversions

Configuration
-------------

Configure your agents in the ``listeners`` section of your ``plano_config.yaml``:

.. code-block:: yaml
    :caption: Travel Booking Multi-Agent Configuration

    version: v0.3.0

    agents:
      - id: weather_agent
        url: http://host.docker.internal:10510
      - id: flight_agent
        url: http://host.docker.internal:10520
      - id: currency_agent
        url: http://host.docker.internal:10530

    model_providers:
      - model: openai/gpt-4o
        access_key: $OPENAI_API_KEY
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_API_KEY

    listeners:
      - type: agent
        name: travel_booking_service
        port: 8001
        router: plano_orchestrator_v1
        agents:
          - id: weather_agent
            description: Get real-time weather conditions and multi-day forecasts for any city worldwide using Open-Meteo API (free, no API key needed). Provides current temperature, multi-day forecasts, weather conditions, sunrise/sunset times, and detailed weather information. Understands conversation context to resolve location references from previous messages.

          - id: flight_agent
            description: Get live flight information between airports using FlightAware AeroAPI. Shows real-time flight status, scheduled/estimated/actual departure and arrival times, gate and terminal information, delays, aircraft type, and flight status. Automatically resolves city names to airport codes (IATA/ICAO). Understands conversation context to infer origin/destination from follow-up questions. Supports queries like "What flights go from London to Seattle?" or "Do they fly out from Seattle?" (using context from previous messages).

          - id: currency_agent
            description: Get real-time currency exchange rates and perform currency conversions using Frankfurter API (free, no API key needed). Provides latest exchange rates, currency conversions with amount calculations, and supports any currency pair. Automatically extracts currency codes from country names and conversation context. Understands pronouns like "their currency" when referring to previously mentioned countries. Uses standard 3-letter ISO currency codes (e.g., USD, EUR, GBP, JPY, PKR).

    tracing:
      random_sampling: 100

**Key Configuration Elements:**

* **agent listener**: A listener of ``type: agent`` tells Plano to perform intent analysis and routing for incoming requests.
* **agents list**: Define each agent with an ``id``, ``description`` (used for routing decisions)
* **router**: The ``plano_orchestrator_v1`` router uses Plano-Orchestrator to analyze user intent and select the appropriate agent.
* **filter_chain**: Optionally attach :ref:`filter chains <filter_chain>` to agents for guardrails, query rewriting, or context enrichment.

**Writing Effective Agent Descriptions**

Agent descriptions are critical—they're used by Plano-Orchestrator to make routing decisions. Be specific about:

* What the agent does (e.g., "Get real-time weather conditions")
* What APIs or tools it uses (e.g., "using Open-Meteo API")
* What information it provides (e.g., "current temperature, multi-day forecasts")
* How it handles context (e.g., "Understands conversation context to resolve location references")

Implementation
--------------

Agents are HTTP services that receive routed requests from Plano. Each agent implements the OpenAI Chat Completions API format, making them compatible with standard LLM clients.

Agent Structure
^^^^^^^^^^^^^^^

Let's examine the Weather Agent implementation:

.. code-block:: python
    :caption: Weather Agent - Core Structure

    from fastapi import FastAPI, Request
    from fastapi.responses import StreamingResponse
    from openai import AsyncOpenAI
    import os
    import httpx
    from .api import ChatCompletionRequest, ChatCompletionStreamResponse

    # Configuration
    PLANO_ENDPOINT = os.getenv("PLANO_ENDPOINT", "http://localhost:12000/v1")
    WEATHER_MODEL = "openai/gpt-4o"
    LOCATION_MODEL = "openai/gpt-4o-mini"

    # Initialize OpenAI client for Plano
    plano = AsyncOpenAI(
        base_url=PLANO_ENDPOINT,
        api_key="EMPTY",
    )

    app = FastAPI(title="Weather Forecast Agent")

    @app.post("/v1/chat/completions")
    async def chat_completion_http(request: Request, request_body: ChatCompletionRequest):
        """HTTP endpoint for chat completions with streaming support."""
        return StreamingResponse(
            stream_chat_completions(request_body),
            media_type="text/event-stream",
        )

**Key Points:**

* Agents expose a ``/v1/chat/completions`` endpoint that matches OpenAI's API format
* They use Plano's LLM gateway (via ``PLANO_ENDPOINT``) for all LLM calls
* They receive the full conversation history in ``request_body.messages``

Information Extraction with LLMs
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Agents use LLMs to extract structured information from natural language queries. This enables them to understand user intent and extract parameters needed for API calls.

The Weather Agent extracts location information:

.. code-block:: python
    :caption: Weather Agent - Location Extraction

    LOCATION_EXTRACTION_PROMPT = """You are a location extraction assistant. Your ONLY job is to extract the geographic location (city, state, country, etc.) from user messages.

    CRITICAL RULES:
    1. Extract ONLY the location name - nothing else
    2. Return just the location name in plain text (e.g., "London", "New York", "Paris, France")
    3. If the user mentions multiple locations, extract the PRIMARY location they're asking about
    4. Ignore error messages, HTML tags, and assistant responses
    5. If no clear location is found, return exactly: "NOT_FOUND"
    """

    async def extract_location_from_messages(messages):
        """Extract location from user messages using LLM."""
        user_messages = [msg for msg in messages if msg.role == "user"]

        if not user_messages:
            return "New York"  # Default fallback

        # Get the most recent user message
        user_content = user_messages[-1].content.strip()

        response = await plano.chat.completions.create(
            model=LOCATION_MODEL,
            messages=[
                {"role": "system", "content": LOCATION_EXTRACTION_PROMPT},
                {"role": "user", "content": user_content},
            ],
            temperature=0.1,
            max_tokens=50,
        )

        location = response.choices[0].message.content.strip()
        return location if location != "NOT_FOUND" else "New York"

The Flight Agent extracts more complex information—origin, destination, and dates:

.. code-block:: python
    :caption: Flight Agent - Flight Information Extraction

    FLIGHT_EXTRACTION_PROMPT = """You are a flight information extraction assistant. Extract flight-related information from user messages and convert it to structured data.

    CRITICAL RULES:
    1. Extract origin city/airport and destination city/airport from the message AND conversation context
    2. Extract any mentioned dates or time references
    3. PAY ATTENTION TO CONVERSATION CONTEXT - THIS IS CRITICAL:
       - If previous messages mention cities/countries, use that context to resolve pronouns and incomplete queries
       - Example: Previous: "What's the weather in Istanbul?" → Current: "Do they fly out from Seattle?"
         → This likely means: origin=Istanbul, destination=Seattle
    4. Return your response as a JSON object:
       {
         "origin": "London" or null,
         "destination": "Seattle" or null,
         "date": "2025-12-20" or null,
         "origin_airport_code": "LHR" or null,
         "destination_airport_code": "SEA" or null
       }
    """

    async def extract_flight_info_from_messages(messages):
        """Extract flight information from user messages using LLM, considering conversation context."""
        # Build conversation context from all messages
        conversation_context = []
        for msg in messages:
            content = msg.content.strip()
            # Skip error messages and HTML tags
            if not any(pattern in content.lower() for pattern in ["<", ">", "error:"]):
                conversation_context.append({"role": msg.role, "content": content})

        # Use last 10 messages for context
        context_messages = conversation_context[-10:] if len(conversation_context) > 10 else conversation_context

        llm_messages = [
            {"role": "system", "content": FLIGHT_EXTRACTION_PROMPT}
        ] + context_messages

        response = await plano.chat.completions.create(
            model=FLIGHT_EXTRACTION_MODEL,
            messages=llm_messages,
            temperature=0.1,
            max_tokens=300,
        )

        extracted_text = response.choices[0].message.content.strip()
        # Parse JSON from response
        flight_info = json.loads(extracted_text)
        return flight_info

**Key Points:**

* Use smaller, faster models (like ``gpt-4o-mini``) for extraction tasks
* Include conversation context to handle follow-up questions and pronouns
* Use structured prompts with clear output formats (JSON)
* Handle edge cases with fallback values

Calling External APIs
^^^^^^^^^^^^^^^^^^^^^^

After extracting information, agents call external APIs to fetch real-time data:

.. code-block:: python
    :caption: Weather Agent - External API Call

    async def get_weather_data(location: str, days: int = 1):
        """Get live weather data for a location using Open-Meteo API."""
        # Step 1: Geocode location to get coordinates
        geocode_result = await geocode_city(location)
        if not geocode_result:
            return {"error": "Could not find location"}

        latitude = geocode_result["latitude"]
        longitude = geocode_result["longitude"]

        # Step 2: Fetch weather data
        url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={latitude}&"
            f"longitude={longitude}&"
            f"daily=temperature_2m_max,temperature_2m_min,weather_code&"
            f"forecast_days={days}&"
            f"timezone=auto"
        )

        response = await http_client.get(url)
        weather_data = response.json()

        # Step 3: Transform API response into structured format
        forecast = []
        for i in range(days):
            forecast.append({
                "date": daily_data["time"][i],
                "temperature_max_c": daily_data["temperature_2m_max"][i],
                "temperature_min_c": daily_data["temperature_2m_min"][i],
                "condition": weather_code_to_condition(daily_data["weather_code"][i]),
            })

        return {"location": location, "forecast": forecast}

The Flight Agent calls FlightAware's AeroAPI:

.. code-block:: python
    :caption: Flight Agent - External API Call

    async def get_flights_between_airports(
        origin_code: str, dest_code: str, start_date: str = None
    ):
        """Get flights between two airports using FlightAware AeroAPI."""
        url = f"{AEROAPI_BASE_URL}/airports/{origin_code}/flights/to/{dest_code}"
        headers = {"x-apikey": AEROAPI_KEY}

        params = {"connection": "nonstop", "max_pages": 1}
        if start_date:
            params["start"] = start_date

        response = await http_client.get(url, headers=headers, params=params)
        data = response.json()

        # Transform API response
        flights = []
        for flight_group in data.get("flights", []):
            segment = flight_group.get("segments", [])[0]
            flights.append({
                "ident": segment.get("ident"),
                "operator": segment.get("operator"),
                "origin": segment.get("origin", {}).get("city"),
                "destination": segment.get("destination", {}).get("city"),
                "scheduled_out": segment.get("scheduled_out"),
                "status": segment.get("status"),
            })

        return {"flights": flights}

**Key Points:**

* Use async HTTP clients (like ``httpx.AsyncClient``) for non-blocking API calls
* Transform external API responses into consistent, structured formats
* Handle errors gracefully with fallback values
* Cache or validate data when appropriate (e.g., airport code validation)

Preparing Context and Generating Responses
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Agents combine extracted information, API data, and conversation history to generate responses:

.. code-block:: python
    :caption: Weather Agent - Context Preparation and Response Generation

    SYSTEM_PROMPT = """You are a professional weather information assistant. Your role is to provide accurate, clear, and helpful weather information based on the structured weather data provided to you.

    CRITICAL INSTRUCTIONS:
    1. You will receive weather data as JSON in a system message
    2. Present temperatures in both Celsius and Fahrenheit when available
    3. Use natural, conversational language
    4. Never invent or guess weather data - only use what's provided
    """

    async def prepare_weather_messages(request_body: ChatCompletionRequest):
        """Prepare messages with weather data."""
        # Step 1: Extract location from conversation
        location = await extract_location_from_messages(request_body.messages)

        # Step 2: Determine if user wants forecast (multi-day)
        last_user_msg = ""
        for msg in reversed(request_body.messages):
            if msg.role == "user":
                last_user_msg = msg.content.lower()
                break

        days = 5 if "forecast" in last_user_msg or "week" in last_user_msg else 1

        # Step 3: Fetch weather data
        weather_data = await get_weather_data(location, days)

        # Step 4: Build context message with structured data
        weather_context = f"""
    Current weather data for {weather_data['location']}:

    {json.dumps(weather_data, indent=2)}

    Use this data to answer the user's weather query.
    """

        # Step 5: Combine system prompt, context, and conversation history
        response_messages = [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "system", "content": weather_context},
        ]

        # Add conversation history
        for msg in request_body.messages:
            response_messages.append({"role": msg.role, "content": msg.content})

        return response_messages

    async def stream_chat_completions(request_body: ChatCompletionRequest):
        """Generate streaming chat completions."""
        # Prepare messages with weather data
        response_messages = await prepare_weather_messages(request_body)

        # Call Plano's LLM gateway
        response_stream = await plano.chat.completions.create(
            model=WEATHER_MODEL,
            messages=response_messages,
            temperature=request_body.temperature or 0.7,
            max_tokens=request_body.max_tokens or 1000,
            stream=True,
        )

        # Stream response chunks
        async for chunk in response_stream:
            if chunk.choices and chunk.choices[0].delta.content:
                content = chunk.choices[0].delta.content
                yield f"data: {chunk.model_dump_json()}\n\n"

        yield "data: [DONE]\n\n"

**Key Points:**

* Use system messages to provide structured data to the LLM
* Include full conversation history for context-aware responses
* Stream responses for better user experience
* Route all LLM calls through Plano's gateway for consistent behavior and observability

Handling Conversation Context
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

One of the most powerful features of multi-agent systems is handling follow-up questions that reference previous context. The Flight Agent demonstrates this:

.. code-block:: python
    :caption: Flight Agent - Context-Aware Follow-up Questions

    # User: "What's the weather in Istanbul?"
    # Plano routes to weather_agent → Response: "Istanbul is sunny, 25°C"

    # User: "Do they fly out from Seattle?"
    # Plano routes to flight_agent → Agent needs to infer:
    #   - "they" refers to Istanbul (from previous message)
    #   - User is asking: "Do flights go from Istanbul to Seattle?"

    async def extract_flight_info_from_messages(messages):
        """Extract flight information using conversation context."""
        # Build full conversation context
        conversation_context = []
        for msg in messages:
            conversation_context.append({"role": msg.role, "content": msg.content})

        # Use context-aware extraction prompt
        llm_messages = [
            {"role": "system", "content": FLIGHT_EXTRACTION_PROMPT}
        ] + conversation_context[-10:]  # Last 10 messages for context

        response = await plano.chat.completions.create(
            model=FLIGHT_EXTRACTION_MODEL,
            messages=llm_messages,
            temperature=0.1,
        )

        # LLM extracts: {"origin": "Istanbul", "destination": "Seattle"}
        flight_info = json.loads(response.choices[0].message.content)
        return flight_info

**Key Points:**

* Include conversation history in extraction prompts
* Use pronouns and references from context (e.g., "they", "there", "that country")
* Limit context window to recent messages (e.g., last 10) for efficiency
* Handle cases where context is insufficient gracefully

How Requests Flow
-----------------

Here's how a complete request flows through the Travel Booking system:

1. **User sends a prompt**: "What's the weather in London?"

2. **Plano analyzes intent**: Plano-Orchestrator analyzes the prompt and determines it should route to ``weather_agent`` based on the agent's description.

3. **Plano forwards request**: Plano sends the request to the correct agent with ``/v1/chat/completions``, along with the full conversation history.

4. **Weather Agent processes**:
   a. Extracts location "London" from the message
   b. Calls Open-Meteo API to get weather data
   c. Prepares context with weather data
   d. Calls Plano's LLM gateway to generate response
   e. Streams response back to Plano

5. **Plano forwards response**: Plano streams the agent's response back to the user.

6. **Follow-up question**: User asks "Do they fly out from Seattle?"

7. **Plano routes again**: Plano-Orchestrator analyzes the new prompt and routes to ``flight_agent``.

8. **Flight Agent processes**:
   a. Extracts flight info using conversation context (infers "they" = London)
   b. Resolves airport codes (London → LHR, Seattle → SEA)
   c. Calls FlightAware API to get flights
   d. Generates response with flight information

9. **Seamless handoff**: The user experiences a smooth transition between agents without needing to repeat context.

Best Practices
--------------

**Write Clear Agent Descriptions**

Agent descriptions are used by Plano-Orchestrator to make routing decisions. Be specific about what each agent handles:

.. code-block:: yaml

    # Good - specific and actionable
    - id: flight_agent
      description: Get live flight information between airports using FlightAware AeroAPI. Shows real-time flight status, scheduled/estimated/actual departure and arrival times, gate and terminal information, delays, aircraft type, and flight status. Automatically resolves city names to airport codes (IATA/ICAO). Understands conversation context to infer origin/destination from follow-up questions.

    # Less ideal - too vague
    - id: flight_agent
      description: Handles flight queries

**Use Conversation Context Effectively**

Include conversation history in your extraction and response generation:

.. code-block:: python

    # Include conversation context for extraction
    conversation_context = []
    for msg in messages:
        conversation_context.append({"role": msg.role, "content": msg.content})

    # Use recent context (last 10 messages)
    context_messages = conversation_context[-10:] if len(conversation_context) > 10 else conversation_context

**Route LLM Calls Through Plano's Model Proxy**

Always route LLM calls through Plano's :ref:`Model Proxy <llm_providers>` for consistent responses, smart routing, and rich observability:

.. code-block:: python

    plano = AsyncOpenAI(
        base_url=PLANO_ENDPOINT,  # Plano's LLM gateway
        api_key="EMPTY",
    )

    response = await plano.chat.completions.create(
        model="openai/gpt-4o",
        messages=messages,
        stream=True,
    )

**Handle Errors Gracefully**

Provide fallback values and clear error messages:

.. code-block:: python

    async def extract_location_from_messages(messages):
        try:
            # ... extraction logic ...
            return location
        except Exception as e:
            logger.error(f"Error extracting location: {e}")
            return "New York"  # Fallback to default

**Use Appropriate Models for Tasks**

Use smaller, faster models for extraction tasks and larger models for final responses:

.. code-block:: python

    # Extraction: Use smaller, faster model
    LOCATION_MODEL = "openai/gpt-4o-mini"

    # Final response: Use larger, more capable model
    WEATHER_MODEL = "openai/gpt-4o"

**Stream Responses**

Stream responses for better user experience:

.. code-block:: python

    async def stream_chat_completions(request_body):
        response_stream = await plano.chat.completions.create(
            model=WEATHER_MODEL,
            messages=messages,
            stream=True,
        )

        async for chunk in response_stream:
            if chunk.choices and chunk.choices[0].delta.content:
                yield f"data: {chunk.model_dump_json()}\n\n"

Common Use Cases
----------------

Multi-agent orchestration is particularly powerful for:

**Travel and Booking Systems**

Route queries to specialized agents for weather, flights, hotels, and currency:

.. code-block:: yaml

    agents:
      - id: weather_agent
        description: Get real-time weather conditions and forecasts
      - id: flight_agent
        description: Search for flights and provide flight status
      - id: hotel_agent
        description: Find hotels and check availability
      - id: currency_agent
        description: Provide currency exchange rates

**Customer Support**

Route common queries to automated support agents while escalating complex issues:

.. code-block:: yaml

    agents:
      - id: tier1_support
        description: Handles common FAQs, password resets, and basic troubleshooting
      - id: tier2_support
        description: Handles complex technical issues requiring deep product knowledge
      - id: human_escalation
        description: Escalates sensitive issues or unresolved problems to human agents

**Sales and Marketing**

Direct leads and inquiries to specialized sales agents:

.. code-block:: yaml

    agents:
      - id: product_recommendation
        description: Recommends products based on user needs and preferences
      - id: pricing_agent
        description: Provides pricing information and quotes
      - id: sales_closer
        description: Handles final negotiations and closes deals

**Technical Documentation and Support**

Combine RAG agents for documentation lookup with specialized troubleshooting agents:

.. code-block:: yaml

    agents:
      - id: docs_agent
        description: Retrieves relevant documentation and guides
        filter_chain:
          - query_rewriter
          - context_builder
      - id: troubleshoot_agent
        description: Diagnoses and resolves technical issues step by step

Next Steps
----------

* Learn more about :ref:`agents <agents>` and the inner vs. outer loop model
* Explore :ref:`filter chains <filter_chain>` for adding guardrails and context enrichment
* See :ref:`observability <observability>` for monitoring multi-agent workflows
* Review the :ref:`LLM Providers <llm_providers>` guide for model routing within agents
* Check out the complete `Travel Booking demo <https://github.com/katanemo/plano/tree/main/demos/use_cases/travel_booking>`_ on GitHub

.. note::
    To observe traffic to and from agents, please read more about :ref:`observability <observability>` in Plano.

By carefully configuring and managing your Agent routing and hand off, you can significantly improve your application's responsiveness, performance, and overall user satisfaction.
