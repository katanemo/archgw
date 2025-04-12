.. _intro_to_arch:

Intro to Arch
=============

Arch is an intelligent proxy server designed agentic applications. **Move faster** by letting Arch handle the **pesky heavy lifting** in building agents:
fast input clarification, agent routing, seamless integration of prompts with tools for common tasks, and unified access and observability of LLMs.

Past the thrill of an AI demo, have you found yourself hitting these walls? You know, the all too familiar ones:

- You break a prompt into specialized ones, but **get stuck writing routing** and handoff logic?
- You want use new LLMs, but **struggle to quickly add LLMs** without writing integration logic?
- You're **trapped in tedious prompting work** to clarify inputs and user intents?
- You're **wasting cycles** choosing and integrating **code for observability** instead of it just happening transparently?

And you think to yourself, can't I move faster by focusing on higher-level objectives in a language and framework agnostic way? Well, you can!

.. figure:: /_static/img/arch_network_diagram_high_level.png
   :width: 100%
   :align: center

   High-level network flow of where Arch Gateway sits in your agentic stack. Designed for both ingress and egress prompt traffic.


**Arch Gateway was built by the contributors of Envoy Proxy with the belief that:**

  *Prompts are nuanced and opaque user requests, which require the same capabilities as traditional HTTP requests
  including secure handling, intelligent routing, robust observability, and integration with backend (API)
  systems for personalization - all outside business logic.*

In practice, achieving the above goal is incredibly difficult. Arch attempts to do so by providing the following high level features:

**Out-of-process architecture, built on** `Envoy <http://envoyproxy.io/>`_:
Arch takes a dependency on Envoy and is a self-contained process that is designed to run alongside your application servers.
Arch uses Envoy's HTTP connection management subsystem, HTTP L7 filtering and telemetry capabilities to extend the functionality exclusively for prompts and LLMs.
This gives Arch several advantages:

* Arch builds on Envoy's proven success. Envoy is used at massive scale by the leading technology companies of our time including `AirBnB <https://www.airbnb.com>`_, `Dropbox <https://www.dropbox.com>`_, `Google <https://www.google.com>`_, `Reddit <https://www.reddit.com>`_, `Stripe <https://www.stripe.com>`_, etc. Its battle tested and scales linearly with usage and enables developers to focus on what really matters: application features and business logic.

* Arch works with any application language. A single Arch deployment can act as gateway for AI applications written in Python, Java, C++, Go, Php, etc.

* Arch can be deployed and upgraded quickly across your infrastructure transparently without the horrid pain of deploying library upgrades in your applications.

**Engineered with Fast LLMs:** Arch is engineered with specialized small LLMs that are designed for fast, cost-effective and accurate handling of prompts.
These LLMs are designed to be best-in-class for critical prompt-related tasks like:

* **Function Calling:** Arch helps you easily personalize your applications by enabling calls to application-specific (API) operations via user prompts.
  This involves any predefined functions or APIs you want to expose to users to perform tasks, gather information, or manipulate data.
  With function calling, you have flexibility to support "agentic" experiences tailored to specific use cases - from updating insurance claims to creating ad campaigns - via prompts.
  Arch analyzes prompts, extracts critical information from prompts, engages in lightweight conversation to gather any missing parameters and makes API calls so that you can focus on writing business logic.
  For more details, read :ref:`Function Calling <function_calling>`.

* **Prompt Guard:** Arch helps you improve the safety of your application by applying prompt guardrails in a centralized way for better governance hygiene.
  With prompt guardrails you can prevent ``jailbreak attempts`` present in user's prompts without having to write a single line of code.
  To learn more about how to configure guardrails available in Arch, read :ref:`Prompt Guard <prompt_guard>`.

**Traffic Management:** Arch offers several capabilities for LLM calls originating from your applications, including smart retries on errors from upstream LLMs, and automatic cut-over to other LLMs configured in Arch for continuous availability and disaster recovery scenarios.
Arch extends Envoy's `cluster subsystem <https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/upstream/cluster_manager>`_ to manage upstream connections to LLMs so that you can build resilient AI applications.

**Front/edge Gateway:** There is substantial benefit in using the same software at the edge (observability, traffic shaping algorithms, applying guardrails, etc.) as for outbound LLM inference use cases.
Arch has the feature set that makes it exceptionally well suited as an edge gateway for AI applications.
This includes TLS termination, applying guardrail early in the process, intelligent parameter gathering from prompts, and prompt-based routing to backend APIs.

**Best-In Class Monitoring:** Arch offers several monitoring metrics that help you understand three critical aspects of
your application: latency, token usage, and error rates by an upstream LLM provider. Latency measures the speed at which
your application is responding to users, which includes metrics like time to first token (TFT), time per output token (TOT)
metrics, and the total latency as perceived by users.

**End-to-End Tracing:** Arch propagates trace context using the W3C Trace Context standard, specifically through the ``traceparent`` header.
This allows each component in the system to record its part of the request flow, enabling end-to-end tracing across the entire application.
By using OpenTelemetry, Arch ensures that developers can capture this trace data consistently and in a format compatible with various observability tools.
For more details, read :ref:`Tracing <arch_overview_tracing>`.
