.. _prompt_guard:

Guardrails
==========

**Guardrails** are Plano's way of applying safety and validation checks to prompts before they reach your application logic. They are typically implemented as
filters in a :ref:`Filter Chain <filter_chain>` attached to an agent, so every request passes through a consistent processing layer.


Why Guardrails
--------------
Guardrails are essential for maintaining control over AI-driven applications. They help enforce organizational policies, ensure compliance with regulations
(like GDPR or HIPAA), and protect users from harmful or inappropriate content. In applications where prompts generate responses or trigger actions, guardrails
minimize risks like malicious inputs, off-topic queries, or misaligned outputs—adding a consistent layer of input scrutiny that makes interactions safer,
more reliable, and easier to reason about.


.. vale Vale.Spelling = NO

- **Jailbreak Prevention**: Detect and filter inputs that attempt to change LLM behavior, expose system prompts, or bypass safety policies.
- **Domain and Topicality Enforcement**: Ensure that agents only respond to prompts within an approved domain (for example, finance-only or healthcare-only use cases) and reject unrelated queries.
- **Dynamic Error Handling**: Provide clear error messages when requests violate policy, helping users correct their inputs.

How Guardrails Work
-------------------

In Plano, guardrails are usually implemented as filters that run as HTTP services. Each filter receives the incoming prompt and related metadata, evaluates it
against policy, and either lets the request continue (HTTP 200) or terminates it early with an appropriate error code (typically HTTP 4xx for policy failures).

The example below shows a simple, plain-Python HTTP service that acts as a topicality guardrail: it rejects any prompt that is not related to the
"weather" domain.

.. code-block:: python
    :caption: Example topicality guard filter in plain Python (FastAPI)

    from fastapi import FastAPI, Request, HTTPException

    app = FastAPI()

    ALLOWED_KEYWORDS = {"weather", "forecast", "temperature", "rain", "snow", "humidity"}

    @app.post("/guardrails/topic")
    async def topic_guard(request: Request):
        body = await request.json()
        # Expecting an OpenAI-style request body with messages
        messages = body.get("messages", [])
        user_content = " ".join(
            m["content"] for m in messages if m.get("role") == "user"
        ).lower()

        if not any(keyword in user_content for keyword in ALLOWED_KEYWORDS):
            # Return 400 to indicate a policy failure (not a server error)
            raise HTTPException(
                status_code=400,
                detail={
                    "error": "off_topic",
                    "message": "This assistant only answers weather-related questions.",
                },
            )

        # If the prompt is on-topic, just pass the original body through
        return body


To wire this guardrail into Plano, you define a listener of ``type: agent`` and attach a filter chain with a single filter that points
to the Python service above.

.. code-block:: yaml
    :caption: Listener (type: agent) with a topicality guard filter

    filters:
      - id: topicality_guard
        url: http://topic-guard:8000/guardrails/topic

    listeners:
    - type: agent
        name: agent_listener
        port: 8001
        router: arch_agent_router
        agents:
        - id: rag_agent
            description: virtual assistant for retrieval augmented generation tasks
            filter_chain:
            - topicality_guard


When a request arrives at ``agent_listener``, Plano will first call the ``topicality_guard`` filter. If the filter returns **HTTP 200**,
the request continues on to the configured agent or prompt target. If the filter returns **HTTP 400**, Plano returns that error back to
the caller and does not forward the request further—enforcing your domain guardrail without changing any application code.
