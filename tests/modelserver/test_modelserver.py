import os
import pytest
import requests
import yaml

from deepdiff import DeepDiff

pytestmark = pytest.mark.skip(
    reason="Skipping entire test file as this these tests are heavily dependent on model output"
)

MODEL_SERVER_ENDPOINT = os.getenv(
    "MODEL_SERVER_ENDPOINT", "http://localhost:51000/function_calling"
)

# Load test data from YAML file
script_dir = os.path.dirname(__file__)

# Construct the full path to the YAML file
yaml_file_path = os.path.join(script_dir, "test_success_data.yaml")

# Load test data from YAML file
with open(yaml_file_path, "r") as file:
    test_data_yaml = yaml.safe_load(file)


@pytest.mark.parametrize(
    "test_data",
    [
        pytest.param(test_case, id=test_case["id"])
        for test_case in test_data_yaml["test_cases"]
    ],
)
def test_model_server(test_data):
    input = test_data["input"]
    expected = test_data["expected"]

    response = requests.post(MODEL_SERVER_ENDPOINT, json=input)
    assert response.status_code == 200
    # ensure that response is json
    assert response.headers["content-type"] == "application/json"
    response_json = response.json()
    assert response_json
    choices = response_json.get("choices", [])
    assert len(choices) == 1
    choice = choices[0]
    assert "message" in choice
    message = choice["message"]
    assert "tool_calls" in message
    tool_calls = message["tool_calls"]
    assert len(tool_calls) == len(expected)

    for tool_call, expected_tool_call in zip(tool_calls, expected):
        assert "id" in tool_call
        del tool_call["id"]
        # ensure that the tool call matches the expected tool call
        diff = DeepDiff(expected_tool_call, tool_call, ignore_string_case=True)
        assert not diff
