from fastapi import FastAPI
from openai import AsyncOpenAI
import os
import logging
import time
import uuid

from .api import ChatCompletionRequest, ChatCompletionResponse

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Configuration for archgw LLM gateway
LLM_GATEWAY_ENDPOINT = os.getenv("LLM_GATEWAY_ENDPOINT", "http://localhost:12000/v1")
RESPONSE_MODEL = "gpt-4o"

# Initialize OpenAI client for archgw
archgw_client = AsyncOpenAI(
    base_url=LLM_GATEWAY_ENDPOINT,
    api_key="EMPTY",  # archgw doesn't require a real API key
)

# FastAPI app for REST server
app = FastAPI(title="RAG Agent Response Generator", version="1.0.0")


@app.post("/v1/chat/completions")
async def chat_completions(request: ChatCompletionRequest):
    """Chat completions endpoint that generates a coherent response based on all context."""
    logger.info(
        f"Received chat completion request with {len(request.messages)} messages"
    )

    # Prepare the system prompt for response generation
    system_prompt = """You are a helpful assistant that generates coherent, contextual responses.

    Given a conversation history, generate a helpful and relevant response based on all the context available in the messages.
    Your response should:
    1. Be contextually aware of the entire conversation
    2. Address the user's needs appropriately
    3. Be helpful and informative
    4. Maintain a natural conversational tone

    Generate a complete response to assist the user."""

    # Prepare messages for response generation
    response_messages = [{"role": "system", "content": system_prompt}]

    # Add conversation history
    for msg in request.messages:
        response_messages.append({"role": msg.role, "content": msg.content})

    try:
        # Call archgw using OpenAI client
        logger.info(f"Calling archgw at {LLM_GATEWAY_ENDPOINT} to generate response")
        response = await archgw_client.chat.completions.create(
            model=RESPONSE_MODEL,
            messages=response_messages,
            temperature=request.temperature or 0.7,
            max_tokens=request.max_tokens or 1000,
        )

        generated_response = response.choices[0].message.content.strip()
        logger.info(f"Response generated successfully")

        return ChatCompletionResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            created=int(time.time()),
            model=request.model,
            choices=[
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": generated_response},
                    "finish_reason": "stop",
                }
            ],
            usage={
                "prompt_tokens": sum(
                    len(msg.content.split()) for msg in request.messages
                ),
                "completion_tokens": len(generated_response.split()),
                "total_tokens": sum(
                    len(msg.content.split()) for msg in request.messages
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
            model=request.model,
            choices=[
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": fallback_message},
                    "finish_reason": "stop",
                }
            ],
            usage={
                "prompt_tokens": sum(
                    len(msg.content.split()) for msg in request.messages
                ),
                "completion_tokens": len(fallback_message.split()),
                "total_tokens": sum(
                    len(msg.content.split()) for msg in request.messages
                )
                + len(fallback_message.split()),
            },
        )


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy"}
