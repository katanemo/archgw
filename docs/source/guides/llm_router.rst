.. _llm_router:

LLM Routing
==============================================================

With the rapid proliferation of large language models (LLM) — each optimized for different strengths, style, or latency/cost profile — routing has become an essential technique to operationalize the use of different models.

Arch provides two distinct routing approaches to meet different use cases:

1. **Static Model Selection**: Direct routing to specific models based on provider configuration and model aliases
2. **Preference-Aligned Dynamic Routing**: Intelligent routing using the Arch-Router model based on context and user-defined preferences

This enables optimal performance, cost efficiency, and response quality by matching requests with the most suitable model from your available LLM fleet.


Routing Methods
---------------

**Static Model Selection**

Static routing allows you to directly specify which model to use, either through:

- **Direct Model Names**: Use provider-specific names like ``openai/gpt-4o-mini``
- **Model Aliases**: Use semantic names like ``fast-model`` or ``arch.summarize.v1`` (see :ref:`model_aliases`)

This approach is ideal when you know exactly which model you want to use for specific tasks or when implementing your own routing logic at the application level.

**Preference-Aligned Dynamic Routing (Arch-Router)**

Dynamic routing uses the Arch-Router model to automatically select the most appropriate LLM for each request based on:

- **Domain Analysis**: Identifies the subject matter (e.g., legal, healthcare, programming)
- **Action Classification**: Determines the type of operation (e.g., summarization, code generation, translation)
- **User-Defined Preferences**: Maps domains and actions to preferred models

This approach is ideal when you want intelligent, context-aware routing that adapts to the content and intent of each request.


Static Model Selection Workflow
--------------------------------

For static routing, the process is straightforward:

#. **Client Request**

    The client specifies the exact model to use, either by provider name (``openai/gpt-4o``) or alias (``fast-model``).

#. **Model Resolution**

    If using an alias, Arch resolves it to the actual provider model name.

#. **Direct Routing**

    The request is sent directly to the specified model without analysis or decision-making.

#. **Response Handling**

    The response is returned to the client with optional metadata about the routing decision.


Preference-Aligned Dynamic Routing Workflow (Arch-Router)
---------------------------------------

For preference-aligned dynamic routing, the process involves intelligent analysis:

#. **Prompt Analysis**

    When a user submits a prompt without specifying a model, the Arch-Router analyzes it to determine the domain (subject matter) and action (type of operation requested).

#. **Model Selection**

    Based on the analyzed intent and your configured routing preferences, the Router selects the most appropriate model from your available LLM fleet.

#. **Request Forwarding**

    Once the optimal model is identified, our gateway forwards the original prompt to the selected LLM endpoint. The routing decision is transparent and can be logged for monitoring and optimization purposes.

#. **Response Handling**

    After the selected model processes the request, the response is returned through the gateway. The gateway can optionally add routing metadata or performance metrics to help you understand and optimize your routing decisions.

Arch-Router
-------------------------
The `Arch-Router <https://huggingface.co/katanemo/Arch-Router-1.5B>`_ is a state-of-the-art **preference-based routing model** specifically designed for intelligent LLM selection. This model delivers production-ready performance with low latency and high accuracy.

To support effective routing, Arch-Router introduces two key concepts:

- **Domain** – the high-level thematic category or subject matter of a request (e.g., legal, healthcare, programming).

- **Action** – the specific type of operation the user wants performed (e.g., summarization, code generation, booking appointment, translation).

Both domain and action configs are associated with preferred models or model variants. At inference time, Arch-Router analyzes the incoming prompt to infer its domain and action using semantic similarity, task indicators, and contextual cues. It then applies the user-defined routing preferences to select the model best suited to handle the request.

In summary, Arch-Router demonstrates:

- **Structured Preference Routing**: Aligns prompt request with model strengths using explicit domain–action mappings.

- **Transparent and Controllable**: Makes routing decisions transparent and configurable, empowering users to customize system behavior.

- **Flexible and Adaptive**: Supports evolving user needs, model updates, and new domains/actions without retraining the router.

- **Production-Ready Performance**: Optimized for low-latency, high-throughput applications in multi-model environments.


Implementing Routing
--------------------

**Static Model Selection**

For static routing, simply configure your LLM providers and optionally define model aliases:

.. code-block:: yaml
    :caption: Static Routing Configuration

    listeners:
      egress_traffic:
        address: 0.0.0.0
        port: 12000
        message_format: openai
        timeout: 30s

    llm_providers:
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_API_KEY
        default: true

      - model: openai/gpt-4o
        access_key: $OPENAI_API_KEY

      - model: anthropic/claude-3-5-sonnet-20241022
        access_key: $ANTHROPIC_API_KEY

    # Optional: Define aliases for easier client usage
    model_aliases:
      fast-model:
        target: gpt-4o-mini
      smart-model:
        target: gpt-4o
      creative-model:
        target: claude-3-5-sonnet-20241022

Clients can then specify models directly:

.. code-block:: python

    # Using provider model names
    response = client.chat.completions.create(
        model="openai/gpt-4o-mini",
        messages=[{"role": "user", "content": "Hello!"}]
    )

    # Using aliases
    response = client.chat.completions.create(
        model="fast-model",
        messages=[{"role": "user", "content": "Hello!"}]
    )

**Preference-Aligned Dynamic Routing (Arch-Router)**

To configure preference-aligned dynamic routing, you need to define routing preferences that map domains and actions to specific models:

.. code-block:: yaml
    :caption: Preference-Aligned Dynamic Routing Configuration

    listeners:
      egress_traffic:
        address: 0.0.0.0
        port: 12000
        message_format: openai
        timeout: 30s

    llm_providers:
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_API_KEY
        default: true

      - model: openai/gpt-4o
        access_key: $OPENAI_API_KEY
        routing_preferences:
          - name: code understanding
            description: understand and explain existing code snippets, functions, or libraries
          - name: complex reasoning
            description: deep analysis, mathematical problem solving, and logical reasoning

      - model: anthropic/claude-3-5-sonnet-20241022
        access_key: $ANTHROPIC_API_KEY
        routing_preferences:
          - name: creative writing
            description: creative content generation, storytelling, and writing assistance
          - name: code generation
            description: generating new code snippets, functions, or boilerplate based on user prompts

Clients can let the router decide or use aliases:

.. code-block:: python

    # Let Arch-Router choose based on content
    response = client.chat.completions.create(
        messages=[{"role": "user", "content": "Write a creative story about space exploration"}]
        # No model specified - router will analyze and choose claude-3-5-sonnet-20241022
    )


Combining Routing Methods
-------------------------

You can combine static model selection with dynamic routing preferences for maximum flexibility:

.. code-block:: yaml
    :caption: Hybrid Routing Configuration

    llm_providers:
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_API_KEY
        default: true

      - model: openai/gpt-4o
        access_key: $OPENAI_API_KEY
        routing_preferences:
          - name: complex_reasoning
            description: deep analysis and complex problem solving

      - model: anthropic/claude-3-5-sonnet-20241022
        access_key: $ANTHROPIC_API_KEY
        routing_preferences:
          - name: creative_tasks
            description: creative writing and content generation

    model_aliases:
      # Static aliases for direct routing
      fast-model:
        target: gpt-4o-mini

      reasoning-model:
        target: gpt-4o

      # Aliases that can also participate in dynamic routing
      creative-model:
        target: claude-3-5-sonnet-20241022

This configuration allows clients to:

1. **Use direct model selection**: ``model="fast-model"``
2. **Let the router decide**: No model specified, router analyzes content

Example Use Cases
-------------------------
Here are common scenarios where Arch-Router excels:

- **Coding Tasks**: Distinguish between code generation requests ("write a Python function"), debugging needs ("fix this error"), and code optimization ("make this faster"), routing each to appropriately specialized models.

- **Content Processing Workflows**: Classify requests as summarization ("summarize this document"), translation ("translate to Spanish"), or analysis ("what are the key themes"), enabling targeted model selection.

- **Multi-Domain Applications**: Accurately identify whether requests fall into legal, healthcare, technical, or general domains, even when the subject matter isn't explicitly stated in the prompt.

- **Conversational Routing**: Track conversation context to identify when topics shift between domains or when the type of assistance needed changes mid-conversation.


Best practicesm
-------------------------
- **💡Consistent Naming:**  Route names should align with their descriptions.

  - ❌ Bad:
    ```
    {"name": "math", "description": "handle solving quadratic equations"}
    ```
  - ✅ Good:
    ```
    {"name": "quadratic_equation", "description": "solving quadratic equations"}
    ```

- **💡 Clear Usage Description:**  Make your route names and descriptions specific, unambiguous, and minimizing overlap between routes. The Router performs better when it can clearly distinguish between different types of requests.

  - ❌ Bad:
    ```
    {"name": "math", "description": "anything closely related to mathematics"}
    ```
  - ✅ Good:
    ```
    {"name": "math", "description": "solving, explaining math problems, concepts"}
    ```

- **💡Nouns Descriptor:** Preference-based routers perform better with noun-centric descriptors, as they offer more stable and semantically rich signals for matching.

- **💡Domain Inclusion:** for best user experience, you should always include domain route. This help the router fall back to domain when action is not

.. Unsupported Features
.. -------------------------

.. The following features are **not supported** by the Arch-Router model:

.. - **❌ Multi-Modality:**
..   The model is not trained to process raw image or audio inputs. While it can handle textual queries *about* these modalities (e.g., "generate an image of a cat"), it cannot interpret encoded multimedia data directly.

.. - **❌ Function Calling:**
..   This model is designed for **semantic preference matching**, not exact intent classification or tool execution. For structured function invocation, use models in the **Arch-Function-Calling** collection.

.. - **❌ System Prompt Dependency:**
..   Arch-Router routes based solely on the user’s conversation history. It does not use or rely on system prompts for routing decisions.
