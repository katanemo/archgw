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
        model="arch.summarize.v1",  # This should resolve to 4o-mini
        max_tokens=50,
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
        max_tokens=50,
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
        model="arch.summarize.v1",  # This should resolve to 4o-mini
        max_tokens=50,
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
        max_tokens=50,
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
        model="arch.summarize.v1",  # This should resolve to 4o-mini
        max_tokens=50,
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
        model="arch.summarize.v1",  # This should resolve to 4o-mini
        max_tokens=50,
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
            max_tokens=50,
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
        model="4o-mini",  # Direct model name
        max_tokens=50,
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
        model="4o-mini",  # Direct model name
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
