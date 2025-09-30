import json
import os
from jinja2 import Environment, FileSystemLoader
import yaml
from jsonschema import validate
from urllib.parse import urlparse


SUPPORTED_PROVIDERS = [
    "arch",
    "deepseek",
    "groq",
    "mistral",
    "openai",
    "gemini",
    "anthropic",
    "together_ai",
    "azure_openai",
    "xai",
    "ollama",
    "moonshotai",
    "zhipu",
]


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
    ENVOY_CONFIG_TEMPLATE_FILE = os.getenv(
        "ENVOY_CONFIG_TEMPLATE_FILE", "envoy.template.yaml"
    )
    ARCH_CONFIG_FILE = os.getenv("ARCH_CONFIG_FILE", "/app/arch_config.yaml")
    ARCH_CONFIG_FILE_RENDERED = os.getenv(
        "ARCH_CONFIG_FILE_RENDERED", "/app/arch_config_rendered.yaml"
    )
    ENVOY_CONFIG_FILE_RENDERED = os.getenv(
        "ENVOY_CONFIG_FILE_RENDERED", "/etc/envoy/envoy.yaml"
    )
    ARCH_CONFIG_SCHEMA_FILE = os.getenv(
        "ARCH_CONFIG_SCHEMA_FILE", "arch_config_schema.yaml"
    )

    env = Environment(loader=FileSystemLoader(os.getenv("TEMPLATE_ROOT", "./")))
    template = env.get_template(ENVOY_CONFIG_TEMPLATE_FILE)

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
    _ = yaml.safe_load(arch_config_schema)
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

    llm_provider_name_set = set()
    model_name_keys = set()
    model_usage_name_keys = set()
    for llm_provider in config_yaml["llm_providers"]:
        if llm_provider.get("name") in llm_provider_name_set:
            raise Exception(
                f"Duplicate llm_provider name {llm_provider.get('name')}, please provide unique name for each llm_provider"
            )

        model_name = llm_provider.get("model")
        if model_name in model_name_keys:
            raise Exception(
                f"Duplicate model name {model_name}, please provide unique model name for each llm_provider"
            )

        model_name_keys.add(model_name)
        if llm_provider.get("name") is None:
            llm_provider["name"] = model_name

        llm_provider_name_set.add(llm_provider.get("name"))

        model_name_tokens = model_name.split("/")
        if len(model_name_tokens) < 2:
            raise Exception(
                f"Invalid model name {model_name}. Please provide model name in the format <provider>/<model_id>."
            )
        provider = model_name_tokens[0]
        # Validate azure_openai and ollama provider requires base_url
        if (provider == "azure_openai" or provider == "ollama") and llm_provider.get(
            "base_url"
        ) is None:
            raise Exception(
                f"Provider '{provider}' requires 'base_url' to be set for model {model_name}"
            )

        model_id = "/".join(model_name_tokens[1:])
        if provider not in SUPPORTED_PROVIDERS:
            if (
                llm_provider.get("base_url", None) is None
                or llm_provider.get("provider_interface", None) is None
            ):
                raise Exception(
                    f"Must provide base_url and provider_interface for unsupported provider {provider} for model {model_name}. Supported providers are: {', '.join(SUPPORTED_PROVIDERS)}"
                )
            provider = llm_provider.get("provider_interface", None)
        elif llm_provider.get("provider_interface", None) is not None:
            raise Exception(
                f"Please provide provider interface as part of model name {model_name} using the format <provider>/<model_id>. For example, use 'openai/gpt-3.5-turbo' instead of 'gpt-3.5-turbo' "
            )

        if model_id in model_name_keys:
            raise Exception(
                f"Duplicate model_id {model_id}, please provide unique model_id for each llm_provider"
            )
        model_name_keys.add(model_id)

        for routing_preference in llm_provider.get("routing_preferences", []):
            if routing_preference.get("name") in model_usage_name_keys:
                raise Exception(
                    f"Duplicate routing preference name \"{routing_preference.get('name')}\", please provide unique name for each routing preference"
                )
            model_usage_name_keys.add(routing_preference.get("name"))

        llm_provider["model"] = model_id
        llm_provider["provider_interface"] = provider
        updated_llm_providers.append(llm_provider)

        if llm_provider.get("base_url", None):
            base_url = llm_provider["base_url"]
            urlparse_result = urlparse(base_url)
            url_path = urlparse_result.path
            if url_path and url_path != "/":
                raise Exception(
                    f"Please provide base_url without path, got {base_url}. Use base_url like 'http://example.com' instead of 'http://example.com/path'."
                )
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
            llm_provider["cluster_name"] = (
                provider + "_" + endpoint
            )  # make name unique by appending endpoint
            llms_with_endpoint.append(llm_provider)

    if len(model_usage_name_keys) > 0:
        routing_llm_provider = config_yaml.get("routing", {}).get("llm_provider", None)
        if routing_llm_provider and routing_llm_provider not in llm_provider_name_set:
            raise Exception(
                f"Routing llm_provider {routing_llm_provider} is not defined in llm_providers"
            )
        if routing_llm_provider is None and "arch-router" not in llm_provider_name_set:
            updated_llm_providers.append(
                {
                    "name": "arch-router",
                    "provider_interface": "arch",
                    "model": config_yaml.get("routing", {}).get("model", "Arch-Router"),
                }
            )

    config_yaml["llm_providers"] = updated_llm_providers

    # Validate model aliases if present
    if "model_aliases" in config_yaml:
        model_aliases = config_yaml["model_aliases"]
        for alias_name, alias_config in model_aliases.items():
            target = alias_config.get("target")
            if target not in model_name_keys:
                raise Exception(
                    f"Model alias '{alias_name}' targets '{target}' which is not defined as a model. Available models: {', '.join(sorted(model_name_keys))}"
                )

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
        llm_gateway_listener["timeout"] = "300s"

    use_agent_orchestrator = config_yaml.get("overrides", {}).get(
        "use_agent_orchestrator", False
    )

    agent_orchestrator = None
    if use_agent_orchestrator:
        print("Using agent orchestrator")

        if len(endpoints) == 0:
            raise Exception(
                "Please provide agent orchestrator in the endpoints section in your arch_config.yaml file"
            )
        elif len(endpoints) > 1:
            raise Exception(
                "Please provide single agent orchestrator in the endpoints section in your arch_config.yaml file"
            )
        else:
            agent_orchestrator = list(endpoints.keys())[0]

    print("agent_orchestrator: ", agent_orchestrator)

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

    with open(ARCH_CONFIG_FILE_RENDERED, "w") as file:
        file.write(arch_config_string)


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
            f"Error validating arch_config file: {arch_config_file}, schema file: {arch_config_schema_file}, error: {e}"
        )
        raise e


if __name__ == "__main__":
    validate_and_render_schema()
