# syntax=docker/dockerfile:1

# Base Python image (aligns with requires-python >=3.10)
FROM python:3.12-slim

# Install runtime deps and curl for installing uv
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       bash \
       curl \
       ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Astral's uv (Python package manager/runner used by start_agents.sh)
# Keep both common install paths in PATH so `uv` is available in the container
ENV PATH="/root/.local/bin:/root/.cargo/bin:${PATH}"
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Workdir for app
WORKDIR /app

# Copy project metadata and source first to leverage caching for dependency install
COPY pyproject.toml README.md /app/
COPY src/ /app/src/
COPY start_agents.sh /app/
COPY uv.lock /app/

# Pre-create a project venv and install deps into it for faster startup
# Using `uv sync` ensures the .venv contains all dependencies declared in pyproject.toml
RUN uv venv \
    && uv sync --frozen \
    && chmod +x /app/start_agents.sh

# Make venv the default Python for subsequent commands (optional; uv run will pick .venv automatically)
ENV VIRTUAL_ENV=/app/.venv
ENV PATH="/app/.venv/bin:${PATH}"

# Expose the three agent ports
EXPOSE 10500 10501 10502

# Default environment (can be overridden at `docker run` time)
# Example: point to an external archgw or a compose service name
# ENV LLM_GATEWAY_ENDPOINT="http://archgw:12000/v1"

# Run the multi-agent launcher script
ENTRYPOINT ["/app/start_agents.sh"]
