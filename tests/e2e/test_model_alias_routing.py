import anthropic
import openai
import os
import logging
import pytest
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

# =============================================================================
# MODEL ALIAS TESTS
# =============================================================================


def test_openai_client_with_alias_arch_summarize_v1():
    """Test OpenAI client using model alias 'arch.summarize.v1' which should resolve to '4o-mini'"""
    logger.info("Testing OpenAI client with alias 'arch.summarize.v1' -> '4o-mini'")

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(
        api_key="test-key",
        base_url=f"{base_url}/v1",
    )

    completion = client.chat.completions.create(
        model="arch.summarize.v1",  # This should resolve to 5o-mini
        max_completion_tokens=500,  # Increased token limit to avoid truncation and because the 5o-mini uses reasoning tokens
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from alias arch.summarize.v1!",
            }
        ],
    )

    response_content = completion.choices[0].message.content
    logger.info(f"Response from arch.summarize.v1 alias: {response_content}")
    assert response_content == "Hello from alias arch.summarize.v1!"


def test_openai_client_with_alias_arch_v1():
    """Test OpenAI client using model alias 'arch.v1' which should resolve to 'o3'"""
    logger.info("Testing OpenAI client with alias 'arch.v1' -> 'o3'")

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(
        api_key="test-key",
        base_url=f"{base_url}/v1",
    )

    completion = client.chat.completions.create(
        model="arch.v1",  # This should resolve to gpt-o3
        max_completion_tokens=500,
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from alias arch.v1!",
            }
        ],
    )

    response_content = completion.choices[0].message.content
    logger.info(f"Response from arch.v1 alias: {response_content}")
    assert response_content == "Hello from alias arch.v1!"


def test_anthropic_client_with_alias_arch_summarize_v1():
    """Test Anthropic client using model alias 'arch.summarize.v1' which should resolve to '4o-mini'"""
    logger.info("Testing Anthropic client with alias 'arch.summarize.v1' -> '4o-mini'")

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = anthropic.Anthropic(api_key="test-key", base_url=base_url)

    message = client.messages.create(
        model="arch.summarize.v1",  # This should resolve to 5o-mini
        max_tokens=500,
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from alias arch.summarize.v1 via Anthropic!",
            }
        ],
    )

    response_content = "".join(b.text for b in message.content if b.type == "text")
    logger.info(
        f"Response from arch.summarize.v1 alias via Anthropic: {response_content}"
    )
    assert response_content == "Hello from alias arch.summarize.v1 via Anthropic!"


def test_anthropic_client_with_alias_arch_v1():
    """Test Anthropic client using model alias 'arch.v1' which should resolve to 'o3'"""
    logger.info("Testing Anthropic client with alias 'arch.v1' -> 'o3'")

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = anthropic.Anthropic(api_key="test-key", base_url=base_url)

    message = client.messages.create(
        model="arch.v1",  # This should resolve to o3
        max_tokens=500,
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from alias arch.v1 via Anthropic!",
            }
        ],
    )

    response_content = "".join(b.text for b in message.content if b.type == "text")
    logger.info(f"Response from arch.v1 alias via Anthropic: {response_content}")
    assert response_content == "Hello from alias arch.v1 via Anthropic!"


def test_openai_client_with_alias_streaming():
    """Test OpenAI client using model alias with streaming"""
    logger.info(
        "Testing OpenAI client with alias 'arch.summarize.v1' streaming -> '4o-mini'"
    )

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(
        api_key="test-key",
        base_url=f"{base_url}/v1",
    )

    stream = client.chat.completions.create(
        model="arch.summarize.v1",  # This should resolve to 5o-mini
        max_completion_tokens=500,
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from streaming alias!",
            }
        ],
        stream=True,
    )

    content_chunks = []
    for chunk in stream:
        if chunk.choices[0].delta.content:
            content_chunks.append(chunk.choices[0].delta.content)

    full_content = "".join(content_chunks)
    logger.info(f"Streaming response from arch.summarize.v1 alias: {full_content}")
    assert full_content == "Hello from streaming alias!"


def test_anthropic_client_with_alias_streaming():
    """Test Anthropic client using model alias with streaming"""
    logger.info(
        "Testing Anthropic client with alias 'arch.summarize.v1' streaming -> '4o-mini'"
    )

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = anthropic.Anthropic(api_key="test-key", base_url=base_url)

    with client.messages.stream(
        model="arch.summarize.v1",  # This should resolve to 5o-mini
        max_tokens=500,
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from streaming alias via Anthropic!",
            }
        ],
    ) as stream:
        pieces = [t for t in stream.text_stream]
        full_text = "".join(pieces)

    logger.info(
        f"Streaming response from arch.summarize.v1 alias via Anthropic: {full_text}"
    )
    assert full_text == "Hello from streaming alias via Anthropic!"


def test_400_error_handling_with_alias():
    """Test that 400 errors from upstream are properly returned by archgw"""
    logger.info(
        "Testing 400 error handling with arch.summarize.v1 and invalid parameter"
    )

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(
        api_key="test-key",
        base_url=f"{base_url}/v1",
    )

    try:
        completion = client.chat.completions.create(
            model="arch.summarize.v1",  # This should resolve to gpt-5-mini-2025-08-07
            max_tokens=50,
            messages=[
                {
                    "role": "user",
                    "content": "Hello, this should trigger a 400 error due to invalid parameter name",
                }
            ],
        )
        # If we reach here, the request unexpectedly succeeded
        logger.error(
            f"Expected 400 error but got successful response: {completion.choices[0].message.content}"
        )
        assert False, "Expected 400 error but request succeeded"
    except openai.BadRequestError as e:
        # This is what we expect - a 400 Bad Request error
        logger.info(f"Correctly received 400 Bad Request error: {e}")
        assert e.status_code == 400, f"Expected status code 400, got {e.status_code}"
        logger.info("✓ 400 error handling working correctly")
    except Exception as e:
        # Any other exception is unexpected
        logger.error(
            f"Unexpected error type (should be BadRequestError): {type(e).__name__}: {e}"
        )
        assert False, f"Expected BadRequestError but got {type(e).__name__}: {e}"


def test_400_error_handling_unsupported_parameter():
    """Test that 400 errors for unsupported parameters are properly returned by archgw"""
    logger.info("Testing 400 error handling with unsupported max_tokens parameter")

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(
        api_key="test-key",
        base_url=f"{base_url}/v1",
    )

    try:
        # Use the deprecated max_tokens parameter which should trigger a 400 error
        completion = client.chat.completions.create(
            model="arch.summarize.v1",  # This should resolve to gpt-5-mini-2025-08-07
            max_tokens=150,  # This parameter is unsupported for newer models, should use max_completion_tokens
            messages=[
                {
                    "role": "user",
                    "content": "Hello, this should trigger a 400 error due to unsupported max_tokens parameter",
                }
            ],
        )
        # If we reach here, the request unexpectedly succeeded
        logger.error(
            f"Expected 400 error but got successful response: {completion.choices[0].message.content}"
        )
        assert False, "Expected 400 error but request succeeded"
    except openai.BadRequestError as e:
        # This is what we expect - a 400 Bad Request error
        logger.info(f"Correctly received 400 Bad Request error: {e}")
        assert e.status_code == 400, f"Expected status code 400, got {e.status_code}"
        assert "max_tokens" in str(e), "Expected error message to mention max_tokens"
        logger.info("✓ 400 error handling for unsupported parameters working correctly")
    except Exception as e:
        # Any other exception is unexpected
        logger.error(
            f"Unexpected error type (should be BadRequestError): {type(e).__name__}: {e}"
        )
        assert False, f"Expected BadRequestError but got {type(e).__name__}: {e}"


def test_nonexistent_alias():
    """Test that using a non-existent alias falls back to treating it as a direct model name"""
    logger.info(
        "Testing non-existent alias 'nonexistent.alias' should be treated as direct model"
    )

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(
        api_key="test-key",
        base_url=f"{base_url}/v1",
    )

    try:
        completion = client.chat.completions.create(
            model="nonexistent.alias",  # This alias doesn't exist
            max_completion_tokens=50,
            messages=[
                {
                    "role": "user",
                    "content": "Hello, this should fail or use as direct model name",
                }
            ],
        )
        logger.info("Non-existent alias was handled gracefully")
        # If it succeeds, it means the alias was passed through as a direct model name
        logger.info(f"Response: {completion.choices[0].message.content}")
    except Exception as e:
        logger.info(f"Non-existent alias resulted in error (expected): {e}")
        # This is also acceptable behavior


# =============================================================================
# DIRECT MODEL TESTS (for comparison)
# =============================================================================


def test_direct_model_4o_mini_openai():
    """Test OpenAI client using direct model name '4o-mini'"""
    logger.info("Testing OpenAI client with direct model '4o-mini'")

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = openai.OpenAI(
        api_key="test-key",
        base_url=f"{base_url}/v1",
    )

    completion = client.chat.completions.create(
        model="gpt-4o-mini",  # Direct model name
        max_completion_tokens=50,
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from direct 4o-mini!",
            }
        ],
    )

    response_content = completion.choices[0].message.content
    logger.info(f"Response from direct 4o-mini: {response_content}")
    assert response_content == "Hello from direct 4o-mini!"


def test_direct_model_4o_mini_anthropic():
    """Test Anthropic client using direct model name '4o-mini'"""
    logger.info("Testing Anthropic client with direct model '4o-mini'")

    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")
    client = anthropic.Anthropic(api_key="test-key", base_url=base_url)

    message = client.messages.create(
        model="gpt-4o-mini",  # Direct model name
        max_tokens=50,
        messages=[
            {
                "role": "user",
                "content": "Hello, please respond with exactly: Hello from direct 4o-mini via Anthropic!",
            }
        ],
    )

    response_content = "".join(b.text for b in message.content if b.type == "text")
    logger.info(f"Response from direct 4o-mini via Anthropic: {response_content}")
    assert response_content == "Hello from direct 4o-mini via Anthropic!"


def test_anthropic_thinking_mode_streaming():
    # Anthropic base_url should be the root, not /v1/chat/completions
    base_url = LLM_GATEWAY_ENDPOINT.replace("/v1/chat/completions", "")

    client = anthropic.Anthropic(
        api_key=os.environ.get("ANTHROPIC_API_KEY", "test-key"),
        base_url=base_url,
    )

    thinking_block_started = False
    thinking_delta_seen = False
    text_delta_seen = False

    with client.messages.stream(
        model="claude-sonnet-4-20250514",
        max_tokens=2048,
        thinking={"type": "enabled", "budget_tokens": 1024},  # <- idiomatic
        messages=[{"role": "user", "content": "Explain briefly what 2+2 equals"}],
    ) as stream:
        for event in stream:
            # 1) detect when a thinking block starts
            if event.type == "content_block_start" and getattr(
                event, "content_block", None
            ):
                if getattr(event.content_block, "type", None) == "thinking":
                    thinking_block_started = True

            # 2) collect text vs thinking deltas
            if event.type == "content_block_delta" and getattr(event, "delta", None):
                if event.delta.type == "text_delta":
                    text_delta_seen = True
                elif event.delta.type == "thinking_delta":
                    # some SDKs expose .thinking, others .text for this delta; not needed here
                    thinking_delta_seen = True

        final = stream.get_final_message()

    # Basic integrity
    assert final is not None
    assert final.content and len(final.content) > 0

    # Normal text should have streamed
    assert text_delta_seen, "Expected normal text deltas in stream"

    # With thinking enabled, we expect a thinking block and at least one thinking delta
    assert thinking_block_started, "No thinking block started"
    assert thinking_delta_seen, "No thinking deltas observed"

    # Optional: double-check on the assembled message
    final_block_types = [blk.type for blk in final.content]
    assert "text" in final_block_types
    assert "thinking" in final_block_types
