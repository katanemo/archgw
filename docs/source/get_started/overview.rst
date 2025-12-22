.. _overview:

Overview
========
`Plano <https://github.com/katanemo/plano>`_ is delivery infrastructure for agentic apps. A models-native proxy server and data plane designed to help you build agents faster, and deliver them reliably to production.

Plano pulls out the rote plumbing work (the “hidden AI middleware”) and decouples you from brittle, ever‑changing framework abstractions. It centralizes what shouldn’t be bespoke in every codebase like agent routing and orchestration, rich agentic signals and traces for continuous improvement, guardrail filters for safety and moderation, and smart LLM routing APIs for UX and DX agility. Use any language or AI framework, and ship agents to production faster with Plano.

Built by core contributors to the widely adopted Envoy Proxy <https://www.envoyproxy.io/>_, Plano gives you a production‑grade foundation for agentic applications. It helps **developers** stay focused on the core logic of their agents, helps **product teams** shorten feedback loops for learning, and helps **engineering teams**  standardize policy and safety across agents and LLMs. Plano is grounded in open protocols (de facto: OpenAI‑style v1/responses, de jure: MCP) and proven patterns like sidecar deployments, so it plugs in cleanly while remaining robust, scalable, and flexible.

In this documentation, you’ll learn how to set up Plano quickly, trigger API calls via prompts, apply guardrails without tight coupling with application code, simplify model and provider integration, and improve observability — so that you can focus on what matters most: the core product logic of your agents.

.. figure:: /_static/img/plano_network_diagram_high_level.png
   :width: 100%
   :align: center

   High-level network flow of where Plano sits in your agentic stack. Designed for both ingress and egress traffic.


Get Started
-----------

This section introduces you to Plano and helps you get set up quickly:

.. grid:: 3

    .. grid-item-card:: :octicon:`apps` Overview
        :link: overview.html

        Overview of Plano and Doc navigation

    .. grid-item-card:: :octicon:`book` Intro to Plano
        :link: intro_to_plano.html

        Explore Plano's features and developer workflow

    .. grid-item-card:: :octicon:`rocket` Quickstart
        :link: quickstart.html

        Learn how to quickly set up and integrate


Concepts
--------

Deep dive into essential ideas and mechanisms behind Plano:

.. grid:: 3

    .. grid-item-card:: :octicon:`package` Tech Overview
        :link: ../concepts/tech_overview/tech_overview.html

        Learn about the technology stack

    .. grid-item-card:: :octicon:`webhook` LLM Providers
        :link: ../concepts/llm_providers/llm_providers.html

        Explore Arch’s LLM integration options

    .. grid-item-card:: :octicon:`workflow` Prompt Target
        :link: ../concepts/prompt_target.html

        Understand how Plano handles prompts


Guides
------
Step-by-step tutorials for practical Plano use cases and scenarios:

.. grid:: 3

    .. grid-item-card:: :octicon:`shield-check` Prompt Guard
        :link: ../guides/prompt_guard.html

        Instructions on securing and validating prompts

    .. grid-item-card:: :octicon:`code-square` Function Calling
        :link: ../guides/function_calling.html

        A guide to effective function calling

    .. grid-item-card:: :octicon:`issue-opened` Observability
        :link: ../guides/observability/observability.html

        Learn to monitor and troubleshoot Plano


Build with Plano
----------------

For developers extending and customizing Plano for specialized needs:

.. grid:: 2

    .. grid-item-card:: :octicon:`dependabot` Agentic Workflow
        :link: ../build_with_plano/agent.html

        Discover how to create and manage custom agents within Plano

    .. grid-item-card:: :octicon:`stack` RAG Application
        :link: ../build_with_plano/rag.html

        Integrate RAG for knowledge-driven responses
