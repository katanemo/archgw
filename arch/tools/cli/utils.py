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


def convert_legacy_llm_providers(
    listeners: dict | list, llm_providers: list | None
) -> tuple[list, dict | None, dict | None]:
    llm_gateway_listener = {
        "name": "egress_traffic",
        "port": 12000,
        "address": "0.0.0.0",
        "timeout": "30s",
        "llm_providers": [],
        "protocol": "openai",
    }

    prompt_gateway_listener = {
        "name": "ingress_traffic",
        "port": 10000,
        "address": "0.0.0.0",
        "timeout": "30s",
        "protocol": "openai",
    }

    if isinstance(listeners, dict):
        # legacy listeners
        # check if type is array or object
        # if its dict its legacy format let's convert it to array
        updated_listeners = []
        ingress_traffic = listeners.get("ingress_traffic", {})
        egress_traffic = listeners.get("egress_traffic", {})

        llm_gateway_listener["port"] = egress_traffic.get(
            "port", llm_gateway_listener["port"]
        )
        llm_gateway_listener["address"] = egress_traffic.get(
            "address", llm_gateway_listener["address"]
        )
        llm_gateway_listener["timeout"] = egress_traffic.get(
            "timeout", llm_gateway_listener["timeout"]
        )
        if llm_providers is None or llm_providers == []:
            raise ValueError("llm_providers cannot be empty when using legacy format")

        llm_gateway_listener["llm_providers"] = llm_providers
        updated_listeners.append(llm_gateway_listener)

        if ingress_traffic and ingress_traffic != {}:
            prompt_gateway_listener["port"] = ingress_traffic.get(
                "port", prompt_gateway_listener["port"]
            )
            prompt_gateway_listener["address"] = ingress_traffic.get(
                "address", prompt_gateway_listener["address"]
            )
            prompt_gateway_listener["timeout"] = ingress_traffic.get(
                "timeout", prompt_gateway_listener["timeout"]
            )
            updated_listeners.append(prompt_gateway_listener)

        return updated_listeners, llm_gateway_listener, prompt_gateway_listener

    llm_provider_set = False
    for listener in listeners:
        if listener.get("llm_providers") is not None:
            if llm_provider_set:
                raise ValueError(
                    "Currently only one listener can have llm_providers set"
                )
            llm_gateway_listener = listener
            llm_provider_set = True

    return listeners, llm_gateway_listener, prompt_gateway_listener


def get_llm_provider_access_keys(arch_config_file):
    with open(arch_config_file, "r") as file:
        arch_config = file.read()
        arch_config_yaml = yaml.safe_load(arch_config)

    access_key_list = []
    listeners, _, _ = convert_legacy_llm_providers(
        arch_config_yaml.get("listeners"), arch_config_yaml.get("llm_providers")
    )

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

    for listener in listeners:
        for llm_provider in listener.get("llm_providers", []):
            access_key = llm_provider.get("access_key")
            if access_key is not None:
                access_key_list.append(access_key)

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
