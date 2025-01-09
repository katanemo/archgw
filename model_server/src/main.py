import json
import os
import time
import logging
import src.commons.utils as utils

from src.commons.globals import handler_map
from src.core.utils.model_utils import (
    ChatMessage,
    ChatCompletionResponse,
    GuardRequest,
    GuardResponse,
)

from fastapi import FastAPI, Response
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.sdk.resources import Resource
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter


resource = Resource.create(
    {
        "service.name": "model-server",
    }
)

# Initialize the tracer provider
trace.set_tracer_provider(TracerProvider(resource=resource))
tracer = trace.get_tracer(__name__)

# DEFAULT_OTLP_HOST = "http://localhost:4317"
DEFAULT_OTLP_HOST = "none"

# Configure the OTLP exporter (Jaeger, Zipkin, etc.)
otlp_exporter = OTLPSpanExporter(
    endpoint=os.getenv("OTLP_HOST", DEFAULT_OTLP_HOST)  # noqa: F821
)

trace.get_tracer_provider().add_span_processor(BatchSpanProcessor(otlp_exporter))


logger = utils.get_model_server_logger()
logging.getLogger("httpx").setLevel(logging.ERROR)
logging.getLogger("opentelemetry.exporter.otlp.proto.grpc.exporter").setLevel(
    logging.ERROR
)

app = FastAPI()
FastAPIInstrumentor().instrument_app(app)


@app.get("/healthz")
async def healthz():
    return {"status": "ok"}


@app.get("/models")
async def models():
    return {
        "object": "list",
        "data": [{"id": model_name, "object": "model"} for model_name in handler_map],
    }


@app.post("/function_calling")
async def function_calling(req: ChatMessage, res: Response):
    logger.info("[Endpoint: /function_calling]")
    logger.info(f"[request body]: {json.dumps(req.model_dump())}")

    final_response: ChatCompletionResponse = None
    error_messages = None

    try:
        intent_start_time = time.perf_counter()
        intent_response = await handler_map["Arch-Intent"].chat_completion(req)
        intent_latency = time.perf_counter() - intent_start_time

        if handler_map["Arch-Intent"].detect_intent(intent_response):
            # TODO: measure agreement between intent detection and function calling
            try:
                function_start_time = time.perf_counter()
                final_response = await handler_map["Arch-Function"].chat_completion(req)
                function_latency = time.perf_counter() - function_start_time

                final_response.metadata = {
                    "intent_latency": str(round(intent_latency * 1000, 3)),
                    "function_latency": str(round(function_latency * 1000, 3)),
                    "hallucination": str(
                        handler_map["Arch-Function"].hallucination_state.hallucination
                    ),
                }
            except ValueError as e:
                res.statuscode = 503
                error_messages = f"[Arch-Function] - Error in tool call extraction: {e}"
            except StopIteration as e:
                res.statuscode = 500
                error_messages = f"[Arch-Function] - Error in hallucination check: {e}"
            except Exception as e:
                res.status_code = 500
                error_messages = f"[Arch-Function] - Error in ChatCompletion: {e}"
        else:
            intent_response.metadata = {
                "intent_latency": str(round(intent_latency * 1000, 3)),
            }
            final_response = intent_response

    except Exception as e:
        res.status_code = 500
        error_messages = f"[Arch-Intent] - Error in ChatCompletion: {e}"

    if error_messages is not None:
        logger.error(error_messages)
        final_response = ChatCompletionResponse(metadata={"error": error_messages})

    return final_response


@app.post("/guardrails")
async def guardrails(req: GuardRequest, res: Response, max_num_words=300):
    logger.info("[Endpoint: /guardrails] - Gateway")
    logger.info(f"[request body]: {json.dumps(req.model_dump())}")

    final_response: GuardResponse = None
    error_messages = None

    try:
        guard_start_time = time.perf_counter()
        final_response = handler_map["Arch-Guard"].predict(req)
        guard_latency = time.perf_counter() - guard_start_time
        final_response.metadata = {
            "guard_latency": round(guard_latency * 1000, 3),
        }
    except Exception as e:
        res.status_code = 500
        error_messages = f"[Arch-Guard]: {e}"

    if error_messages is not None:
        logger.error(error_messages)
        final_response = GuardResponse(metadata={"error": error_messages})

    return final_response
