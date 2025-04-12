.. _agent_routing:

Agent Routing and Hand Off
===========================

Agent Routing and Hand Off is a key feature in Arch that enables intelligent routing of user prompts to specialized AI agents or human agents based on the nature and complexity of the user's request.

This capability significantly enhances the efficiency and personalization of interactions, ensuring each prompt receives the most appropriate and effective handling. The following section describes
the workflow, configuration, and implementation of Agent routing and hand off in Arch.

#. **Agent Selection**
   When a user submits a prompt, Arch analyzes the input to determine the intent and complexity. Based on the analysis, Arch selects the most suitable agent configured within your application to handle the specific category of the user's request—such as sales inquiries, technical issues, or complex scenarios requiring human attention.

#. **Prompt Routing**
   After selecting the appropriate agent, Arch routes the user's prompt to the designated agent's endpoint and waits for the agent to respond back with the processed output or further instructions.

#. **Hand Off**
   Based on follow-up queries from the user, Arch repeats the process of analysis, agent selection, and routing to ensure a seamless hand off between AI agents as needed.

.. code-block:: yaml
    :caption: Agent Routing and Hand Off Configuration Example

    prompt_targets:
      - name: sales_agent
        description: Handles queries related to sales and purchases

      - name: issues_and_repairs
        description: handles issues, repairs, or refunds

      - name: escalate_to_human
        description: escalates to human agent

.. code-block:: python
    :caption: Agent Routing and Hand Off Implementation Example via FastAPI

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

.. note::
    The above example demonstrates a simple implementation of Agent Routing and Hand Off using FastAPI. For the full implementation of this example
    please see our `GitHub demo <https://github.com/katanemo/archgw/tree/main/demos/use_cases/orchestrating_agents>`_.

Example Use Cases
-----------------
Agent Routing and Hand Off is particularly beneficial in scenarios such as:

- **Customer Support**: Routing common customer queries to automated support agents, while escalating complex or sensitive issues to human support staff.
- **Sales and Marketing**: Automatically directing potential leads and sales inquiries to specialized sales agents for timely and targeted follow-ups.
- **Technical Assistance**: Managing user-reported issues, repairs, or refunds by assigning them to the correct technical or support agent efficiently.

Best Practices and Tips
------------------------
When implementing Agent Routing and Hand Off in your applications, consider these best practices:

- Clearly define agent responsibilities: Ensure each agent or human endpoint has a clear, specific description of the prompts they handle, reducing mis-routing.
- Monitor and optimize routes: Regularly review how prompts are routed to adjust and optimize agent definitions and configurations.

.. note::
    To observe traffic to and from agents, please read more about :ref:`observability <observability>` in Arch.

By carefully configuring and managing your Agent routing and hand off, you can significantly improve your application's responsiveness, performance, and overall user satisfaction.
