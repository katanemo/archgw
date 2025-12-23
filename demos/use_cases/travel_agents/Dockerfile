FROM python:3.11-slim

WORKDIR /app

# Install uv for faster dependency management
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Copy dependency files
COPY pyproject.toml README.md ./

# Install dependencies (without lock file to resolve fresh)
RUN uv sync --no-dev

# Copy application code
COPY src/ ./src/

# Set environment variables
ENV PYTHONUNBUFFERED=1

# Default command (will be overridden in docker-compose)
CMD ["uv", "run", "python", "src/travel_agents/weather_agent.py"]
