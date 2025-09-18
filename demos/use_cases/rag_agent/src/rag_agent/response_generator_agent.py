import json
from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
from openai import AsyncOpenAI
import os
import logging
import time
import uuid
import uvicorn
import asyncio

from .api import (
    ChatCompletionRequest,
    ChatCompletionResponse,
    ChatCompletionStreamResponse,
)

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Configuration for archgw LLM gateway
LLM_GATEWAY_ENDPOINT = os.getenv("LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1")
RESPONSE_MODEL = "gpt-4o"

# System prompt for response generation
SYSTEM_PROMPT = """You are a helpful assistant that generates coherent, contextual responses.

Given a conversation history, generate a helpful and relevant response based on all the context available in the messages.
Your response should:
1. Be contextually aware of the entire conversation
2. Address the user's needs appropriately
3. Be helpful and informative
4. Maintain a natural conversational tone

Generate a complete response to assist the user."""

# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",  # archgw doesn't require a real API key
)

# FastAPI app for REST server
app = FastAPI(title="RAG Agent Response Generator", version="1.0.0")


def prepare_response_messages(request_body: ChatCompletionRequest):
    """Prepare messages for response generation by adding system prompt."""
    response_messages = [{"role": "system", "content": SYSTEM_PROMPT}]

    # Add conversation history
    for msg in request_body.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    return response_messages


@app.post("/v1/chat/completions")
async def chat_completions(request_body: ChatCompletionRequest, request: Request):
    """Chat completions endpoint that generates a coherent response based on all context."""
    logger.info(
        f"Received chat completion request with {len(request_body.messages)} messages"
    )

    # Read traceparent header if present
    traceparent_header = request.headers.get("traceparent")
    if traceparent_header:
        logger.info(f"Received traceparent header: {traceparent_header}")
    else:
        logger.info("No traceparent header found")

    # Check if streaming is requested
    if request_body.stream:
        return StreamingResponse(
            stream_chat_completions(request_body, traceparent_header),
            media_type="text/plain",
        )
    else:
        return await non_streaming_chat_completions(request_body, traceparent_header)


async def stream_chat_completions(
    request_body: ChatCompletionRequest, traceparent_header: str = None
):
    """Generate streaming chat completions."""
    # Prepare messages for response generation
    response_messages = prepare_response_messages(request_body)

    try:
        # Call archgw using OpenAI client for streaming
        logger.info(
            f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate streaming response"
        )

        # Prepare extra headers if traceparent is provided
        extra_headers = {"x-envoy-max-retries": "3"}
        if traceparent_header:
            extra_headers["traceparent"] = traceparent_header

        response_stream = await archgw_client.chat.completions.create(
            model=RESPONSE_MODEL,
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

                # Create streaming response chunk
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

        # Send final chunk with complete response in expected format
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
        logger.error(f"Error generating streaming response: {e}")

        # Send error as streaming response
        error_chunk = ChatCompletionStreamResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "delta": {
                        "content": "I apologize, but I'm having trouble generating a response right now. Please try again."
                    },
                    "finish_reason": "stop",
                }
            ],
        )

        yield f"data: {error_chunk.model_dump_json()}\n\n"
        yield "data: [DONE]\n\n"


async def non_streaming_chat_completions(
    request_body: ChatCompletionRequest, traceparent_header: str = None
):
    """Generate non-streaming chat completions."""
    # Prepare messages for response generation
    response_messages = prepare_response_messages(request_body)

    try:
        # Call archgw using OpenAI client
        logger.info(f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate response")

        # Prepare extra headers if traceparent is provided
        extra_headers = {"x-envoy-max-retries": "3"}
        if traceparent_header:
            extra_headers["traceparent"] = traceparent_header

        response = await archgw_client.chat.completions.create(
            model=RESPONSE_MODEL,
            messages=response_messages,
            temperature=request_body.temperature or 0.7,
            max_tokens=request_body.max_tokens or 1000,
            extra_headers=extra_headers,
        )

        generated_response = response.choices[0].message.content.strip()
        logger.info(f"Response generated successfully")

        updated_history = [{"role": "assistant", "content": generated_response}]

        return ChatCompletionResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": json.dumps(updated_history),
                    },
                    "finish_reason": "stop",
                }
            ],
            usage={
                "prompt_tokens": sum(
                    len(msg.content.split()) for msg in request_body.messages
                ),
                "completion_tokens": len(generated_response.split()),
                "total_tokens": sum(
                    len(msg.content.split()) for msg in request_body.messages
                )
                + len(generated_response.split()),
            },
        )

    except Exception as e:
        logger.error(f"Error generating response: {e}")

        # Fallback response
        fallback_message = "I apologize, but I'm having trouble generating a response right now. Please try again."
        return ChatCompletionResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request_body.model,
            choices=[
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": fallback_message},
                    "finish_reason": "stop",
                }
            ],
            usage={
                "prompt_tokens": sum(
                    len(msg.content.split()) for msg in request_body.messages
                ),
                "completion_tokens": len(fallback_message.split()),
                "total_tokens": sum(
                    len(msg.content.split()) for msg in request_body.messages
                )
                + len(fallback_message.split()),
            },
        )


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy"}


def start_server(host: str = "localhost", port: int = 8000):
    """Start the REST server."""
    uvicorn.run(app, host=host, port=port)
