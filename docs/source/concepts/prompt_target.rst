.. _prompt_target:

Prompt Target
==============

**Prompt Targets** are a core concept in Arch, empowering developers to clearly define how user prompts are interpreted, processed, and routed within their generative AI applications. Prompts can seamlessly be routed either to specialized AI agents capable of handling sophisticated, context-driven tasks or to targeted tools provided by your application, offering users a fast, precise, and personalized experience.

This section covers the essentials of prompt targets—what they are, how to configure them, their practical uses, and recommended best practices—to help you fully utilize this feature in your applications.

What Are Prompt Targets?
------------------------
Prompt targets are endpoints within Arch that handle specific types of user prompts. They act as the bridge between user inputs and your backend agents or tools (APIs), enabling Arch to route, process, and manage prompts efficiently. Defining prompt targets helps you decouple your application's core logic from processing and handling complexities, leading to clearer code organization, better scalability, and easier maintenance.


.. table::
    :width: 100%

    ====================    ============================================
    **Capability**          **Description**
    ====================    ============================================
    Intent Recognition      Identify the purpose of a user prompt.
    Parameter Extraction    Extract necessary data from the prompt.
    Invocation              Call relevant backend agents or tools (APIs).
    Response Handling       Process and return responses to the user.
    ====================    ============================================

Key Features
~~~~~~~~~~~~

Below are the key features of prompt targets that empower developers to build efficient, scalable, and personalized GenAI solutions:

- **Design Scenarios**: Define prompt targets to effectively handle specific agentic scenarios.
- **Input Management**: Specify required and optional parameters for each target.
- **Tools Integration**: Seamlessly connect prompts to backend APIs or functions.
- **Error Handling**: Direct errors to designated handlers for streamlined troubleshooting.
- **Metadata Enrichment**: Attach additional context to prompts for enhanced processing.

Configuring Prompt Targets
--------------------------
Configuring prompt targets involves defining them in Arch's configuration file. Each Prompt target specifies how a particular type of prompt should be handled, including the endpoint to invoke and any parameters required.

Basic Configuration
~~~~~~~~~~~~~~~~~~~

A prompt target configuration includes the following elements:

.. vale Vale.Spelling = NO

- ``name``: A unique identifier for the prompt target.
- ``description``: A brief explanation of what the prompt target does.
- ``endpoint``: Required if you want to call a tool or specific API. ``name`` and ``path`` ``http_method`` are the three attributes of the endpoint.
- ``parameters`` (Optional): A list of parameters to extract from the prompt.

.. _defining_prompt_target_parameters:

Defining Parameters
~~~~~~~~~~~~~~~~~~~
Parameters are the pieces of information that Arch needs to extract from the user's prompt to perform the desired action.
Each parameter can be marked as required or optional. Here is a full list of parameter attributes that Arch can support:

.. table::
    :width: 100%

    ========================  ============================================================================
    **Attribute**             **Description**
    ========================  ============================================================================
    ``name (req.)``           Specifies name of the parameter.
    ``description (req.)``    Provides a human-readable explanation of the parameter's purpose.
    ``type (req.)``           Specifies the data type. Supported types include: **int**, **str**, **float**, **bool**, **list**, **set**, **dict**, **tuple**
    ``in_path``               Indicates whether the parameter is part of the path in the endpoint url. Valid values: **true** or **false**
    ``default``               Specifies a default value for the parameter if not provided by the user.
    ``format``                Specifies a format for the parameter value. For example: `2019-12-31` for a date value.
    ``enum``                  Lists of allowable values for the parameter with data type matching the ``type`` attribute. **Usage Example**: ``enum: ["celsius`", "fahrenheit"]``
    ``items``                 Specifies the attribute of the elements when type equals **list**, **set**, **dict**, **tuple**. **Usage Example**: ``items: {"type": "str"}``
    ``required``              Indicates whether the parameter is mandatory or optional. Valid values: **true** or **false**
    ========================  ============================================================================

Example Configuration For Tools
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: yaml
    :caption: Tools and Function Calling Configuration Example

    prompt_targets:
      - name: get_weather
        description: Get the current weather for a location
        parameters:
          - name: location
            description: The city and state, e.g. San Francisco, New York
            type: str
            required: true
          - name: unit
            description: The unit of temperature
            type: str
            default: fahrenheit
            enum: [celsius, fahrenheit]
        endpoint:
          name: api_server
          path: /weather

Example Configuration For Agents
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: yaml
    :caption: Agent Orchestration Configuration Example

    overrides:
      use_agent_orchestrator: true

    prompt_targets:
      - name: sales_agent
        description: handles queries related to sales and purchases

      - name: issues_and_repairs
        description: handles issues, repairs, or refunds

      - name: escalate_to_human
        description: escalates to human agent

.. note::
    Today, you can use Arch to coordinate more specific agentic scenarios via tools and function calling, or use it for high-level agent routing and hand off scenarios. In the future, we plan to offer you the ability to combine these two approaches for more complex scenarios. Please see `github issues <https://github.com/katanemo/archgw/issues/442>`_ for more details.

Routing Logic
-------------
Prompt targets determine where and how user prompts are processed. Arch uses intelligent routing logic to ensure that prompts are directed to the appropriate targets based on their intent and context.

Default Targets
~~~~~~~~~~~~~~~
For general-purpose prompts that do not match any specific prompt target, Arch routes them to a designated default target. This is useful for handling open-ended queries like document summarization or information extraction.

Intent Matching
~~~~~~~~~~~~~~~
Arch analyzes the user's prompt to determine its intent and matches it with the most suitable prompt target based on the name and description defined in the configuration.

For example:

.. code-block:: bash

  Prompt: "Can you reboot the router?"
  Matching Target: reboot_device (based on description matching "reboot devices")


Summary
--------
Prompt targets are essential for defining how user prompts are handled within your generative AI applications using Arch.

By carefully configuring prompt targets, you can ensure that prompts are accurately routed, necessary parameters are extracted, and backend services are invoked seamlessly. This modular approach not only simplifies your application's architecture but also enhances scalability, maintainability, and overall user experience.
