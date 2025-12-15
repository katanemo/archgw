import openai
import pytest
import os
import logging
import sys

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)
logger = logging.getLogger(__name__)

LLM_GATEWAY_ENDPOINT = os.getenv(
    "LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1/chat/completions"
)


# -----------------------
# v1/responses API tests
# -----------------------
def test_openai_responses_api_non_streaming_passthrough():
    """Build a v1/responses API request (pass-through) and ensure gateway accepts it"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    # Simple responses API request using a direct model (pass-through)
    resp = client.responses.create(
        model="gpt-4o", input="Hello via responses passthrough"
    )

    # Print the response content - handle both responses format and chat completions format
    print(f"\n{'='*80}")
    print(f"Model: {resp.model}")
    print(f"Output: {resp.output_text}")
    print(f"{'='*80}\n")

    # Minimal sanity checks
    assert resp is not None
    assert (
        getattr(resp, "id", None) is not None
        or getattr(resp, "output", None) is not None
    )


def test_openai_responses_api_with_streaming_passthrough():
    """Build a v1/responses API streaming request (pass-through) and ensure gateway accepts it"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    # Simple streaming responses API request using a direct model (pass-through)
    stream = client.responses.create(
        model="gpt-4o",
        input="Write a short haiku about coding",
        stream=True,
    )

    # Collect streamed content using the official Responses API streaming shape
    text_chunks = []
    final_message = None

    for event in stream:
        # The Python SDK surfaces a high-level Responses streaming interface.
        # We rely on its typed helpers instead of digging into model_extra.
        if getattr(event, "type", None) == "response.output_text.delta" and getattr(
            event, "delta", None
        ):
            # Each delta contains a text fragment
            text_chunks.append(event.delta)

        # Track the final response message if provided by the SDK
        if getattr(event, "type", None) == "response.completed" and getattr(
            event, "response", None
        ):
            final_message = event.response

    full_content = "".join(text_chunks)

    # Print the streaming response
    print(f"\n{'='*80}")
    print(
        f"Model: {getattr(final_message, 'model', 'unknown') if final_message else 'unknown'}"
    )
    print(f"Streamed Output: {full_content}")
    print(f"{'='*80}\n")

    assert len(text_chunks) > 0, "Should have received streaming text deltas"
    assert len(full_content) > 0, "Should have received content"


def test_openai_responses_api_non_streaming_with_tools_passthrough():
    """Responses API with a function/tool definition (pass-through)"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1", max_retries=0)

    # Define a simple tool/function for the Responses API
    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    resp = client.responses.create(
        model="gpt-5",
        input="Call the echo tool",
        tools=tools,
    )

    assert resp is not None
    assert (
        getattr(resp, "id", None) is not None
        or getattr(resp, "output", None) is not None
    )


def test_openai_responses_api_with_streaming_with_tools_passthrough():
    """Responses API with a function/tool definition (streaming, pass-through)"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1", max_retries=0)

    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    stream = client.responses.create(
        model="gpt-5",
        input="Call the echo tool",
        tools=tools,
        stream=True,
    )

    text_chunks = []
    tool_calls = []

    for event in stream:
        etype = getattr(event, "type", None)

        # Collect streamed text output
        if etype == "response.output_text.delta" and getattr(event, "delta", None):
            text_chunks.append(event.delta)

        # Collect streamed tool call arguments
        if etype == "response.function_call_arguments.delta" and getattr(
            event, "delta", None
        ):
            tool_calls.append(event.delta)

    full_text = "".join(text_chunks)

    print(f"\n{'='*80}")
    print("Responses tools streaming test")
    print(f"Streamed text: {full_text}")
    print(f"Tool call argument chunks: {len(tool_calls)}")
    print(f"{'='*80}\n")

    # We expect either streamed text output or streamed tool-call arguments
    assert (
        full_text or tool_calls
    ), "Expected streamed text or tool call argument deltas from Responses tools stream"


def test_openai_responses_api_non_streaming_upstream_chat_completions():
    """Send a v1/responses request using the grok alias to verify translation/routing"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    resp = client.responses.create(
        model="arch.grok.v1", input="Hello, translate this via grok alias"
    )

    # Print the response content - handle both responses format and chat completions format
    print(f"\n{'='*80}")
    print(f"Model: {resp.model}")
    print(f"Output: {resp.output_text}")
    print(f"{'='*80}\n")

    assert resp is not None
    assert resp.id is not None


def test_openai_responses_api_with_streaming_upstream_chat_completions():
    """Build a v1/responses API streaming request (pass-through) and ensure gateway accepts it"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    # Simple streaming responses API request using a direct model (pass-through)
    stream = client.responses.create(
        model="arch.grok.v1",
        input="Write a short haiku about coding",
        stream=True,
    )

    # Collect streamed content using the official Responses API streaming shape
    text_chunks = []
    final_message = None

    for event in stream:
        # The Python SDK surfaces a high-level Responses streaming interface.
        # We rely on its typed helpers instead of digging into model_extra.
        if getattr(event, "type", None) == "response.output_text.delta" and getattr(
            event, "delta", None
        ):
            # Each delta contains a text fragment
            text_chunks.append(event.delta)

        # Track the final response message if provided by the SDK
        if getattr(event, "type", None) == "response.completed" and getattr(
            event, "response", None
        ):
            final_message = event.response

    full_content = "".join(text_chunks)

    # Print the streaming response
    print(f"\n{'='*80}")
    print(
        f"Model: {getattr(final_message, 'model', 'unknown') if final_message else 'unknown'}"
    )
    print(f"Streamed Output: {full_content}")
    print(f"{'='*80}\n")

    assert len(text_chunks) > 0, "Should have received streaming text deltas"
    assert len(full_content) > 0, "Should have received content"


def test_openai_responses_api_non_streaming_with_tools_upstream_chat_completions():
    """Responses API wioutputling routed to grok via alias"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    resp = client.responses.create(
        model="arch.grok.v1",
        input="Call the echo tool",
        tools=tools,
    )

    assert resp.id is not None

    print(f"\n{'='*80}")
    print(f"Model: {resp.model}")
    print(f"Output: {resp.output_text}")
    print(f"{'='*80}\n")


def test_openai_responses_api_streaming_with_tools_upstream_chat_completions():
    """Responses API with a function/tool definition (streaming, pass-through)"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1", max_retries=0)

    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    stream = client.responses.create(
        model="arch.grok.v1",
        input="Call the echo tool",
        tools=tools,
        stream=True,
    )

    text_chunks = []
    tool_calls = []

    for event in stream:
        etype = getattr(event, "type", None)

        # Collect streamed text output
        if etype == "response.output_text.delta" and getattr(event, "delta", None):
            text_chunks.append(event.delta)

        # Collect streamed tool call arguments
        if etype == "response.function_call_arguments.delta" and getattr(
            event, "delta", None
        ):
            tool_calls.append(event.delta)

    full_text = "".join(text_chunks)

    print(f"\n{'='*80}")
    print("Responses tools streaming test")
    print(f"Streamed text: {full_text}")
    print(f"Tool call argument chunks: {len(tool_calls)}")
    print(f"{'='*80}\n")

    # We expect either streamed text output or streamed tool-call arguments
    assert (
        full_text or tool_calls
    ), "Expected streamed text or tool call argument deltas from Responses tools stream"


def test_openai_responses_api_non_streaming_upstream_bedrock():
    """Send a v1/responses request using the coding-model alias to verify Bedrock translation/routing"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    resp = client.responses.create(
        model="coding-model",
        input="Hello, translate this via coding-model alias to Bedrock",
    )

    # Print the response content - handle both responses format and chat completions format
    print(f"\n{'='*80}")
    print(f"Model: {resp.model}")
    print(f"Output: {resp.output_text}")
    print(f"{'='*80}\n")

    assert resp is not None
    assert resp.id is not None


def test_openai_responses_api_with_streaming_upstream_bedrock():
    """Build a v1/responses API streaming request routed to Bedrock via coding-model alias"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    # Simple streaming responses API request using coding-model alias
    stream = client.responses.create(
        model="coding-model",
        input="Write a short haiku about coding",
        stream=True,
    )

    # Collect streamed content using the official Responses API streaming shape
    text_chunks = []
    final_message = None

    for event in stream:
        # The Python SDK surfaces a high-level Responses streaming interface.
        # We rely on its typed helpers instead of digging into model_extra.
        if getattr(event, "type", None) == "response.output_text.delta" and getattr(
            event, "delta", None
        ):
            # Each delta contains a text fragment
            text_chunks.append(event.delta)

        # Track the final response message if provided by the SDK
        if getattr(event, "type", None) == "response.completed" and getattr(
            event, "response", None
        ):
            final_message = event.response

    full_content = "".join(text_chunks)

    # Print the streaming response
    print(f"\n{'='*80}")
    print(
        f"Model: {getattr(final_message, 'model', 'unknown') if final_message else 'unknown'}"
    )
    print(f"Streamed Output: {full_content}")
    print(f"{'='*80}\n")

    assert len(text_chunks) > 0, "Should have received streaming text deltas"
    assert len(full_content) > 0, "Should have received content"


def test_openai_responses_api_non_streaming_with_tools_upstream_bedrock():
    """Responses API with tools routed to Bedrock via coding-model alias"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    resp = client.responses.create(
        model="coding-model",
        input="Call the echo tool",
        tools=tools,
    )

    assert resp.id is not None

    print(f"\n{'='*80}")
    print(f"Model: {resp.model}")
    print(f"Output: {resp.output_text}")
    print(f"{'='*80}\n")


def test_openai_responses_api_streaming_with_tools_upstream_bedrock():
    """Responses API with a function/tool definition streaming to Bedrock via coding-model alias"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1", max_retries=0)

    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    stream = client.responses.create(
        model="coding-model",
        input="Call the echo tool",
        tools=tools,
        stream=True,
    )

    text_chunks = []
    tool_calls = []

    for event in stream:
        etype = getattr(event, "type", None)

        # Collect streamed text output
        if etype == "response.output_text.delta" and getattr(event, "delta", None):
            text_chunks.append(event.delta)

        # Collect streamed tool call arguments
        if etype == "response.function_call_arguments.delta" and getattr(
            event, "delta", None
        ):
            tool_calls.append(event.delta)

    full_text = "".join(text_chunks)

    print(f"\n{'='*80}")
    print("Responses tools streaming test (Bedrock)")
    print(f"Streamed text: {full_text}")
    print(f"Tool call argument chunks: {len(tool_calls)}")
    print(f"{'='*80}\n")

    # We expect either streamed text output or streamed tool-call arguments
    assert (
        full_text or tool_calls
    ), "Expected streamed text or tool call argument deltas from Responses tools stream"


def test_openai_responses_api_non_streaming_upstream_anthropic():
    """Send a v1/responses request using the grok alias to verify translation/routing"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    resp = client.responses.create(
        model="claude-sonnet-4-20250514", input="Hello, translate this via grok alias"
    )

    # Print the response content - handle both responses format and chat completions format
    print(f"\n{'='*80}")
    print(f"Model: {resp.model}")
    print(f"Output: {resp.output_text}")
    print(f"{'='*80}\n")

    assert resp is not None
    assert resp.id is not None


def test_openai_responses_api_with_streaming_upstream_anthropic():
    """Build a v1/responses API streaming request (pass-through) and ensure gateway accepts it"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    # Simple streaming responses API request using a direct model (pass-through)
    stream = client.responses.create(
        model="claude-sonnet-4-20250514",
        input="Write a short haiku about coding",
        stream=True,
    )

    # Collect streamed content using the official Responses API streaming shape
    text_chunks = []
    final_message = None

    for event in stream:
        # The Python SDK surfaces a high-level Responses streaming interface.
        # We rely on its typed helpers instead of digging into model_extra.
        if getattr(event, "type", None) == "response.output_text.delta" and getattr(
            event, "delta", None
        ):
            # Each delta contains a text fragment
            text_chunks.append(event.delta)

        # Track the final response message if provided by the SDK
        if getattr(event, "type", None) == "response.completed" and getattr(
            event, "response", None
        ):
            final_message = event.response

    full_content = "".join(text_chunks)

    # Print the streaming response
    print(f"\n{'='*80}")
    print(
        f"Model: {getattr(final_message, 'model', 'unknown') if final_message else 'unknown'}"
    )
    print(f"Streamed Output: {full_content}")
    print(f"{'='*80}\n")

    assert len(text_chunks) > 0, "Should have received streaming text deltas"
    assert len(full_content) > 0, "Should have received content"


def test_openai_responses_api_non_streaming_with_tools_upstream_anthropic():
    """Responses API with tools routed to grok via alias"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input: hello_world",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    resp = client.responses.create(
        model="claude-sonnet-4-20250514",
        input="Call the echo tool",
        tools=tools,
    )

    assert resp.id is not None

    print(f"\n{'='*80}")
    print(f"Model: {resp.model}")
    print(f"Output: {resp.output_text}")
    print(f"{'='*80}\n")


def test_openai_responses_api_streaming_with_tools_upstream_anthropic():
    """Responses API with a function/tool definition (streaming, pass-through)"""
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1", max_retries=0)

    tools = [
        {
            "type": "function",
            "name": "echo_tool",
            "description": "Echo back the provided input: hello_world",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }
    ]

    stream = client.responses.create(
        model="claude-sonnet-4-20250514",
        input="Call the echo tool",
        tools=tools,
        stream=True,
    )

    text_chunks = []
    tool_calls = []

    for event in stream:
        etype = getattr(event, "type", None)

        # Collect streamed text output
        if etype == "response.output_text.delta" and getattr(event, "delta", None):
            text_chunks.append(event.delta)

        # Collect streamed tool call arguments
        if etype == "response.function_call_arguments.delta" and getattr(
            event, "delta", None
        ):
            tool_calls.append(event.delta)

    full_text = "".join(text_chunks)

    print(f"\n{'='*80}")
    print("Responses tools streaming test")
    print(f"Streamed text: {full_text}")
    print(f"Tool call argument chunks: {len(tool_calls)}")
    print(f"{'='*80}\n")

    # We expect either streamed text output or streamed tool-call arguments
    assert (
        full_text or tool_calls
    ), "Expected streamed text or tool call argument deltas from Responses tools stream"


def test_conversation_state_management_two_turn():
    """
    Test conversation state management across two turns:
    1. Send initial message to non-OpenAI model via v1/responses
    2. Capture response_id from first response
    3. Send second message with previous_response_id
    4. Verify model receives both messages in correct order
    """
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    logger.info("\n" + "=" * 80)
    logger.info("TEST: Conversation State Management - Two Turn Flow")
    logger.info("=" * 80)

    # Turn 1: Send initial message to Anthropic (non-OpenAI model)
    logger.info("\n[TURN 1] Sending initial message...")
    resp1 = client.responses.create(
        model="claude-sonnet-4-20250514",
        input="My name is Alice and I like pizza.",
    )

    # Extract response_id from first response
    response_id_1 = resp1.id
    logger.info(f"[TURN 1] Received response_id: {response_id_1}")
    logger.info(f"[TURN 1] Model response: {resp1.output_text}")

    assert response_id_1 is not None, "First response should have an id"
    assert len(resp1.output_text) > 0, "First response should have content"

    # Turn 2: Send follow-up message with previous_response_id
    # Ask the model to list all messages to verify state was combined
    logger.info(
        f"\n[TURN 2] Sending follow-up with previous_response_id={response_id_1}"
    )
    resp2 = client.responses.create(
        model="claude-sonnet-4-20250514",
        input="Please list all the messages you have received in our conversation, numbering each one.",
        previous_response_id=response_id_1,
    )

    response_id_2 = resp2.id
    logger.info(f"[TURN 2] Received response_id: {response_id_2}")
    logger.info(f"[TURN 2] Model response: {resp2.output_text}")

    assert response_id_2 is not None, "Second response should have an id"
    assert response_id_2 != response_id_1, "Second response should have different id"

    # Verify the model received the conversation history
    # The response should reference both the initial message and the follow-up
    response_lower = resp2.output_text.lower()

    # Check if the model acknowledges receiving multiple messages
    # Different models might format this differently, so we check for various indicators
    has_conversation_context = (
        "alice" in response_lower
        or "pizza" in response_lower  # References the name from turn 1
        or "two" in response_lower  # References the preference from turn 1
        or "2" in response_lower  # Mentions number of messages
        or "first" in response_lower  # Numeric indicator
        or "second"  # References first message
        in response_lower  # References second message
    )

    logger.info(
        f"\n[VALIDATION] Conversation context preserved: {has_conversation_context}"
    )
    logger.info(
        f"[VALIDATION] Response contains conversation markers: {has_conversation_context}"
    )

    print(f"\n{'='*80}")
    print("Conversation State Test Results:")
    print(f"Turn 1 Response ID: {response_id_1}")
    print(f"Turn 2 Response ID: {response_id_2}")
    print(f"Turn 1 Output: {resp1.output_text[:100]}...")
    print(f"Turn 2 Output: {resp2.output_text}")
    print(f"Conversation Context Preserved: {has_conversation_context}")
    print(f"{'='*80}\n")

    assert has_conversation_context, (
        f"Model should have received conversation history. "
        f"Response: {resp2.output_text}"
    )


def test_conversation_state_management_two_turn_streaming():
    """
    Test conversation state management across two turns with streaming:
    1. Send initial streaming message to non-OpenAI model via v1/responses
    2. Capture response_id from first response
    3. Send second streaming message with previous_response_id
    4. Verify model receives both messages in correct order
    """
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(api_key="test-key", base_url=f"{base_url}/v1")

    logger.info("\n" + "=" * 80)
    logger.info("TEST: Conversation State Management - Two Turn Streaming Flow")
    logger.info("=" * 80)

    # Turn 1: Send initial streaming message to Anthropic (non-OpenAI model)
    logger.info("\n[TURN 1] Sending initial streaming message...")
    stream1 = client.responses.create(
        model="claude-sonnet-4-20250514",
        input="My name is Alice and I like pizza.",
        stream=True,
    )

    # Collect streamed content and capture response_id
    text_chunks_1 = []
    response_id_1 = None

    for event in stream1:
        if getattr(event, "type", None) == "response.output_text.delta" and getattr(
            event, "delta", None
        ):
            text_chunks_1.append(event.delta)

        # Capture response_id from response.completed event
        if getattr(event, "type", None) == "response.completed" and getattr(
            event, "response", None
        ):
            response_id_1 = event.response.id

    output_1 = "".join(text_chunks_1)
    logger.info(f"[TURN 1] Received response_id: {response_id_1}")
    logger.info(f"[TURN 1] Model response: {output_1}")

    assert response_id_1 is not None, "First response should have an id"
    assert len(output_1) > 0, "First response should have content"

    # Turn 2: Send follow-up streaming message with previous_response_id
    logger.info(
        f"\n[TURN 2] Sending follow-up streaming request with previous_response_id={response_id_1}"
    )
    stream2 = client.responses.create(
        model="claude-sonnet-4-20250514",
        input="Please list all the messages you have received in our conversation, numbering each one.",
        previous_response_id=response_id_1,
        stream=True,
    )

    # Collect streamed content from second response
    text_chunks_2 = []
    response_id_2 = None

    for event in stream2:
        if getattr(event, "type", None) == "response.output_text.delta" and getattr(
            event, "delta", None
        ):
            text_chunks_2.append(event.delta)

        # Capture response_id from response.completed event
        if getattr(event, "type", None) == "response.completed" and getattr(
            event, "response", None
        ):
            response_id_2 = event.response.id

    output_2 = "".join(text_chunks_2)
    logger.info(f"[TURN 2] Received response_id: {response_id_2}")
    logger.info(f"[TURN 2] Model response: {output_2}")

    assert response_id_2 is not None, "Second response should have an id"
    assert response_id_2 != response_id_1, "Second response should have different id"

    # Verify the model received the conversation history
    response_lower = output_2.lower()

    # Check if the model acknowledges receiving multiple messages
    has_conversation_context = (
        "alice" in response_lower
        or "pizza" in response_lower  # References the name from turn 1
        or "two" in response_lower  # References the preference from turn 1
        or "2" in response_lower  # Mentions number of messages
        or "first" in response_lower  # Numeric indicator
        or "second"  # References first message
        in response_lower  # References second message
    )

    logger.info(
        f"\n[VALIDATION] Conversation context preserved: {has_conversation_context}"
    )
    logger.info(
        f"[VALIDATION] Response contains conversation markers: {has_conversation_context}"
    )

    print(f"\n{'='*80}")
    print("Streaming Conversation State Test Results:")
    print(f"Turn 1 Response ID: {response_id_1}")
    print(f"Turn 2 Response ID: {response_id_2}")
    print(f"Turn 1 Output: {output_1[:100]}...")
    print(f"Turn 2 Output: {output_2}")
    print(f"Conversation Context Preserved: {has_conversation_context}")
    print(f"{'='*80}\n")

    assert has_conversation_context, (
        f"Model should have received conversation history. " f"Response: {output_2}"
    )
