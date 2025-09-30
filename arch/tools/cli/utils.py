import glob
import os
import subprocess
import sys
import yaml
import logging


logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)


def getLogger(name="cli"):
    logger = logging.getLogger(name)
    logger.setLevel(logging.INFO)
    return logger


log = getLogger(__name__)


def has_ingress_listener(arch_config_file):
    """Check if the arch config file has ingress_traffic listener configured."""
    try:
        with open(arch_config_file) as f:
            arch_config_dict = yaml.safe_load(f)

        ingress_traffic = arch_config_dict.get("listeners", {}).get(
            "ingress_traffic", {}
        )

        return bool(ingress_traffic)
    except Exception as e:
        log.error(f"Error reading config file {arch_config_file}: {e}")
        return False


def get_llm_provider_access_keys(arch_config_file):
    with open(arch_config_file, "r") as file:
        arch_config = file.read()
        arch_config_yaml = yaml.safe_load(arch_config)

    access_key_list = []
    for llm_provider in arch_config_yaml.get("llm_providers", []):
        acess_key = llm_provider.get("access_key")
        if acess_key is not None:
            access_key_list.append(acess_key)

    for prompt_target in arch_config_yaml.get("prompt_targets", []):
        for k, v in prompt_target.get("endpoint", {}).get("http_headers", {}).items():
            if k.lower() == "authorization":
                print(
                    f"found auth header: {k} for prompt_target: {prompt_target.get('name')}/{prompt_target.get('endpoint').get('name')}"
                )
                auth_tokens = v.split(" ")
                if len(auth_tokens) > 1:
                    access_key_list.append(auth_tokens[1])
                else:
                    access_key_list.append(v)

    return access_key_list


def load_env_file_to_dict(file_path):
    env_dict = {}

    # Open and read the .env file
    with open(file_path, "r") as file:
        for line in file:
            # Strip any leading/trailing whitespaces
            line = line.strip()

            # Skip empty lines and comments
            if not line or line.startswith("#"):
                continue

            # Split the line into key and value at the first '=' sign
            if "=" in line:
                key, value = line.split("=", 1)
                key = key.strip()
                value = value.strip()

                # Add key-value pair to the dictionary
                env_dict[key] = value

    return env_dict


def find_config_file(path=".", file=None):
    """Find the appropriate config file path."""
    if file:
        # If a file is provided, process that file
        return os.path.abspath(file)
    else:
        # If no file is provided, use the path and look for arch_config.yaml first, then config.yaml for convenience
        arch_config_file = os.path.abspath(os.path.join(path, "config.yaml"))
        if not os.path.exists(arch_config_file):
            arch_config_file = os.path.abspath(os.path.join(path, "arch_config.yaml"))
        return arch_config_file


def stream_access_logs(follow):
    """
    Get the archgw access logs
    """

    follow_arg = "-f" if follow else ""

    stream_command = [
        "docker",
        "exec",
        "archgw",
        "sh",
        "-c",
        f"tail {follow_arg} /var/log/access_*.log",
    ]

    subprocess.run(
        stream_command,
        check=True,
        stdout=sys.stdout,
        stderr=sys.stderr,
    )
