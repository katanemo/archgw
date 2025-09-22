.. _supported_providers:

Supported Providers & Configuration
===================================

Arch provides first-class support for multiple LLM providers through native integrations and OpenAI-compatible interfaces. This comprehensive guide covers all supported providers, their available chat models, and detailed configuration instructions.

.. note::
   **Model Support:** Arch supports all chat models from each provider, not just the examples shown in this guide. The configurations below demonstrate common models for reference, but you can use any chat model available from your chosen provider.

Configuration Structure
-----------------------

All providers are configured in the ``llm_providers`` section of your ``arch_config.yaml`` file:

.. code-block:: yaml

    version: v0.1

    listeners:
      egress_traffic:
        address: 0.0.0.0
        port: 12000
        message_format: openai
        timeout: 30s

    llm_providers:
      # Provider configurations go here
      - model: provider/model-name
        access_key: $API_KEY
        # Additional provider-specific options

**Common Configuration Fields:**

- ``model``: Provider prefix and model name (format: ``provider/model-name``)
- ``access_key``: API key for authentication (supports environment variables)
- ``default``: Mark a model as the default (optional, boolean)
- ``name``: Custom name for the provider instance (optional)
- ``base_url``: Custom endpoint URL (required for some providers)

Provider Categories
-------------------

**First-Class Providers**
Native integrations with built-in support for provider-specific features and authentication.

**OpenAI-Compatible Providers**
Any provider that implements the OpenAI API interface can be configured using custom endpoints.

Supported API Endpoints
------------------------

Arch supports the following standardized endpoints across providers:

.. list-table::
   :header-rows: 1
   :widths: 30 30 40

   * - Endpoint
     - Purpose
     - Supported Clients
   * - ``/v1/chat/completions``
     - OpenAI-style chat completions
     - OpenAI SDK, cURL, custom clients
   * - ``/v1/messages``
     - Anthropic-style messages
     - Anthropic SDK, cURL, custom clients

First-Class Providers
---------------------

OpenAI
~~~~~~

**Provider Prefix:** ``openai/``

**API Endpoint:** ``/v1/chat/completions``

**Authentication:** API Key - Get your OpenAI API key from `OpenAI Platform <https://platform.openai.com/api-keys>`_.

**Supported Chat Models:** All OpenAI chat models including GPT-5, GPT-4o, GPT-4, GPT-3.5-turbo, and all future releases.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - GPT-5
     - ``openai/gpt-5``
     - Next-generation model (use any model name from OpenAI's API)
   * - GPT-4o
     - ``openai/gpt-4o``
     - Latest multimodal model
   * - GPT-4o mini
     - ``openai/gpt-4o-mini``
     - Fast, cost-effective model
   * - GPT-4
     - ``openai/gpt-4``
     - High-capability reasoning model
   * - GPT-3.5 Turbo
     - ``openai/gpt-3.5-turbo``
     - Balanced performance and cost
   * - o3-mini
     - ``openai/o3-mini``
     - Reasoning-focused model (preview)
   * - o3
     - ``openai/o3``
     - Advanced reasoning model (preview)

**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      # Latest models (examples - use any OpenAI chat model)
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_API_KEY
        default: true

      - model: openai/gpt-4o
        access_key: $OPENAI_API_KEY

      # Use any model name from OpenAI's API
      - model: openai/gpt-5
        access_key: $OPENAI_API_KEY

Anthropic
~~~~~~~~~

**Provider Prefix:** ``anthropic/``

**API Endpoint:** ``/v1/messages``

**Authentication:** API Key - Get your Anthropic API key from `Anthropic Console <https://console.anthropic.com/settings/keys>`_.

**Supported Chat Models:** All Anthropic Claude models including Claude Sonnet 4, Claude 3.5 Sonnet, Claude 3.5 Haiku, Claude 3 Opus, and all future releases.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - Claude Sonnet 4
     - ``anthropic/claude-sonnet-4``
     - Next-generation model (use any model name from Anthropic's API)
   * - Claude 3.5 Sonnet
     - ``anthropic/claude-3-5-sonnet-20241022``
     - Latest high-performance model
   * - Claude 3.5 Haiku
     - ``anthropic/claude-3-5-haiku-20241022``
     - Fast and efficient model
   * - Claude 3 Opus
     - ``anthropic/claude-3-opus-20240229``
     - Most capable model for complex tasks
   * - Claude 3 Sonnet
     - ``anthropic/claude-3-sonnet-20240229``
     - Balanced performance model
   * - Claude 3 Haiku
     - ``anthropic/claude-3-haiku-20240307``
     - Fastest model

**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      # Latest models (examples - use any Anthropic chat model)
      - model: anthropic/claude-3-5-sonnet-20241022
        access_key: $ANTHROPIC_API_KEY

      - model: anthropic/claude-3-5-haiku-20241022
        access_key: $ANTHROPIC_API_KEY

      # Use any model name from Anthropic's API
      - model: anthropic/claude-sonnet-4
        access_key: $ANTHROPIC_API_KEY

DeepSeek
~~~~~~~~

**Provider Prefix:** ``deepseek/``

**API Endpoint:** ``/v1/chat/completions``

**Authentication:** API Key - Get your DeepSeek API key from `DeepSeek Platform <https://platform.deepseek.com/api_keys>`_.

**Supported Chat Models:** All DeepSeek chat models including DeepSeek-Chat, DeepSeek-Coder, and all future releases.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - DeepSeek Chat
     - ``deepseek/deepseek-chat``
     - General purpose chat model
   * - DeepSeek Coder
     - ``deepseek/deepseek-coder``
     - Code-specialized model

**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      - model: deepseek/deepseek-chat
        access_key: $DEEPSEEK_API_KEY

      - model: deepseek/deepseek-coder
        access_key: $DEEPSEEK_API_KEY

Mistral AI
~~~~~~~~~~

**Provider Prefix:** ``mistral/``

**API Endpoint:** ``/v1/chat/completions``

**Authentication:** API Key - Get your Mistral API key from `Mistral AI Console <https://console.mistral.ai/api-keys/>`_.

**Supported Chat Models:** All Mistral chat models including Mistral Large, Mistral Small, Ministral, and all future releases.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - Mistral Large
     - ``mistral/mistral-large-latest``
     - Most capable model
   * - Mistral Medium
     - ``mistral/mistral-medium-latest``
     - Balanced performance
   * - Mistral Small
     - ``mistral/mistral-small-latest``
     - Fast and efficient
   * - Ministral 3B
     - ``mistral/ministral-3b-latest``
     - Compact model

**Configuration Examples:**
**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      - model: mistral/mistral-large-latest
        access_key: $MISTRAL_API_KEY

      - model: mistral/mistral-small-latest
        access_key: $MISTRAL_API_KEY

Groq
~~~~

**Provider Prefix:** ``groq/``

**API Endpoint:** ``/openai/v1/chat/completions`` (transformed internally)

**Authentication:** API Key - Get your Groq API key from `Groq Console <https://console.groq.com/keys>`_.

**Supported Chat Models:** All Groq chat models including Llama 3, Mixtral, Gemma, and all future releases.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - Llama 3.1 8B
     - ``groq/llama3-8b-8192``
     - Fast inference Llama model
   * - Llama 3.1 70B
     - ``groq/llama3-70b-8192``
     - Larger Llama model
   * - Mixtral 8x7B
     - ``groq/mixtral-8x7b-32768``
     - Mixture of experts model

**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      - model: groq/llama3-8b-8192
        access_key: $GROQ_API_KEY

      - model: groq/mixtral-8x7b-32768
        access_key: $GROQ_API_KEY

Google Gemini
~~~~~~~~~~~~~

**Provider Prefix:** ``gemini/``

**API Endpoint:** ``/v1beta/openai/chat/completions`` (transformed internally)

**Authentication:** API Key - Get your Google AI API key from `Google AI Studio <https://aistudio.google.com/app/apikey>`_.

**Supported Chat Models:** All Google Gemini chat models including Gemini 1.5 Pro, Gemini 1.5 Flash, and all future releases.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - Gemini 1.5 Pro
     - ``gemini/gemini-1.5-pro``
     - Advanced reasoning and creativity
   * - Gemini 1.5 Flash
     - ``gemini/gemini-1.5-flash``
     - Fast and efficient model

**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      - model: gemini/gemini-1.5-pro
        access_key: $GOOGLE_API_KEY

      - model: gemini/gemini-1.5-flash
        access_key: $GOOGLE_API_KEY

Together AI
~~~~~~~~~~~

**Provider Prefix:** ``together_ai/``

**API Endpoint:** ``/v1/chat/completions``

**Authentication:** API Key - Get your Together AI API key from `Together AI Settings <https://api.together.xyz/settings/api-keys>`_.

**Supported Chat Models:** All Together AI chat models including Llama, CodeLlama, Mixtral, Qwen, and hundreds of other open-source models.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - Meta Llama 2 7B
     - ``together_ai/meta-llama/Llama-2-7b-chat-hf``
     - Open source chat model
   * - Meta Llama 2 13B
     - ``together_ai/meta-llama/Llama-2-13b-chat-hf``
     - Larger open source model
   * - Code Llama 34B
     - ``together_ai/codellama/CodeLlama-34b-Instruct-hf``
     - Code-specialized model

**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      - model: together_ai/meta-llama/Llama-2-7b-chat-hf
        access_key: $TOGETHER_API_KEY

      - model: together_ai/codellama/CodeLlama-34b-Instruct-hf
        access_key: $TOGETHER_API_KEY

xAI
~~~

**Provider Prefix:** ``xai/``

**API Endpoint:** ``/v1/chat/completions``

**Authentication:** API Key - Get your xAI API key from `xAI Console <https://console.x.ai/>`_.

**Supported Chat Models:** All xAI chat models including Grok Beta and all future releases.

.. list-table::
   :header-rows: 1
   :widths: 30 20 50

   * - Model Name
     - Model ID for Config
     - Description
   * - Grok Beta
     - ``xai/grok-beta``
     - Conversational AI model

**Configuration Examples:**

.. code-block:: yaml

    llm_providers:
      - model: xai/grok-beta
        access_key: $XAI_API_KEY

Providers Requiring Base URL
----------------------------

Azure OpenAI
~~~~~~~~~~~~

**Provider Prefix:** ``azure_openai/``

**API Endpoint:** ``/openai/deployments/{deployment-name}/chat/completions`` (constructed automatically)

**Authentication:** API Key + Base URL - Get your Azure OpenAI API key from `Azure Portal <https://portal.azure.com/>`_ → Your OpenAI Resource → Keys and Endpoint.

**Supported Chat Models:** All Azure OpenAI chat models including GPT-4o, GPT-4, GPT-3.5-turbo deployed in your Azure subscription.

.. code-block:: yaml

    llm_providers:
      # Single deployment
      - model: azure_openai/gpt-4o
        access_key: $AZURE_OPENAI_API_KEY
        base_url: https://your-resource.openai.azure.com

      # Multiple deployments
      - model: azure_openai/gpt-4o-mini
        access_key: $AZURE_OPENAI_API_KEY
        base_url: https://your-resource.openai.azure.com

Ollama
~~~~~~

**Provider Prefix:** ``ollama/``

**API Endpoint:** ``/v1/chat/completions`` (Ollama's OpenAI-compatible endpoint)

**Authentication:** None (Base URL only) - Install Ollama from `Ollama.com <https://ollama.com/>`_ and pull your desired models.

**Supported Chat Models:** All chat models available in your local Ollama installation. Use ``ollama list`` to see installed models.

.. code-block:: yaml

    llm_providers:
      # Local Ollama installation
      - model: ollama/llama3.1
        base_url: http://localhost:11434

      # Ollama in Docker (from host)
      - model: ollama/codellama
        base_url: http://host.docker.internal:11434

OpenAI-Compatible Providers
~~~~~~~~~~~~~~~~~~~~~~~~~~~

**Supported Models:** Any chat models from providers that implement the OpenAI Chat Completions API standard.

For providers that implement the OpenAI API but aren't natively supported:

.. code-block:: yaml

    llm_providers:
      # Generic OpenAI-compatible provider
      - model: custom-provider/custom-model
        base_url: https://api.customprovider.com
        provider_interface: openai
        access_key: $CUSTOM_API_KEY

      # Local deployment
      - model: local/llama2-7b
        base_url: http://localhost:8000
        provider_interface: openai

Advanced Configuration
----------------------

Multiple Provider Instances
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Configure multiple instances of the same provider:

.. code-block:: yaml

    llm_providers:
      # Production OpenAI
      - model: openai/gpt-4o
        access_key: $OPENAI_PROD_KEY
        name: openai-prod

      # Development OpenAI (different key/quota)
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_DEV_KEY
        name: openai-dev

Default Model Configuration
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Mark one model as the default for fallback scenarios:

.. code-block:: yaml

    llm_providers:
      - model: openai/gpt-4o-mini
        access_key: $OPENAI_API_KEY
        default: true  # Used when no specific model is requested

Routing Preferences
~~~~~~~~~~~~~~~~~~~

Configure routing preferences for dynamic model selection:

.. code-block:: yaml

    llm_providers:
      - model: openai/gpt-4o
        access_key: $OPENAI_API_KEY
        routing_preferences:
          - name: complex_reasoning
            description: deep analysis, mathematical problem solving, and logical reasoning
          - name: code_review
            description: reviewing and analyzing existing code for bugs and improvements

      - model: anthropic/claude-3-5-sonnet-20241022
        access_key: $ANTHROPIC_API_KEY
        routing_preferences:
          - name: creative_writing
            description: creative content generation, storytelling, and writing assistance

Model Selection Guidelines
--------------------------

**For Production Applications:**
- **High Performance**: OpenAI GPT-4o, Anthropic Claude 3.5 Sonnet
- **Cost-Effective**: OpenAI GPT-4o mini, Anthropic Claude 3.5 Haiku
- **Code Tasks**: DeepSeek Coder, Together AI Code Llama
- **Local Deployment**: Ollama with Llama 3.1 or Code Llama

**For Development/Testing:**
- **Fast Iteration**: Groq models (optimized inference)
- **Local Testing**: Ollama models
- **Cost Control**: Smaller models like GPT-4o mini or Mistral Small

See Also
--------

- :ref:`client_libraries` - Using different client libraries with providers
- :ref:`model_aliases` - Creating semantic model names
- :ref:`llm_router` - Setting up intelligent routing
- :ref:`client_libraries` - Using different client libraries
- :ref:`model_aliases` - Creating semantic model names
