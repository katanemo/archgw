# Model Server Package #
This model server package is a dependency of the Arch intelligent prompt gateway. It should not be used alone. Please refer to the [quickstart-guide](https://github.com/katanemo/arch?tab=readme-ov-file#quickstart) for more details on how to get start with Arch.

## Local development

You can start/stop the local server via the CLI entry point exposed by this package.

Using uv (recommended):

```sh
uv run model_server --help
# run in foreground (stays attached until Ctrl+C)
uv run model_server start --port 51000 --foreground
# run in background (then stop using the CLI)
uv run model_server start --port 51000
uv run model_server stop
```

Alternative without uv:

```sh
python -m src.cli --help
# foreground
python -m src.cli start --port 51000 --foreground
# background
python -m src.cli start --port 51000
python -m src.cli stop
```

The FastAPI app lives at `src.main:app` and exposes a health check at `/healthz`.
