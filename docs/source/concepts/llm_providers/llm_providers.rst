.. _llm_providers:

LLM Providers
=============
**LLM Providers** are a top-level primitive in Arch, helping developers centrally define, secure, observe,
and manage the usage of their LLMs. Arch builds on Envoy's reliable `cluster subsystem <https://www.envoyproxy.io/docs/envoy/v1.31.2/intro/arch_overview/upstream/cluster_manager>`_
to manage egress traffic to LLMs, which includes intelligent routing, retry and fail-over mechanisms,
ensuring high availability and fault tolerance. This abstraction also enables developers to seamlessly
switch between LLM providers or upgrade LLM versions, simplifying the integration and scaling of LLMs
across applications.

Today, we are enabling you to connect to 11+ different AI providers through a unified interface with advanced routing and management capabilities.
Whether you're using OpenAI, Anthropic, Azure OpenAI, local Ollama models, or any OpenAI-compatible provider, Arch provides seamless integration with enterprise-grade features.

Core Capabilities
-----------------

**Multi-Provider Support**
Connect to any combination of providers simultaneously:

- **First-Class Providers**: Native integrations with OpenAI, Anthropic, DeepSeek, Mistral, Groq, Google Gemini, Together AI, xAI, Azure OpenAI, and Ollama
- **OpenAI-Compatible Providers**: Support for any provider implementing OpenAI's API interface

**Intelligent Routing**
Two powerful routing approaches to optimize model selection:

- **Static Model Selection**: Direct routing using provider names or semantic model aliases
- **Preference-Aligned Dynamic Routing**: Intelligent, context-aware routing using the Arch-Router model that analyzes prompts and selects optimal models based on domain and action preferences

**Model Aliases & Management**
Create semantic, version-controlled names for simplified model management:

- **Semantic Naming**: Use descriptive names like ``fast-model``, ``reasoning-model``, or ``arch.summarize.v1``
- **Environment Management**: Different aliases for dev/staging/production environments
- **Version Control**: Implement versioning schemes for gradual model upgrades
- **Future Features**: Planned support for guardrails, fallback chains, and traffic splitting

**Unified Client Interface**
Use your preferred client library without changing existing code:

- **OpenAI Python SDK**: Full compatibility with all providers
- **Anthropic Python SDK**: Native support with cross-provider capabilities
- **cURL & HTTP Clients**: Direct REST API access for any programming language
- **Custom Integrations**: Standard HTTP interfaces for seamless integration

Key Benefits
------------

- **Provider Flexibility**: Switch between providers without changing client code
- **Intelligent Routing**: Automatically select the best model for each request
- **Cost Optimization**: Route requests to cost-effective models based on complexity
- **Performance Optimization**: Use fast models for simple tasks, powerful models for complex reasoning
- **Environment Management**: Configure different models for different environments
- **Future-Proof**: Easy to add new providers and upgrade models

Getting Started
---------------
Dive into specific areas based on your needs:

.. toctree::
  :maxdepth: 2

  supported_providers
  client_libraries
  model_aliases

**3. Advanced Features**
- **:ref:`llm_router`**: Learn about preference-aligned dynamic routing and intelligent model selection

Common Use Cases
----------------

**Development Teams**
- Use aliases like ``dev.chat.v1`` and ``prod.chat.v1`` for environment-specific models
- Route simple queries to fast/cheap models, complex tasks to powerful models
- Test new models safely using canary deployments (coming soon)

**Production Applications**
- Implement fallback strategies across multiple providers for reliability
- Use intelligent routing to optimize cost and performance automatically
- Monitor usage patterns and model performance across providers

**Enterprise Deployments**
- Connect to both cloud providers and on-premises models (Ollama, custom deployments)
- Apply consistent security and governance policies across all providers
- Scale across regions using different provider endpoints

Next Steps
----------

1. **:ref:`supported_providers`** - See all supported providers, models, and configuration examples
2. **:ref:`client_libraries`** - Start using with your preferred client
3. **:ref:`model_aliases`** - Create semantic model names
4. **:ref:`llm_router`** - Set up intelligent routing
