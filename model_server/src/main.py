import json
import os
import time
import logging
import src.commons.utils as utils

from src.commons.globals import ARCH_ENDPOINT, handler_map
from src.core.function_calling import ArchFunctionHandler
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

logger.info(f"using archfc endpoint: {ARCH_ENDPOINT}")


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
    logger.info(f"[request body]: {json.dumps(req.model_dump(exclude_none=True))}")

    final_response: ChatCompletionResponse = None
    error_messages = None

    use_agent_orchestrator = req.metadata.get("use_agent_orchestrator", False)
    logger.info(f"Use agent orchestrator: {use_agent_orchestrator}")

    try:
        handler_name = "Arch-Agent" if use_agent_orchestrator else "Arch-Function"
        model_handler: ArchFunctionHandler = handler_map[handler_name]

        start_time = time.perf_counter()
        final_response = await model_handler.chat_completion(req)
        latency = time.perf_counter() - start_time

        if not final_response.metadata:
            final_response.metadata = {}

        # Parameter gathering for detected intents
        if final_response.choices[0].message.content:
            final_response.metadata["function_latency"] = str(round(latency * 1000, 3))
        # Function Calling
        elif final_response.choices[0].message.tool_calls:
            final_response.metadata["function_latency"] = str(round(latency * 1000, 3))

            if not use_agent_orchestrator:
                final_response.metadata["hallucination"] = str(
                    model_handler.hallucination_state.hallucination
                )
        # No intent detected
        else:
            final_response.metadata["intent_latency"] = str(round(latency * 1000, 3))

        if not use_agent_orchestrator:
            final_response.metadata["intent_latency"] = str(round(latency * 1000, 3))

            final_response.metadata["hallucination"] = str(
                model_handler.hallucination_state.hallucination
            )

    except ValueError as e:
        res.statuscode = 503
        error_messages = f"[{handler_name}] - Error in tool call extraction: {e}"
        raise
    except StopIteration as e:
        res.statuscode = 500
        error_messages = f"[{handler_name}] - Error in hallucination check: {e}"
        raise
    except Exception as e:
        res.status_code = 500
        error_messages = f"[{handler_name}] - Error in ChatCompletion: {e}"
        raise

    if error_messages is not None:
        logger.error(error_messages)
        final_response = ChatCompletionResponse(metadata={"error": error_messages})

    return final_response


@app.post("/guardrails")
async def guardrails(req: GuardRequest, res: Response, max_num_words=300):
    logger.info("[Endpoint: /guardrails] - Gateway")
    logger.info(f"[request body]: {json.dumps(req.model_dump(exclude_none=True))}")

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
