import click
import os
import sys
import subprocess
import multiprocessing
import importlib.metadata
import json
from cli import targets
from cli.docker_cli import (
    docker_validate_archgw_schema,
    stream_gateway_logs,
    docker_container_status,
)
from cli.utils import (
    getLogger,
    get_llm_provider_access_keys,
    has_ingress_listener,
    load_env_file_to_dict,
    stream_access_logs,
    find_config_file,
)
from cli.core import (
    start_arch,
    stop_docker_container,
    start_cli_agent,
)
from cli.consts import (
    ARCHGW_DOCKER_IMAGE,
    ARCHGW_DOCKER_NAME,
    SERVICE_NAME_ARCHGW,
)

log = getLogger(__name__)

logo = r"""
     _                _
    / \    _ __  ___ | |__
   / _ \  | '__|/ __|| '_ \
  / ___ \ | |  | (__ | | | |
 /_/   \_\|_|   \___||_| |_|

"""

# Command to build archgw Docker images
ARCHGW_DOCKERFILE = "./arch/Dockerfile"


def get_version():
    try:
        version = importlib.metadata.version("archgw")
        return version
    except importlib.metadata.PackageNotFoundError:
        return "version not found"


@click.group(invoke_without_command=True)
@click.option("--version", is_flag=True, help="Show the archgw cli version and exit.")
@click.pass_context
def main(ctx, version):
    if version:
        click.echo(f"archgw cli version: {get_version()}")
        ctx.exit()

    log.info(f"Starting archgw cli version: {get_version()}")

    if ctx.invoked_subcommand is None:
        click.echo("""Arch (The Intelligent Prompt Gateway) CLI""")
        click.echo(logo)
        click.echo(ctx.get_help())


@click.command()
def build():
    """Build Arch from source. Must be in root of cloned repo."""

    # Check if /arch/Dockerfile exists
    if os.path.exists(ARCHGW_DOCKERFILE):
        if os.path.exists(ARCHGW_DOCKERFILE):
            click.echo("Building archgw image...")
            try:
                subprocess.run(
                    [
                        "docker",
                        "build",
                        "-f",
                        ARCHGW_DOCKERFILE,
                        "-t",
                        f"{ARCHGW_DOCKER_IMAGE}",
                        ".",
                        "--add-host=host.docker.internal:host-gateway",
                    ],
                    check=True,
                )
                click.echo("archgw image built successfully.")
            except subprocess.CalledProcessError as e:
                click.echo(f"Error building archgw image: {e}")
                sys.exit(1)
        else:
            click.echo("Error: Dockerfile not found in /arch")
            sys.exit(1)

    click.echo("archgw image built successfully.")


@click.command()
@click.argument("file", required=False)  # Optional file argument
@click.option(
    "--path", default=".", help="Path to the directory containing arch_config.yaml"
)
@click.option(
    "--foreground",
    default=False,
    help="Run Arch in the foreground. Default is False",
    is_flag=True,
)
def up(file, path, foreground):
    """Starts Arch."""
    # Use the utility function to find config file
    arch_config_file = find_config_file(path, file)

    # Check if the file exists
    if not os.path.exists(arch_config_file):
        log.info(f"Error: {arch_config_file} does not exist.")
        return

    log.info(f"Validating {arch_config_file}")
    (
        validation_return_code,
        validation_stdout,
        validation_stderr,
    ) = docker_validate_archgw_schema(arch_config_file)
    if validation_return_code != 0:
        log.info(f"Error: Validation failed. Exiting")
        log.info(f"Validation stdout: {validation_stdout}")
        log.info(f"Validation stderr: {validation_stderr}")
        sys.exit(1)

    # Set the ARCH_CONFIG_FILE environment variable
    env_stage = {
        "OTEL_TRACING_HTTP_ENDPOINT": "http://host.docker.internal:4318/v1/traces",
    }
    env = os.environ.copy()
    # Remove PATH variable if present
    env.pop("PATH", None)
    # check if access_keys are preesnt in the config file
    access_keys = get_llm_provider_access_keys(arch_config_file=arch_config_file)

    # remove duplicates
    access_keys = set(access_keys)
    # remove the $ from the access_keys
    access_keys = [item[1:] if item.startswith("$") else item for item in access_keys]

    if access_keys:
        if file:
            app_env_file = os.path.join(
                os.path.dirname(os.path.abspath(file)), ".env"
            )  # check the .env file in the path
        else:
            app_env_file = os.path.abspath(os.path.join(path, ".env"))

        if not os.path.exists(
            app_env_file
        ):  # check to see if the environment variables in the current environment or not
            for access_key in access_keys:
                if env.get(access_key) is None:
                    log.info(f"Access Key: {access_key} not found. Exiting Start")
                    sys.exit(1)
                else:
                    env_stage[access_key] = env.get(access_key)
        else:  # .env file exists, use that to send parameters to Arch
            env_file_dict = load_env_file_to_dict(app_env_file)
            for access_key in access_keys:
                if env_file_dict.get(access_key) is None:
                    log.info(f"Access Key: {access_key} not found. Exiting Start")
                    sys.exit(1)
                else:
                    env_stage[access_key] = env_file_dict[access_key]

    env.update(env_stage)
    start_arch(arch_config_file, env, foreground=foreground)


@click.command()
def down():
    """Stops Arch."""
    stop_docker_container()


@click.command()
@click.option(
    "--f",
    "--file",
    type=click.Path(exists=True),
    required=True,
    help="Path to the Python file",
)
def generate_prompt_targets(file):
    """Generats prompt_targets from python methods.
    Note: This works for simple data types like ['int', 'float', 'bool', 'str', 'list', 'tuple', 'set', 'dict']:
    If you have a complex pydantic data type, you will have to flatten those manually until we add support for it.
    """

    print(f"Processing file: {file}")
    if not file.endswith(".py"):
        print("Error: Input file must be a .py file")
        sys.exit(1)

    targets.generate_prompt_targets(file)


@click.command()
@click.option(
    "--debug",
    help="For detailed debug logs to trace calls from archgw <> api_server, etc",
    is_flag=True,
)
@click.option("--follow", help="Follow the logs", is_flag=True)
def logs(debug, follow):
    """Stream logs from access logs services."""

    archgw_process = None
    try:
        if debug:
            archgw_process = multiprocessing.Process(
                target=stream_gateway_logs, args=(follow,)
            )
            archgw_process.start()

        archgw_access_logs_process = multiprocessing.Process(
            target=stream_access_logs, args=(follow,)
        )
        archgw_access_logs_process.start()
        archgw_access_logs_process.join()

        if archgw_process:
            archgw_process.join()
    except KeyboardInterrupt:
        log.info("KeyboardInterrupt detected. Exiting.")
        if archgw_access_logs_process.is_alive():
            archgw_access_logs_process.terminate()
        if archgw_process and archgw_process.is_alive():
            archgw_process.terminate()


@click.command()
@click.argument("type", type=click.Choice(["claude"]), required=True)
@click.argument("file", required=False)  # Optional file argument
@click.option(
    "--path", default=".", help="Path to the directory containing arch_config.yaml"
)
@click.option(
    "--settings",
    default="{}",
    help="Additional settings as JSON string for the CLI agent.",
)
def cli_agent(type, file, path, settings):
    """Start a CLI agent connected to Arch.

    CLI_AGENT: The type of CLI agent to start (currently only 'claude' is supported)
    """

    # Check if archgw docker container is running
    archgw_status = docker_container_status(ARCHGW_DOCKER_NAME)
    if archgw_status != "running":
        log.error(f"archgw docker container is not running (status: {archgw_status})")
        log.error("Please start archgw using the 'archgw up' command.")
        sys.exit(1)

    # Determine arch_config.yaml path
    arch_config_file = find_config_file(path, file)
    if not os.path.exists(arch_config_file):
        log.error(f"Config file not found: {arch_config_file}")
        sys.exit(1)

    try:
        start_cli_agent(arch_config_file, settings)
    except SystemExit:
        # Re-raise SystemExit to preserve exit codes
        raise
    except Exception as e:
        click.echo(f"Error: {e}")
        sys.exit(1)


main.add_command(up)
main.add_command(down)
main.add_command(build)
main.add_command(logs)
main.add_command(cli_agent)
main.add_command(generate_prompt_targets)

if __name__ == "__main__":
    main()
