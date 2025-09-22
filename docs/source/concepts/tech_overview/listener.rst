.. _arch_overview_listeners:

Listener
---------
**Listener** is a top level primitive in Arch, which simplifies the configuration required to bind incoming
connections from downstream clients, and for egress connections to LLMs (hosted or API)

Arch builds on Envoy's Listener subsystem to streamline connection management for developers. Arch minimizes
the complexity of Envoy's listener setup by using best-practices and exposing only essential settings,
making it easier for developers to bind connections without deep knowledge of Envoy’s configuration model. This
simplification ensures that connections are secure, reliable, and optimized for performance.

Downstream (Ingress)
^^^^^^^^^^^^^^^^^^^^^^
Developers can configure Arch to accept connections from downstream clients. A downstream listener acts as the
primary entry point for incoming traffic, handling initial connection setup, including network filtering, guardrails,
and additional network security checks. For more details on prompt security and safety,
see :ref:`here <arch_overview_prompt_handling>`.

Upstream (Egress)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^
Arch automatically configures a listener to route requests from your application to upstream LLM API providers (or hosts).
When you start Arch, it creates a listener for egress traffic based on the presence of the ``listener`` configuration
section in the configuration file. Arch binds itself to a local address such as ``127.0.0.1:12000/v1`` or a DNS-based
address like ``arch.local:12000/v1`` for outgoing traffic. For more details on LLM providers, read :ref:`here <llm_providers>`.

Configure Listener
^^^^^^^^^^^^^^^^^^

To configure a Downstream (Ingress) Listener, simply add the ``listener`` directive to your configuration file:

.. literalinclude:: ../includes/arch_config.yaml
    :language: yaml
    :linenos:
    :lines: 1-18
    :emphasize-lines: 3-7
    :caption: Example Configuration
