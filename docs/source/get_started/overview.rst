.. _overview:


Overview
============
`Arch <https://github.com/katanemo/arch>`_ is a modular edge and AI gateway for AI-native apps - one that is natively designed to handle and process prompts, not just network traffic.

Built by contributors to the widely adopted `Envoy Proxy <https://www.envoyproxy.io/>`_, Arch helps you move faster by handling the pesky *low-level* work in AI agent development—fast input clarification, intelligent agent routing, seamless prompt-to-tool integration, and unified LLM access and observability—all without locking you into a framework.


In this documentation, you will learn how to quickly set up Arch to trigger API calls via prompts, apply prompt guardrails without writing any application-level logic,
simplify the interaction with upstream LLMs, and improve observability all while simplifying your application development process.

.. figure:: /_static/img/arch_network_diagram_high_level.png
   :width: 100%
   :align: center

   High-level network flow of where Arch Gateway sits in your agentic stack. Designed for both ingress and egress prompt traffic.


Get Started
-----------

This section introduces you to Arch and helps you get set up quickly:

.. grid:: 3

    .. grid-item-card:: :octicon:`apps` Overview
        :link: overview.html

        Overview of Arch and Doc navigation

    .. grid-item-card:: :octicon:`book` Intro to Arch
        :link: intro_to_arch.html

        Explore Arch's features and developer workflow

    .. grid-item-card:: :octicon:`rocket` Quickstart
        :link: quickstart.html

        Learn how to quickly set up and integrate


Concepts
--------

Deep dive into essential ideas and mechanisms behind Arch:

.. grid:: 3

    .. grid-item-card:: :octicon:`package` Tech Overview
        :link: ../concepts/tech_overview/tech_overview.html

        Learn about the technology stack

    .. grid-item-card:: :octicon:`webhook` LLM Provider
        :link: ../concepts/llm_provider.html

        Explore Arch’s LLM integration options

    .. grid-item-card:: :octicon:`workflow` Prompt Target
        :link: ../concepts/prompt_target.html

        Understand how Arch handles prompts


Guides
------
Step-by-step tutorials for practical Arch use cases and scenarios:

.. grid:: 3

    .. grid-item-card:: :octicon:`shield-check` Prompt Guard
        :link: ../guides/prompt_guard.html

        Instructions on securing and validating prompts

    .. grid-item-card:: :octicon:`code-square` Function Calling
        :link: ../guides/function_calling.html

        A guide to effective function calling

    .. grid-item-card:: :octicon:`issue-opened` Observability
        :link: ../guides/observability/observability.html

        Learn to monitor and troubleshoot Arch


Build with Arch
---------------

For developers extending and customizing Arch for specialized needs:

.. grid:: 2

    .. grid-item-card:: :octicon:`dependabot` Agentic Workflow
        :link: ../build_with_arch/agent.html

        Discover how to create and manage custom agents within Arch

    .. grid-item-card:: :octicon:`stack` RAG Application
        :link: ../build_with_arch/rag.html

        Integrate RAG for knowledge-driven responses
