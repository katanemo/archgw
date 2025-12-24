.. _bright_staff:

Bright Staff
============

Bright Staff is Plano's memory-efficient, lightweight controller for agentic traffic. It sits inside the Plano
data plane and makes real-time decisions about how prompts are handled, forwarded, and processed.

Rather than running a separate "model server" subsystem, Plano relies on Envoy's HTTP connection management
and cluster subsystem to talk to different models and backends over HTTP(S). Bright Staff uses these primitives to:
* Inspect prompts, conversation state, and metadata.
* Decide which upstream model(s), tool backends, or APIs to call, and in what order.
* Coordinate retries, fallbacks, and traffic splitting across providers and models.

Plano is designed to run alongside your application servers in your cloud VPC, on-premises, or in local
development. It does not require a GPU itself; GPUs live where your models are hosted (third-party APIs or your
own deployments), and Plano reaches them via HTTP.

.. image:: /_static/img/plano-system-architecture.png
    :align: center
    :width: 40%
