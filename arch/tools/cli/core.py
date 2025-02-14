import subprocess
import os
import time
import sys
import glob
import docker
from docker.errors import DockerException
from cli.utils import getLogger, update_docker_host_env
from cli.consts import (
    ARCHGW_DOCKER_IMAGE,
    ARCHGW_DOCKER_NAME,
    KATANEMO_LOCAL_MODEL_LIST,
    MODEL_SERVER_LOG_FILE,
    ACCESS_LOG_FILES,
)
from huggingface_hub import snapshot_download
from dotenv import dotenv_values
import yaml


log = getLogger(__name__)


def start_archgw_docker(
    client, arch_config_file, env, prompt_gateway_port, llm_gateway_port
):
    logs_path = "~/archgw_logs"
    logs_path_abs = os.path.expanduser(logs_path)

    return client.containers.run(
        name=ARCHGW_DOCKER_NAME,
        image=ARCHGW_DOCKER_IMAGE,
        detach=True,  # Run in detached mode
        ports={
            f"{prompt_gateway_port}/tcp": prompt_gateway_port,
            "10001/tcp": 10001,
            "11000/tcp": 11000,
            f"{llm_gateway_port}/tcp": llm_gateway_port,
            "9901/tcp": 19901,
        },
        volumes={
            f"{arch_config_file}": {
                "bind": "/app/arch_config.yaml",
                "mode": "ro",
            },
            "/etc/ssl/cert.pem": {"bind": "/etc/ssl/cert.pem", "mode": "ro"},
            logs_path_abs: {"bind": "/var/log"},
        },
        environment={
            "OTEL_TRACING_HTTP_ENDPOINT": "http://host.docker.internal:4318/v1/traces",
            "MODEL_SERVER_PORT": os.getenv("MODEL_SERVER_PORT", "51000"),
            **env,
        },
        extra_hosts={"host.docker.internal": "host-gateway"},
        healthcheck={
            "test": [
                "CMD",
                "curl",
                "-f",
                f"http://localhost:{prompt_gateway_port}/healthz",
            ],
            "interval": 5000000000,  # 5 seconds
            "timeout": 1000000000,  # 1 seconds
            "retries": 3,
        },
    )


def stream_gateway_logs(follow):
    """
    Stream logs from the arch gateway service.
    """
    log.info("Logs from arch gateway service.")

    options = ["docker", "logs", "archgw"]
    if follow:
        options.append("-f")
    try:
        # Run `docker-compose logs` to stream logs from the gateway service
        subprocess.run(
            options,
            check=True,
            stdout=sys.stdout,
            stderr=sys.stderr,
        )

    except subprocess.CalledProcessError as e:
        log.info(f"Failed to stream logs: {str(e)}")


def stream_access_logs(follow):
    """
    Get the archgw access logs
    """
    log_file_pattern_expanded = os.path.expanduser(ACCESS_LOG_FILES)
    log_files = glob.glob(log_file_pattern_expanded)

    stream_command = ["tail"]
    if follow:
        stream_command.append("-f")

    stream_command.extend(log_files)
    subprocess.run(
        stream_command,
        check=True,
        stdout=sys.stdout,
        stderr=sys.stderr,
    )


def start_arch(arch_config_file, env, log_timeout=120, foreground=False):
    """
    Start Docker Compose in detached mode and stream logs until services are healthy.

    Args:
        path (str): The path where the prompt_config.yml file is located.
        log_timeout (int): Time in seconds to show logs before checking for healthy state.
    """
    log.info("Starting arch gateway")

    try:
        try:
            client = docker.from_env()
        except DockerException as e:
            # try setting up the docker host environment variable and retry
            update_docker_host_env()
            client = docker.from_env()

        try:
            container = client.containers.get("archgw")
            log.info("archgw container found in docker, stopping and removing it")
            # ensure that previous docker container is stopped and removed
            container.stop()
            container.remove()
            log.info("Stopped and removed archgw container")
        except docker.errors.NotFound as e:
            pass

        # parse arch_config_file yaml file and get prompt_gateway_port
        arch_config_dict = {}
        with open(arch_config_file) as f:
            arch_config_dict = yaml.safe_load(f)

        prompt_gateway_port = (
            arch_config_dict.get("listeners", {})
            .get("prompt_gateway", {})
            .get("port", 10000)
        )
        llm_gateway_port = (
            arch_config_dict.get("listeners", {})
            .get("llm_gateway", {})
            .get("port", 12000)
        )

        container = start_archgw_docker(
            client, arch_config_file, env, prompt_gateway_port, llm_gateway_port
        )

        start_time = time.time()

        while True:
            container = client.containers.get(container.id)
            current_time = time.time()
            elapsed_time = current_time - start_time

            # Check if timeout is reached
            if elapsed_time > log_timeout:
                log.info(f"Stopping log monitoring after {log_timeout} seconds.")
                break

            container_status = container.attrs["State"]["Health"]["Status"]

            if container_status == "healthy":
                log.info("Container is healthy!")
                break
            else:
                log.info(f"Container health status: {container_status}")
                time.sleep(1)

        if foreground:
            for line in container.logs(stream=True):
                print(line.decode("utf-8").strip("\n"))

    except KeyboardInterrupt:
        log.info("Keyboard interrupt received, stopping arch gateway service.")
        stop_arch()
    except docker.errors.APIError as e:
        log.info(f"Failed to start Arch: {str(e)}")


def stop_arch():
    """
    Shutdown all Docker Compose services by running `docker-compose down`.

    Args:
        path (str): The path where the docker-compose.yml file is located.
    """
    log.info("Shutting down arch gateway service.")

    try:
        subprocess.run(
            ["docker", "stop", "archgw"],
        )
        subprocess.run(
            ["docker", "remove", "archgw"],
        )

        log.info("Successfully shut down arch gateway service.")

    except subprocess.CalledProcessError as e:
        log.info(f"Failed to shut down services: {str(e)}")


def download_models_from_hf():
    for model in KATANEMO_LOCAL_MODEL_LIST:
        log.info(f"Downloading model: {model}")
        snapshot_download(repo_id=model)


def start_arch_modelserver(foreground):
    """
    Start the model server. This assumes that the archgw_modelserver package is installed locally

    """
    try:
        log.info("archgw_modelserver restart")
        if foreground:
            subprocess.run(
                ["archgw_modelserver", "start", "--foreground"],
                check=True,
            )
        else:
            subprocess.run(
                ["archgw_modelserver", "start"],
                check=True,
            )
    except subprocess.CalledProcessError as e:
        log.info(f"Failed to start model_server. Please check archgw_modelserver logs")
        sys.exit(1)


def stop_arch_modelserver():
    """
    Stop the model server. This assumes that the archgw_modelserver package is installed locally

    """
    try:
        subprocess.run(
            ["archgw_modelserver", "stop"],
            check=True,
        )
    except subprocess.CalledProcessError as e:
        log.info(f"Failed to start model_server. Please check archgw_modelserver logs")
        sys.exit(1)
