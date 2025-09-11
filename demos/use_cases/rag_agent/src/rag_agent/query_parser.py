from pydantic import BaseModel
from typing import List, Optional, Dict, Any
from fastapi import FastAPI, HTTPException
import uvicorn


# OpenAI Chat Completions API models
class ChatMessage(BaseModel):
    role: str
    content: str


class ChatCompletionRequest(BaseModel):
    model: str
    messages: List[ChatMessage]
    temperature: Optional[float] = 1.0
    max_tokens: Optional[int] = None
    top_p: Optional[float] = 1.0
    frequency_penalty: Optional[float] = 0.0
    presence_penalty: Optional[float] = 0.0
    stream: Optional[bool] = False
    stop: Optional[List[str]] = None


class ChatCompletionResponse(BaseModel):
    id: str
    object: str = "chat.completion"
    created: int
    model: str
    choices: List[Dict[str, Any]]
    usage: Dict[str, int]


class Response(BaseModel):
    query: str
    metadata: dict


# FastAPI app for REST server
app = FastAPI(title="RAG Agent Query Parser", version="1.0.0")


@app.post("/v1/chat/completions")
async def chat_completions(request: ChatCompletionRequest):
    """Chat completions endpoint that passes through the request as-is."""
    import time
    import uuid

    # Pass-through: return the last user message as the assistant response
    last_user_message = ""
    for message in reversed(request.messages):
        if message.role == "user":
            last_user_message = message.content
            break

    response = ChatCompletionResponse(
        id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
        created=int(time.time()),
        model=request.model,
        choices=[
            {
                "index": 0,
                "message": {"role": "assistant", "content": last_user_message},
                "finish_reason": "stop",
            }
        ],
        usage={
            "prompt_tokens": sum(len(msg.content.split()) for msg in request.messages),
            "completion_tokens": len(last_user_message.split()),
            "total_tokens": sum(len(msg.content.split()) for msg in request.messages)
            + len(last_user_message.split()),
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


# Register MCP tool only if mcp is available
try:
    from . import mcp

    if mcp is not None:
        mcp.tool()(parse_query)
except (ImportError, AttributeError):
    pass


def start_server(host: str = "localhost", port: int = 8000):
    """Start the REST server."""
    uvicorn.run(app, host=host, port=port)
