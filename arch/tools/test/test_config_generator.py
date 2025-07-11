import pytest
from unittest import mock
import sys
from cli.config_generator import validate_and_render_schema

# Patch sys.path to allow import from cli/
import os

sys.path.insert(
    0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "cli"))
)


@pytest.fixture(autouse=True)
def cleanup_env(monkeypatch):
    # Clean up environment variables and mocks after each test
    yield
    monkeypatch.undo()


def test_validate_and_render_happy_path(monkeypatch):
    monkeypatch.setenv("ARCH_CONFIG_FILE", "fake_arch_config.yaml")
    monkeypatch.setenv("ARCH_CONFIG_SCHEMA_FILE", "fake_arch_config_schema.yaml")
    monkeypatch.setenv("ENVOY_CONFIG_TEMPLATE_FILE", "./envoy.template.yaml")
    monkeypatch.setenv("ARCH_CONFIG_FILE_RENDERED", "fake_arch_config_rendered.yaml")
    monkeypatch.setenv("ENVOY_CONFIG_FILE_RENDERED", "fake_envoy.yaml")
    monkeypatch.setenv("TEMPLATE_ROOT", "../")

    arch_config = """
version: v0.1.0

listeners:
  egress_traffic:
    address: 0.0.0.0
    port: 12000
    message_format: openai
    timeout: 30s

llm_providers:

  - model: openai/gpt-4o-mini
    access_key: $OPENAI_API_KEY
    default: true

  - model: openai/gpt-4o
    access_key: $OPENAI_API_KEY
    routing_preferences:
      - name: code understanding
        description: understand and explain existing code snippets, functions, or libraries

  - model: openai/gpt-4.1
    access_key: $OPENAI_API_KEY
    routing_preferences:
      - name: code generation
        description: generating new code snippets, functions, or boilerplate based on user prompts or requirements

tracing:
  random_sampling: 100
"""
    arch_config_schema = ""
    with open("../arch_config_schema.yaml", "r") as file:
        arch_config_schema = file.read()

    m_open = mock.mock_open()
    # Provide enough file handles for all open() calls in validate_and_render_schema
    m_open.side_effect = [
        mock.mock_open(read_data="").return_value,
        mock.mock_open(read_data=arch_config).return_value,  # ARCH_CONFIG_FILE
        mock.mock_open(
            read_data=arch_config_schema
        ).return_value,  # ARCH_CONFIG_SCHEMA_FILE
        mock.mock_open(read_data=arch_config).return_value,  # ARCH_CONFIG_FILE
        mock.mock_open(
            read_data=arch_config_schema
        ).return_value,  # ARCH_CONFIG_SCHEMA_FILE
        mock.mock_open().return_value,  # ENVOY_CONFIG_FILE_RENDERED (write)
        mock.mock_open().return_value,  # ARCH_CONFIG_FILE_RENDERED (write)
    ]
    with mock.patch("builtins.open", m_open):
        with mock.patch("config_generator.Environment"):
            validate_and_render_schema()


arch_config_test_cases = [
    {
        "id": "duplicate_provider_name",
        "expected_error": "Duplicate llm_provider name",
        "arch_config": """
version: v0.1.0

listeners:
  egress_traffic:
    address: 0.0.0.0
    port: 12000
    message_format: openai
    timeout: 30s

llm_providers:

  - name: test1
    model: openai/gpt-4o
    access_key: $OPENAI_API_KEY

  - name: test1
    model: openai/gpt-4o
    access_key: $OPENAI_API_KEY

""",
    },
    {
        "id": "provider_interface_with_model_id",
        "expected_error": "Please provide provider interface as part of model name",
        "arch_config": """
version: v0.1.0

listeners:
  egress_traffic:
    address: 0.0.0.0
    port: 12000
    message_format: openai
    timeout: 30s

llm_providers:

  - model: openai/gpt-4o
    access_key: $OPENAI_API_KEY
    provider_interface: openai

""",
    },
    {
        "id": "duplicate_model_id",
        "expected_error": "Duplicate model_id",
        "arch_config": """
version: v0.1.0

listeners:
  egress_traffic:
    address: 0.0.0.0
    port: 12000
    message_format: openai
    timeout: 30s

llm_providers:

  - model: openai/gpt-4o
    access_key: $OPENAI_API_KEY

  - model: mistral/gpt-4o

""",
    },
    {
        "id": "custom_provider_base_url",
        "expected_error": "Must provide base_url and provider_interface",
        "arch_config": """
version: v0.1.0

listeners:
  egress_traffic:
    address: 0.0.0.0
    port: 12000
    message_format: openai
    timeout: 30s

llm_providers:

  - model: custom/gpt-4o

""",
    },
    {
        "id": "base_url_no_prefix",
        "expected_error": "Please provide base_url without path",
        "arch_config": """
version: v0.1.0

listeners:
  egress_traffic:
    address: 0.0.0.0
    port: 12000
    message_format: openai
    timeout: 30s

llm_providers:

  - model: custom/gpt-4o
    base_url: "http://custom.com/test"
    provider_interface: openai

""",
    },
    {
        "id": "duplicate_routeing_preference_name",
        "expected_error": "Duplicate routing preference name",
        "arch_config": """
version: v0.1.0

listeners:
  egress_traffic:
    address: 0.0.0.0
    port: 12000
    message_format: openai
    timeout: 30s

llm_providers:

  - model: openai/gpt-4o-mini
    access_key: $OPENAI_API_KEY
    default: true

  - model: openai/gpt-4o
    access_key: $OPENAI_API_KEY
    routing_preferences:
      - name: code understanding
        description: understand and explain existing code snippets, functions, or libraries

  - model: openai/gpt-4.1
    access_key: $OPENAI_API_KEY
    routing_preferences:
      - name: code understanding
        description: generating new code snippets, functions, or boilerplate based on user prompts or requirements

tracing:
  random_sampling: 100

""",
    },
]


@pytest.mark.parametrize(
    "arch_config_test_case",
    arch_config_test_cases,
    ids=[case["id"] for case in arch_config_test_cases],
)
def test_validate_and_render_schema_tests(monkeypatch, arch_config_test_case):
    monkeypatch.setenv("ARCH_CONFIG_FILE", "fake_arch_config.yaml")
    monkeypatch.setenv("ARCH_CONFIG_SCHEMA_FILE", "fake_arch_config_schema.yaml")
    monkeypatch.setenv("ENVOY_CONFIG_TEMPLATE_FILE", "./envoy.template.yaml")
    monkeypatch.setenv("ARCH_CONFIG_FILE_RENDERED", "fake_arch_config_rendered.yaml")
    monkeypatch.setenv("ENVOY_CONFIG_FILE_RENDERED", "fake_envoy.yaml")
    monkeypatch.setenv("TEMPLATE_ROOT", "../")

    arch_config = arch_config_test_case["arch_config"]
    expected_error = arch_config_test_case["expected_error"]
    test_id = arch_config_test_case["id"]

    arch_config_schema = ""
    with open("../arch_config_schema.yaml", "r") as file:
        arch_config_schema = file.read()

    m_open = mock.mock_open()
    # Provide enough file handles for all open() calls in validate_and_render_schema
    m_open.side_effect = [
        mock.mock_open(read_data="").return_value,
        mock.mock_open(read_data=arch_config).return_value,  # ARCH_CONFIG_FILE
        mock.mock_open(
            read_data=arch_config_schema
        ).return_value,  # ARCH_CONFIG_SCHEMA_FILE
        mock.mock_open(read_data=arch_config).return_value,  # ARCH_CONFIG_FILE
        mock.mock_open(
            read_data=arch_config_schema
        ).return_value,  # ARCH_CONFIG_SCHEMA_FILE
        mock.mock_open().return_value,  # ENVOY_CONFIG_FILE_RENDERED (write)
        mock.mock_open().return_value,  # ARCH_CONFIG_FILE_RENDERED (write)
    ]
    with mock.patch("builtins.open", m_open):
        with mock.patch("config_generator.Environment"):
            with pytest.raises(Exception) as excinfo:
                validate_and_render_schema()
            assert expected_error in str(excinfo.value)
