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

Guardrails can be implemented as either in-process MCP filters or as HTTP-based filters. HTTP filters are external services that receive the request over HTTP, validate it, and return a response to allow or reject the request. This makes it easy to use filters written in any language or run them as independent services.

Each filter receives the chat messages, evaluates them against policy, and either lets the request continue or raises a ``ToolError`` (or returns an error response) to reject it with a helpful error message.

The example below shows an input guard for TechCorp's customer support system that validates queries are within the company's domain:

.. code-block:: python
    :caption: Example domain validation guard using FastMCP

    from typing import List
    from fastmcp.exceptions import ToolError
    from . import mcp

    @mcp.tool
    async def input_guards(messages: List[ChatMessage]) -> List[ChatMessage]:
        """Validates queries are within TechCorp's domain."""

        # Get the user's query
        user_query = next(
            (msg.content for msg in reversed(messages) if msg.role == "user"),
            ""
        )

        # Use an LLM to validate the query scope (simplified)
        is_valid = await validate_with_llm(user_query)

        if not is_valid:
            raise ToolError(
                "I can only assist with questions related to TechCorp and its services. "
                "Please ask about TechCorp's products, pricing, SLAs, or technical support."
            )

        return messages


To wire this guardrail into Plano, define the filter and add it to your agent's filter chain:

.. code-block:: yaml
    :caption: Plano configuration with input guard filter

    filters:
      - id: input_guards
        url: http://localhost:10500

    listeners:
      - type: agent
        name: agent_1
        port: 8001
        router: plano_orchestrator_v1
        agents:
          - id: rag_agent
            description: virtual assistant for retrieval augmented generation tasks
            filter_chain:
              - input_guards


When a request arrives at ``agent_1``, Plano invokes the ``input_guards`` filter first. If validation passes, the request continues to
the agent. If validation fails (``ToolError`` raised), Plano returns an error response to the caller.

Testing the Guardrail
---------------------

Here's an example of the guardrail in action, rejecting a query about Apple Corporation (outside TechCorp's domain):

.. code-block:: bash
    :caption: Request that violates the guardrail policy

    curl -X POST http://localhost:8001/v1/chat/completions \
      -H "Content-Type: application/json" \
      -d '{
        "model": "gpt-4",
        "messages": [
          {
            "role": "user",
            "content": "what is sla for apple corporation?"
          }
        ],
        "stream": false
      }'

.. code-block:: json
    :caption: Error response from the guardrail

    {
      "error": "ClientError",
      "agent": "input_guards",
      "status": 400,
      "agent_response": "I apologize, but I can only assist with questions related to TechCorp and its services. Your query appears to be outside this scope. The query is about SLA for Apple Corporation, which is unrelated to TechCorp.\n\nPlease ask me about TechCorp's products, services, pricing, SLAs, or technical support."
    }

This prevents out-of-scope queries from reaching your agent while providing clear feedback to users about why their request was rejected.
