.. _deployment:

Deployment
==========

This guide shows how to deploy Arch directly using Docker without the archgw CLI, including basic runtime checks for routing and health monitoring.

Docker Deployment
-----------------

Below is a minimal, production-ready example showing how to deploy the Arch Docker image directly and run basic runtime checks. Adjust image names, tags, and the ``arch_config.yaml`` path to match your environment.

.. note::
   You will need to pass all required environment variables that are referenced in your ``arch_config.yaml`` file.

For ``arch_config.yaml``, you can use any sample configuration defined earlier in the documentation. For example, you can try the :ref:`LLM Routing <llm_router>` sample config.

Docker Compose Setup
~~~~~~~~~~~~~~~~~~~~

Create a ``docker-compose.yml`` file with the following configuration:

.. code-block:: yaml

   # docker-compose.yml
   services:
     archgw:
       image: katanemo/archgw:0.3.15
       container_name: archgw
       ports:
         - "10000:10000" # ingress (client -> arch)
         - "12000:12000" # egress (arch -> upstream/llm proxy)
       volumes:
         - ./arch_config.yaml:/app/arch_config.yaml:ro
       environment:
         - OPENAI_API_KEY=${OPENAI_API_KEY:?error}
         - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:?error}
         - MODEL_SERVER_PORT=51000

Starting the Stack
~~~~~~~~~~~~~~~~~~

Start the services from the directory containing ``docker-compose.yml`` and ``arch_config.yaml``:

.. code-block:: bash

   # Set required environment variables and start services
   OPENAI_API_KEY=xxx ANTHROPIC_API_KEY=yyy docker compose up -d

Check container health and logs:

.. code-block:: bash

   docker compose ps
   docker compose logs -f archgw

Runtime Tests
-------------

Perform basic runtime tests to verify routing and functionality.

Gateway Smoke Test
~~~~~~~~~~~~~~~~~~

Test the chat completion endpoint with automatic routing:

.. code-block:: bash

   # Request handled by the gateway. 'model: "none"' lets Arch decide routing
   curl --header 'Content-Type: application/json' \
     --data '{"messages":[{"role":"user","content":"tell me a joke"}], "model":"none"}' \
     http://localhost:12000/v1/chat/completions | jq .model

Expected output:

.. code-block:: json

   "gpt-4o-2024-08-06"

Model-Based Routing
~~~~~~~~~~~~~~~~~~~

Test explicit provider and model routing:

.. code-block:: bash

   curl -s -H "Content-Type: application/json" \
     -d '{"messages":[{"role":"user","content":"Explain quantum computing"}], "model":"anthropic/claude-3-5-sonnet-20241022"}' \
     http://localhost:12000/v1/chat/completions | jq .model

Expected output:

.. code-block:: json

   "claude-3-5-sonnet-20241022"

Troubleshooting
---------------

Common Issues and Solutions
~~~~~~~~~~~~~~~~~~~~~~~~~~~

**Environment Variables**
   Ensure all environment variables (``OPENAI_API_KEY``, ``ANTHROPIC_API_KEY``, etc.) used by ``arch_config.yaml`` are set before starting services.

**TLS/Connection Errors**
   If you encounter TLS or connection errors to upstream providers:

   - Check DNS resolution
   - Verify proxy settings
   - Confirm correct protocol and port in your ``arch_config`` endpoints

**Verbose Logging**
   To enable more detailed logs for debugging:

   - Run archgw with a higher component log level
   - See the :ref:`Observability <observability>` guide for logging and monitoring details
   - Rebuild the image if required with updated log configuration

**CI/Automated Checks**
   For continuous integration or automated testing, you can use the curl commands above as health checks in your deployment pipeline.
