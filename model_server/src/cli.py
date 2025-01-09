import importlib
import os
import sys
import subprocess
import argparse
import signal
import tempfile
import time
import requests

import src.commons.utils as utils


logger = utils.get_model_server_logger()


def get_version():
    try:
        version = importlib.metadata.version("archgw_modelserver")
        return version
    except importlib.metadata.PackageNotFoundError:
        return "version not found"


def wait_for_health_check(url, timeout=300):
    """Wait for the Uvicorn server to respond to health-check requests."""

    start_time = time.time()
    while time.time() - start_time < timeout:
        try:
            response = requests.get(url)
            if response.status_code == 200:
                return True
        except requests.ConnectionError:
            time.sleep(1)

    return False


def get_pid_file():
    temp_dir = tempfile.gettempdir()
    return os.path.join(temp_dir, "model_server.pid")


def ensure_killed(process):
    process.terminate()
    # if the process is not terminated, kill it
    now = time.time()
    # wait for 5 seconds
    while time.time() - now < 5:
        if process.poll() is not None:
            break
        time.sleep(1)
    if process.poll() is None:
        logger.info("Killing model server")
        process.kill()


def start_server(port=51000, foreground=False):
    """Start the Uvicorn server."""

    logger.info("model server version: %s", get_version())

    stop_server()

    logger.info(
        "starting model server, port: %s, foreground: %s. Please wait ...",
        port,
        foreground,
    )

    if foreground:
        process = subprocess.Popen(
            [
                "python",
                "-m",
                "uvicorn",
                "src.main:app",
                "--host",
                "0.0.0.0",
                "--port",
                str(port),
            ],
        )
    else:
        process = subprocess.Popen(
            [
                "python",
                "-m",
                "uvicorn",
                "src.main:app",
                "--host",
                "0.0.0.0",
                "--port",
                str(port),
            ],
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
        )

    try:
        if wait_for_health_check(f"http://0.0.0.0:{port}/healthz"):
            logger.info(
                f"model server health check passed, port {port}, pid: {process.pid}"
            )
        else:
            logger.error("health check failed, shutting it down.")
            process.terminate()
    except KeyboardInterrupt:
        logger.info("model server stopped by user during initialization.")
        ensure_killed(process)

    # write process id to temp file in temp folder
    pid_file = get_pid_file()
    logger.info(f"writing pid {process.pid} to {pid_file}")
    with open(pid_file, "w") as f:
        f.write(str(process.pid))

    if foreground:
        try:
            process.wait()
        except KeyboardInterrupt:
            logger.info("model server stopped by user.")
            ensure_killed(process)


def stop_server():
    """Stop the Uvicorn server."""

    pid_file = get_pid_file()
    if os.path.exists(pid_file):
        logger.info("PID file found, shutting down the server.")
        # read pid from file
        with open(pid_file, "r") as f:
            pid = int(f.read())
            logger.info(f"Killing model server {pid}")
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                logger.info(f"Process {pid} not found")
        os.remove(pid_file)
    else:
        logger.info("No PID file found, server is not running.")


def restart_server(port=51000, foreground=False):
    """Restart the Uvicorn server."""
    stop_server()
    start_server(port, foreground)


def parse_args():
    parser = argparse.ArgumentParser(description="Manage the Uvicorn server.")
    parser.add_argument(
        "action",
        choices=["start", "stop", "restart"],
        default="start",
        nargs="?",
        help="Action to perform on the server (default: start).",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=51000,
        help="Port number for the server (default: 51000).",
    )

    parser.add_argument(
        "--foreground",
        default=False,
        action="store_true",
        help="Run the server in the foreground (default: False).",
    )

    return parser.parse_args()


def main():
    """
    Start, stop, or restart the Uvicorn server based on command-line arguments.
    """

    args = parse_args()

    if args.action == "start":
        logger.info("[CLI] - Starting server")
        start_server(args.port, args.foreground)
    elif args.action == "stop":
        logger.info("[CLI] - Stopping server")
        stop_server()
    elif args.action == "restart":
        logger.info("[CLI] - Restarting server")
        restart_server(args.port)
    else:
        logger.error(f"[CLI] - Unknown action: {args.action}")
        sys.exit(1)
