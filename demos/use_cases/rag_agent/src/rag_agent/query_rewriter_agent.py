import json
from pydantic import BaseModel
from typing import List, Optional, Dict, Any
from fastapi import FastAPI, HTTPException, Request
from openai import AsyncOpenAI
import os
import logging
import uvicorn

from .api import ChatMessage, ChatCompletionRequest, ChatCompletionResponse


# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - [QUERY_REWRITER] - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)


# Configuration for archgw LLM gateway
LLM_GATEWAY_ENDPOINT = os.getenv("LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1")
QUERY_REWRITE_MODEL = "gpt-4o-mini"

# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",  # archgw doesn't require a real API key
)


async def rewrite_query_with_archgw(
    messages: List[ChatMessage], traceparent_header: str
) -> str:
    # Prepare the system prompt for query rewriting
    system_prompt = """You are a query rewriter that improves user queries for better retrieval.

    Given a conversation history, rewrite the last user message to be more specific and context-aware.
    The rewritten query should:
    1. Include relevant context from previous messages
    2. Be clear and specific for information retrieval
    3. Maintain the user's intent
    4. Be concise but comprehensive

    Return only the rewritten query, nothing else."""

    # Prepare messages for the query rewriter - just add system prompt to existing messages
    rewrite_messages = [{"role": "system", "content": system_prompt}]

    # Add conversation history
    for msg in messages:
        rewrite_messages.append({"role": msg.role, "content": msg.content})

    try:
        # Call archgw using OpenAI client
        extra_headers = {"x-envoy-max-retries": "3"}
        if traceparent_header:
            extra_headers["traceparent"] = traceparent_header
        logger.info(f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to rewrite query")
        response = await archgw_client.chat.completions.create(
            model=QUERY_REWRITE_MODEL,
            messages=rewrite_messages,
            temperature=0.3,
            max_tokens=200,
            extra_headers=extra_headers,
        )

        rewritten_query = response.choices[0].message.content.strip()
        logger.info(f"Query rewritten successfully: '{rewritten_query}'")
        return rewritten_query

    except Exception as e:
        logger.error(f"Error rewriting query: {e}")

    # If rewriting fails, return the original last user message
    logger.info("Falling back to original user message")
    for message in reversed(messages):
        if message.role == "user":
            return message.content
    return ""


class Response(BaseModel):
    query: str
    metadata: dict


# FastAPI app for REST server
app = FastAPI(title="RAG Agent Query Parser", version="1.0.0")


@app.post("/v1/chat/completions")
async def chat_completions(request_body: ChatCompletionRequest, request: Request):
    """Chat completions endpoint that rewrites the last user query using archgw."""
    import time
    import uuid

    logger.info(
        f"Received chat completion request with {len(request_body.messages)} messages"
    )

    # Read traceparent header if present
    traceparent_header = request.headers.get("traceparent")
    if traceparent_header:
        logger.info(f"Received traceparent header: {traceparent_header}")
    else:
        logger.info("No traceparent header found")

    # Call archgw to rewrite the last user query
    rewritten_query = await rewrite_query_with_archgw(
        request_body.messages, traceparent_header
    )

    # Create updated messages with the rewritten query
    updated_messages = request_body.messages.copy()

    # Find and update the last user message with the rewritten query
    for i in range(len(updated_messages) - 1, -1, -1):
        if updated_messages[i].role == "user":
            original_query = updated_messages[i].content
            updated_messages[i] = ChatMessage(role="user", content=rewritten_query)
            logger.info(
                f"Updated user query from '{original_query}' to '{rewritten_query}'"
            )
            break

    messages_history_json = json.dumps([msg.dict() for msg in updated_messages])

    response = ChatCompletionResponse(
        id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
        created=int(time.time()),
        model=request_body.model,
        choices=[
            {
                "index": 0,
                "message": {"role": "user", "content": messages_history_json},
                "finish_reason": "stop",
            }
        ],
        usage={
            "prompt_tokens": sum(len(msg.content.split()) for msg in updated_messages),
            "completion_tokens": len("Updated query for better retrieval.".split()),
            "total_tokens": sum(len(msg.content.split()) for msg in updated_messages)
            + len("Updated query for better retrieval.".split()),
        },
    )

    return response


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy"}


def parse_query(query):
    """Parse the user query and returns metadata extracted from query."""
    return Response(query=query, metadata={"is_valid": True})


def start_server(host: str = "localhost", port: int = 8000):
    """Start the REST server."""
    uvicorn.run(app, host=host, port=port)
