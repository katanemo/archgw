import json
import os
from jinja2 import Environment, FileSystemLoader
import yaml
from jsonschema import validate
from urllib.parse import urlparse

ENVOY_CONFIG_TEMPLATE_FILE = os.getenv(
    "ENVOY_CONFIG_TEMPLATE_FILE", "envoy.template.yaml"
)
ARCH_CONFIG_FILE = os.getenv("ARCH_CONFIG_FILE", "/app/arch_config.yaml")
ENVOY_CONFIG_FILE_RENDERED = os.getenv(
    "ENVOY_CONFIG_FILE_RENDERED", "/etc/envoy/envoy.yaml"
)
ARCH_CONFIG_SCHEMA_FILE = os.getenv(
    "ARCH_CONFIG_SCHEMA_FILE", "arch_config_schema.yaml"
)


def get_endpoint_and_port(endpoint, protocol):
    endpoint_tokens = endpoint.split(":")
    if len(endpoint_tokens) > 1:
        endpoint = endpoint_tokens[0]
        port = int(endpoint_tokens[1])
        return endpoint, port
    else:
        if protocol == "http":
            port = 80
        else:
            port = 443
        return endpoint, port


def validate_and_render_schema():
    env = Environment(loader=FileSystemLoader("./"))
    template = env.get_template("envoy.template.yaml")

    try:
        validate_prompt_config(ARCH_CONFIG_FILE, ARCH_CONFIG_SCHEMA_FILE)
    except Exception as e:
        print(str(e))
        exit(1)  # validate_prompt_config failed. Exit

    with open(ARCH_CONFIG_FILE, "r") as file:
        arch_config = file.read()

    with open(ARCH_CONFIG_SCHEMA_FILE, "r") as file:
        arch_config_schema = file.read()

    config_yaml = yaml.safe_load(arch_config)
    config_schema_yaml = yaml.safe_load(arch_config_schema)
    inferred_clusters = {}

    endpoints = config_yaml.get("endpoints", {})

    # override the inferred clusters with the ones defined in the config
    for name, endpoint_details in endpoints.items():
        inferred_clusters[name] = endpoint_details
        endpoint = inferred_clusters[name]["endpoint"]
        protocol = inferred_clusters[name].get("protocol", "http")
        (
            inferred_clusters[name]["endpoint"],
            inferred_clusters[name]["port"],
        ) = get_endpoint_and_port(endpoint, protocol)

    print("defined clusters from arch_config.yaml: ", json.dumps(inferred_clusters))

    if "prompt_targets" in config_yaml:
        for prompt_target in config_yaml["prompt_targets"]:
            name = prompt_target.get("endpoint", {}).get("name", None)
            if not name:
                continue
            if name not in inferred_clusters:
                raise Exception(
                    f"Unknown endpoint {name}, please add it in endpoints section in your arch_config.yaml file"
                )

    arch_tracing = config_yaml.get("tracing", {})

    llms_with_endpoint = []

    updated_llm_providers = []
    for llm_provider in config_yaml["llm_providers"]:
        provider = None
        if llm_provider.get("provider") and llm_provider.get("provider_interface"):
            raise Exception(
                "Please provide either provider or provider_interface, not both"
            )
        if llm_provider.get("provider"):
            provider = llm_provider["provider"]
            llm_provider["provider_interface"] = provider
            del llm_provider["provider"]
        updated_llm_providers.append(llm_provider)

        if llm_provider.get("endpoint") and llm_provider.get("base_url"):
            raise Exception("Please provide either endpoint or base_url, not both")

        if llm_provider.get("endpoint", None):
            endpoint = llm_provider["endpoint"]
            protocol = llm_provider.get("protocol", "http")
            llm_provider["endpoint"], llm_provider["port"] = get_endpoint_and_port(
                endpoint, protocol
            )
            llms_with_endpoint.append(llm_provider)
        elif llm_provider.get("base_url", None):
            base_url = llm_provider["base_url"]
            urlparse_result = urlparse(base_url)
            if llm_provider.get("port"):
                raise Exception("Please provider port in base_url")
            if urlparse_result.scheme == "" or urlparse_result.scheme not in [
                "http",
                "https",
            ]:
                raise Exception(
                    "Please provide a valid URL with scheme (http/https) in base_url"
                )
            protocol = urlparse_result.scheme
            port = urlparse_result.port
            if port is None:
                if protocol == "http":
                    port = 80
                else:
                    port = 443
            endpoint = urlparse_result.hostname
            llm_provider["endpoint"] = endpoint
            llm_provider["port"] = port
            llm_provider["protocol"] = protocol
            llms_with_endpoint.append(llm_provider)

    config_yaml["llm_providers"] = updated_llm_providers

    arch_config_string = yaml.dump(config_yaml)
    arch_llm_config_string = yaml.dump(config_yaml)

    prompt_gateway_listener = config_yaml.get("listeners", {}).get(
        "ingress_traffic", {}
    )
    if prompt_gateway_listener.get("port") == None:
        prompt_gateway_listener["port"] = 10000  # default port for prompt gateway
    if prompt_gateway_listener.get("address") == None:
        prompt_gateway_listener["address"] = "127.0.0.1"
    if prompt_gateway_listener.get("timeout") == None:
        prompt_gateway_listener["timeout"] = "10s"

    llm_gateway_listener = config_yaml.get("listeners", {}).get("egress_traffic", {})
    if llm_gateway_listener.get("port") == None:
        llm_gateway_listener["port"] = 12000  # default port for llm gateway
    if llm_gateway_listener.get("address") == None:
        llm_gateway_listener["address"] = "127.0.0.1"
    if llm_gateway_listener.get("timeout") == None:
        llm_gateway_listener["timeout"] = "10s"

    agent_orchestrator = None
    for name, endpoint_details in endpoints.items():
        if endpoint_details.get("agent_orchestrator", False):
            agent_orchestrator = name
            break

    data = {
        "prompt_gateway_listener": prompt_gateway_listener,
        "llm_gateway_listener": llm_gateway_listener,
        "arch_config": arch_config_string,
        "arch_llm_config": arch_llm_config_string,
        "arch_clusters": inferred_clusters,
        "arch_llm_providers": config_yaml["llm_providers"],
        "arch_tracing": arch_tracing,
        "local_llms": llms_with_endpoint,
        "agent_orchestrator": agent_orchestrator,
    }

    rendered = template.render(data)
    print(ENVOY_CONFIG_FILE_RENDERED)
    print(rendered)
    with open(ENVOY_CONFIG_FILE_RENDERED, "w") as file:
        file.write(rendered)


def validate_prompt_config(arch_config_file, arch_config_schema_file):
    with open(arch_config_file, "r") as file:
        arch_config = file.read()

    with open(arch_config_schema_file, "r") as file:
        arch_config_schema = file.read()

    config_yaml = yaml.safe_load(arch_config)
    config_schema_yaml = yaml.safe_load(arch_config_schema)

    try:
        validate(config_yaml, config_schema_yaml)
    except Exception as e:
        print(
            f"Error validating arch_config file: {arch_config_file}, schema file: {arch_config_schema_file}, error: {e.message}"
        )
        raise e


if __name__ == "__main__":
    validate_and_render_schema()
