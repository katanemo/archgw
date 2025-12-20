.. _agent_routing:

Orchestration
==============

Building multi-agent systems allow you to route requests across multiple specialized agents, each designed to handle specific types of tasks.
Plano makes it easy to build and scale these systems by managing the orchestration layer—deciding which agent(s) should handle each request—while you focus on implementing individual agent logic.

This guide shows you how to configure and implement multi-agent orchestration in Plano.

How It Works
------------

Plano's orchestration layer analyzes incoming prompts and routes them to the most appropriate agent based on user intent and conversation context. The workflow is:

1. **User submits a prompt**: The request arrives at Plano's agent listener.
2. **Agent selection**: Plano analyzes the prompt to determine user intent and complexity, and routes the request to the most suitable agent configured in your system—such as a sales agent, technical support agent, or RAG agent.
3. **Agent handles request**: The selected agent processes the prompt using its specialized logic and tools.
4. **Seamless handoffs**: For multi-turn conversations, Plano repeats the intent analysis for each follow-up query, enabling smooth handoffs between agents as the conversation evolves.

Configuration
-------------

Configure your agents in the ``listeners`` section of your ``plano_config.yaml``:

.. code-block:: yaml
    :caption: Multi-Agent Configuration Example

    listeners:
      - type: agent
        name: agent_listener
        port: 8001
        agents:
          - id: sales_agent
            description: Handles sales inquiries, product recommendations, and pricing questions
            endpoint: http://sales-service:8000/agent

          - id: support_agent
            description: Handles technical issues, troubleshooting, and repairs
            endpoint: http://support-service:8000/agent

          - id: rag_agent
            description: Answers questions using retrieval augmented generation
            endpoint: http://rag-service:8000/agent
            filter_chain:
              - query_rewriter
              - context_builder

**Key Configuration Elements:**

* **agent listener**: A listener of ``type: agent`` tells Plano to perform intent analysis and routing for incoming requests.
* **agents list**: Define each agent with an ``id``, ``description`` (used for routing decisions), and ``endpoint`` (where Plano forwards requests).
* **filter_chain**: Optionally attach :ref:`filter chains <filter_chain>` to agents for guardrails, query rewriting, or context enrichment.

Implementation
--------------

Agents are HTTP services that receive routed requests from Plano. Here's how to implement a simple multi-agent system using Python and FastAPI:

.. code-block:: python
    :caption: Multi-Agent Implementation Example

    class Agent:
        def __init__(self, role: str, instructions: str):
            self.system_prompt = f"You are a {role}.\n{instructions}"

        def handle(self, req: ChatCompletionsRequest):
            messages = [{"role": "system", "content": self.get_system_prompt()}] + [
                message.model_dump() for message in req.messages
            ]
            return call_openai(messages, req.stream) #call_openai is a placeholder for the actual API call

        def get_system_prompt(self) -> str:
            return self.system_prompt

    # Define your agents
    AGENTS = {
        "sales_agent": Agent(
            role="sales agent",
            instructions=(
                "Always answer in a sentence or less.\n"
                "Follow the following routine with the user:\n"
                "1. Engage\n"
                "2. Quote ridiculous price\n"
                "3. Reveal caveat if user agrees."
            ),
        ),
        "issues_and_repairs": Agent(
            role="issues and repairs agent",
            instructions="Propose a solution, offer refund if necessary.",
        ),
        "escalate_to_human": Agent(
            role="human escalation agent", instructions="Escalate issues to a human."
        ),
        "unknown_agent": Agent(
            role="general assistant", instructions="Assist the user in general queries."
        ),
    }

    #handle the request from arch gateway
    @app.post("/v1/chat/completions")
    def completion_api(req: ChatCompletionsRequest, request: Request):

        agent_name = req.metadata.get("agent-name", "unknown_agent")
        agent = AGENTS.get(agent_name)
        logger.info(f"Routing to agent: {agent_name}")

        return agent.handle(req)

**How Requests Flow:**

1. User sends a prompt to Plano's agent listener (e.g., "I need help with a billing issue").
2. Plano-Orchestrator analyzes the intent and routes to the ``support_agent``.
3. Plano forwards the request to ``http://support-service:8000/agent`` with metadata indicating which agent was selected.
4. Your agent service receives the request, processes it using its specialized logic, and returns a response.
5. Plano forwards the agent's response back to the user.

.. note::
    For a complete working example with multiple agents, see our `multi-agent orchestration demo <https://github.com/katanemo/archgw/tree/main/demos/use_cases/orchestrating_agents>`_ on GitHub.

Common Use Cases
----------------

Multi-agent orchestration is particularly powerful for:

**Customer Support**

Route common queries to automated support agents while escalating complex or sensitive issues to human support staff.

.. code-block:: yaml

    agents:
      - id: tier1_support
        description: Handles common FAQs, password resets, and basic troubleshooting
      - id: tier2_support
        description: Handles complex technical issues requiring deep product knowledge
      - id: human_escalation
        description: Escalates sensitive issues or unresolved problems to human agents

**Sales and Marketing**

Direct potential leads and sales inquiries to specialized sales agents for timely, targeted follow-ups.

.. code-block:: yaml

    agents:
      - id: product_recommendation
        description: Recommends products based on user needs and preferences
      - id: pricing_agent
        description: Provides pricing information and quotes
      - id: sales_closer
        description: Handles final negotiations and closes deals

**Technical Documentation and Support**

Combine RAG agents for documentation lookup with specialized troubleshooting agents.

.. code-block:: yaml

    agents:
      - id: docs_agent
        description: Retrieves relevant documentation and guides
        filter_chain:
          - query_rewriter
          - context_builder
      - id: troubleshoot_agent
        description: Diagnoses and resolves technical issues step by step

Best Practices
--------------

**Write Clear Agent Descriptions**

Agent descriptions are used by Plano-Orchestrator to make routing decisions. Be specific about what each agent handles:

.. code-block:: yaml

    # Good - specific and actionable
    - id: refund_agent
      description: Processes refund requests for orders within 30 days, validates return eligibility

    # Less ideal - too vague
    - id: refund_agent
      description: Handles refunds

**Use Filter Chains for Cross-Cutting Concerns**

Apply :ref:`filter chains <filter_chain>` to agents that need guardrails, context enrichment, or query rewriting:

.. code-block:: yaml

    agents:
      - id: rag_agent
        description: Answers questions using company knowledge base
        filter_chain:
          - compliance_check  # Ensure queries comply with policies
          - query_rewriter    # Optimize query for retrieval
          - context_builder   # Fetch relevant docs

**Monitor and Optimize Routing**

Regularly review which agents handle which requests to identify mis-routing patterns and adjust agent descriptions:

.. code-block:: yaml

    # Monitor routing decisions through Plano's tracing
    # Adjust descriptions if you see unexpected routing behavior

**Route LLM Calls Through Plano's Model Proxy**

When your agents need to call LLMs, route those calls through Plano's :ref:`Model Proxy <llm_providers>` for consistent responses, smart routing, and rich observability. See :ref:`Making LLM Calls from Agents <agents>` for details.

Next Steps
----------

* Learn more about :ref:`agents <agents>` and the inner vs. outer loop model
* Explore :ref:`filter chains <filter_chain>` for adding guardrails and context enrichment
* See :ref:`observability <observability>` for monitoring multi-agent workflows
* Review the :ref:`LLM Providers <llm_providers>` guide for model routing within agents

.. note::
    To observe traffic to and from agents, please read more about :ref:`observability <observability>` in Plano.

By carefully configuring and managing your Agent routing and hand off, you can significantly improve your application's responsiveness, performance, and overall user satisfaction.
