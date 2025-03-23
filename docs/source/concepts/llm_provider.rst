.. _llm_provider:

LLM Provider
============

**LLM provider** is a top-level primitive in Arch, helping developers centrally define, secure, observe,
and manage the usage of their LLMs. Arch builds on Envoy's reliable `cluster subsystem <https://www.envoyproxy.io/docs/envoy/v1.31.2/intro/arch_overview/upstream/cluster_manager>`_
to manage egress traffic to LLMs, which includes intelligent routing, retry and fail-over mechanisms,
ensuring high availability and fault tolerance. This abstraction also enables developers to seamlessly
switching between LLM providers or upgrade LLM versions, simplifying the integration and scaling of LLMs
across applications.


Below is an example of how you can configure ``llm_providers`` with an instance of an Arch gateway.

.. literalinclude:: includes/arch_config.yaml
    :language: yaml
    :linenos:
    :lines: 1-20
    :emphasize-lines: 10-16
    :caption: Example Configuration

.. Note::
    When you start Arch, it creates a listener port for egress traffic based on the presence of ``llm_providers``
    configuration section in the ``arch_config.yml`` file. Arch binds itself to a local address such as
    ``127.0.0.1:12000``.

Arch also offers vendor-agnostic SDKs and libraries to make LLM calls to API-based LLM providers (like OpenAI,
Anthropic, Mistral, Cohere, etc.) and supports calls to OSS LLMs that are hosted on your infrastructure. Arch
abstracts the complexities of integrating with different LLM providers, providing a unified interface for making
calls, handling retries, managing rate limits, and ensuring seamless integration with cloud-based and on-premise
LLMs. Simply configure the details of the LLMs your application will use, and Arch offers a unified interface to
make outbound LLM calls.

Adding custom LLM Provider
--------------------------

We support any OpenAI compliant LLM for example mistral, openai, ollama etc. We offer first class support for openai and ollama. You can easily configure an LLM that communicates over the OpenAI API interface, by following the below guide.

For example following code block shows you how to add an ollama-supported LLM in the `arch_config.yaml` file.

.. code-block:: yaml

    - name: local-llama
      provider_interface: openai
      model: llama3.2
      endpoint: host.docker.internal:11434


For example following code block shows you how to add mistral llm provider in the `arch_config.yaml` file.

.. code-block:: yaml

    - name: mistral-ai
      provider_interface: openai
      model: ministral-3b-latest
      endpoint: api.mistral.ai:443
      protocol: https


Example: Using the OpenAI Python SDK
------------------------------------

.. code-block:: python

   from openai import OpenAI

   # Initialize the Arch client
   client = OpenAI(base_url="http://127.0.0.12000/")

   # Define your LLM provider and prompt
   llm_provider = "openai"
   prompt = "What is the capital of France?"

   # Send the prompt to the LLM through Arch
   response = client.completions.create(llm_provider=llm_provider, prompt=prompt)

   # Print the response
   print("LLM Response:", response)
