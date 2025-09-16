# Weather Forecast Service Demo (Java)

This demo shows how to run the Weather Forecast Service using Arch Gateway and Docker Compose.

## Prerequisites
- Docker & Docker Compose
- `archgw` CLI installed and available in your PATH
- Set your OpenAI API key in your environment: `export OPENAI_API_KEY=your-key-here`

## Usage

### Start the Demo

```bash
./run_demo.sh
```

This will:
1. Check for a `.env` file. If not present, it will create one with your `OPENAI_API_KEY`.
2. Start Arch Gateway using `arch_config.yaml`.
3. Start the Network Agent and related services using Docker Compose.

### Stop the Demo

```bash
./run_demo.sh down
```

This will:
1. Stop Docker Compose services.
2. Stop Arch Gateway.

## Notes
- Make sure your `OPENAI_API_KEY` is set in your environment before running the demo.
- The script will create a `.env` file if it does not exist.
- All services are started/stopped using the provided shell script for convenience.
