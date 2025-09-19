.. _llm_router:

LLM Routing
==============================================================

With the rapid proliferation of large language models (LLM) — each optimized for different strengths, style, or latency/cost profile — routing has become an essential technique to operationalize the use of different models.

Arch provides three distinct routing approaches to meet different use cases:

1. **Model-based Routing**: Direct routing to specific models using provider/model names
2. **Alias-based Routing**: Semantic routing using custom aliases that map to underlying models
3. **Preference-aligned Routing**: Intelligent routing using the Arch-Router model based on context and user-defined preferences

This enables optimal performance, cost efficiency, and response quality by matching requests with the most suitable model from your available LLM fleet.


Routing Methods
---------------

Model-based Routing
~~~~~~~~~~~~~~~~~~~

Direct routing allows you to specify exact provider and model combinations using the format ``provider/model-name``:

- Use provider-specific names like ``openai/gpt-4o`` or ``anthropic/claude-3-5-sonnet-20241022``
- Provides full control and transparency over which model handles each request
- Ideal for production workloads where you want predictable routing behavior

Alias-based Routing
~~~~~~~~~~~~~~~~~~~

Alias-based routing lets you create semantic model names that decouple your application from specific providers:

- Use meaningful names like ``fast-model``, ``reasoning-model``, or ``arch.summarize.v1`` (see :ref:`model_aliases`)
- Maps semantic names to underlying provider models for easier experimentation and provider switching
- Ideal for applications that want abstraction from specific model names while maintaining control

.. _preference_aligned_routing:

Preference-aligned Routing (Arch-Router)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Traditional LLM routing approaches face significant limitations: they evaluate performance using benchmarks that often fail to capture human preferences, select from fixed model pools, and operate as "black boxes" without practical mechanisms for encoding user preferences.

Arch's preference-aligned routing addresses these challenges by applying a fundamental engineering principle: decoupling. The framework separates route selection (matching queries to human-readable policies) from model assignment (mapping policies to specific LLMs). This separation allows you to define routing policies using descriptive labels like ``Domain: 'finance', Action: 'analyze_earnings_report'`` rather than cryptic identifiers, while independently configuring which models handle each policy.

The `Arch-Router <https://huggingface.co/katanemo/Arch-Router-1.5B>`_ model automatically selects the most appropriate LLM based on:

- Domain Analysis: Identifies the subject matter (e.g., legal, healthcare, programming)
- Action Classification: Determines the type of operation (e.g., summarization, code generation, translation)
- User-Defined Preferences: Maps domains and actions to preferred models using transparent, configurable routing decisions
- Human Preference Alignment: Uses domain-action mappings that capture subjective evaluation criteria, ensuring routing aligns with real-world user needs rather than just benchmark scores

This approach supports seamlessly adding new models without retraining and is ideal for dynamic, context-aware routing that adapts to request content and intent.


Model-based Routing Workflow
----------------------------

For direct model routing, the process is straightforward:

#. **Client Request**

    The client specifies the exact model using provider/model format (``openai/gpt-4o``).

#. **Provider Validation**

    Arch validates that the specified provider and model are configured and available.

#. **Direct Routing**

    The request is sent directly to the specified model without analysis or decision-making.

#. **Response Handling**

    The response is returned to the client with optional metadata about the routing decision.


Alias-based Routing Workflow
-----------------------------

For alias-based routing, the process includes name resolution:

#. **Client Request**

    The client specifies a semantic alias name (``reasoning-model``).

#. **Alias Resolution**

    Arch resolves the alias to the actual provider/model name based on configuration.

#. **Model Selection**

    If the alias maps to multiple models, Arch selects one based on availability and load balancing.

#. **Request Forwarding**

    The request is forwarded to the resolved model.

#. **Response Handling**

    The response is returned with optional metadata about the alias resolution.


.. _preference_aligned_routing_workflow:

Preference-aligned Routing Workflow (Arch-Router)
-------------------------------------------------

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
The `Arch-Router <https://huggingface.co/katanemo/Arch-Router-1.5B>`_ is a state-of-the-art **preference-based routing model** specifically designed to address the limitations of traditional LLM routing. This compact 1.5B model delivers production-ready performance with low latency and high accuracy while solving key routing challenges.

**Addressing Traditional Routing Limitations:**

**Human Preference Alignment**
Unlike benchmark-driven approaches, Arch-Router learns to match queries with human preferences by using domain-action mappings that capture subjective evaluation criteria, ensuring routing decisions align with real-world user needs.

**Flexible Model Integration**
The system supports seamlessly adding new models for routing without requiring retraining or architectural modifications, enabling dynamic adaptation to evolving model landscapes.

**Preference-Encoded Routing**
Provides a practical mechanism to encode user preferences through domain-action mappings, offering transparent and controllable routing decisions that can be customized for specific use cases.

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

**Model-based Routing**

For direct model routing, configure your LLM providers with specific provider/model names:

.. code-block:: yaml
    :caption: Model-based Routing Configuration

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

Clients specify exact models:

.. code-block:: python

    # Direct provider/model specification
    response = client.chat.completions.create(
        model="openai/gpt-4o-mini",
        messages=[{"role": "user", "content": "Hello!"}]
    )

    response = client.chat.completions.create(
        model="anthropic/claude-3-5-sonnet-20241022",
        messages=[{"role": "user", "content": "Write a story"}]
    )

**Alias-based Routing**

Configure semantic aliases that map to underlying models:

.. code-block:: yaml
    :caption: Alias-based Routing Configuration

    listeners:
      egress_traffic:
        address: 0.0.0.0
        port: 12000
        message_format: openai
        timeout: 30s

    llm_providers:
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_API_KEY

      - model: openai/gpt-4o
        access_key: $OPENAI_API_KEY

      - model: anthropic/claude-3-5-sonnet-20241022
        access_key: $ANTHROPIC_API_KEY

    model_aliases:
      # Model aliases - friendly names that map to actual provider names
      fast-model:
        target: gpt-4o-mini

      reasoning-model:
        target: gpt-4o

      creative-model:
        target: claude-3-5-sonnet-20241022

Clients use semantic names:

.. code-block:: python

    # Using semantic aliases
    response = client.chat.completions.create(
        model="fast-model",  # Routes to best available fast model
        messages=[{"role": "user", "content": "Quick summary please"}]
    )

    response = client.chat.completions.create(
        model="reasoning-model",  # Routes to best reasoning model
        messages=[{"role": "user", "content": "Solve this complex problem"}]
    )

**Preference-aligned Routing (Arch-Router)**

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
      # Model aliases - friendly names that map to actual provider names
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
