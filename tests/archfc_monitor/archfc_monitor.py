#!/usr/bin/env python3

import requests
import os
import sys
from datetime import datetime
from pathlib import Path

# Configuration
ARCHFC_ENDPOINT = "https://archfc.katanemo.dev/v1/chat/completions"
FAILURE_COUNT_FILE = "/tmp/archfc_monitor_failures.txt"
MAX_CONSECUTIVE_FAILURES = 2

# Get Slack webhook from environment variable
SLACK_WEBHOOK = os.environ.get("SLACK_WEBHOOK")
if not SLACK_WEBHOOK:
    print("ERROR: SLACK_WEBHOOK environment variable is not set!")
    print("Please set it using: export SLACK_WEBHOOK='your-webhook-url'")
    sys.exit(1)


def initialize_failure_count():
    """Initialize failure count file if it doesn't exist."""
    failure_file = Path(FAILURE_COUNT_FILE)
    if not failure_file.exists():
        failure_file.write_text("0")


def get_failure_count():
    """Read current failure count from file."""
    return int(Path(FAILURE_COUNT_FILE).read_text().strip())


def set_failure_count(count):
    """Write failure count to file."""
    Path(FAILURE_COUNT_FILE).write_text(str(count))


def send_slack_alert(message):
    """Send slack notification."""
    try:
        response = requests.post(
            SLACK_WEBHOOK,
            json={"text": message},
            headers={"Content-type": "application/json"},
        )
        response.raise_for_status()
        print(f"Slack alert sent successfully")
    except Exception as e:
        print(f"Failed to send Slack alert: {e}")


def run_test(test_name, url, headers, data, expected_response=None):
    """Run test and check HTTP status."""
    print(f"Running test: {test_name}")

    try:
        response = requests.post(url, headers=headers, json=data, timeout=30)
        http_code = response.status_code

        print(f"HTTP Status Code: {http_code}")

        if http_code == 200:
            # Validate expected response if provided
            if expected_response:
                try:
                    response_json = response.json()
                    actual_message = response_json.get("choices", [{}])[0].get("message", {})
                    
                    # Check if the actual response matches expected
                    matches = True
                    mismatches = []
                    
                    def normalize_value(value):
                        """Normalize values for case-insensitive comparison."""
                        if isinstance(value, str):
                            return value.lower()
                        elif isinstance(value, dict):
                            return {k: normalize_value(v) for k, v in value.items()}
                        elif isinstance(value, list):
                            return [normalize_value(v) for v in value]
                        return value
                    
                    for key, expected_value in expected_response.items():
                        actual_value = actual_message.get(key)
                        
                        # Normalize both values for comparison
                        normalized_expected = normalize_value(expected_value)
                        normalized_actual = normalize_value(actual_value)
                        
                        if normalized_actual != normalized_expected:
                            matches = False
                            mismatches.append(f"{key}: expected {expected_value}, got {actual_value}")
                    
                    if not matches:
                        error_details = {
                            "test_name": test_name,
                            "status_code": http_code,
                            "validation_error": "Response validation failed",
                            "mismatches": mismatches,
                            "expected": expected_response,
                            "actual": actual_message,
                            "url": url,
                        }
                        print(f"✗ {test_name} failed validation:")
                        for mismatch in mismatches:
                            print(f"  - {mismatch}")
                        return False, error_details
                    
                    print(f"✓ Response validation passed")
                except Exception as e:
                    error_details = {
                        "test_name": test_name,
                        "status_code": http_code,
                        "validation_error": f"Failed to parse/validate response: {str(e)}",
                        "response_text": response.text[:500],
                        "url": url,
                    }
                    print(f"✗ {test_name} failed to validate response: {e}")
                    return False, error_details
            
            print(f"✓ {test_name} passed")
            return True, None
        else:
            error_details = {
                "test_name": test_name,
                "status_code": http_code,
                "response_text": response.text[:500],  # Limit to first 500 chars
                "url": url,
            }
            print(f"✗ {test_name} failed with status code: {http_code}")
            return False, error_details
    except Exception as e:
        error_details = {
            "test_name": test_name,
            "error_type": type(e).__name__,
            "error_message": str(e),
            "url": url,
        }
        print(f"✗ {test_name} failed with exception: {e}")
        return False, error_details


def monitor():
    """Main monitoring function."""
    all_tests_passed = True
    failure_details = []

    print(f"=== Starting Arch-FC Monitoring at {datetime.now()} ===")

    # Test 1: Arch-Function routing model
    test1_passed, test1_error = run_test(
        "Arch-Function routing model",
        ARCHFC_ENDPOINT,
        {"Content-Type": "application/json"},
        {
            "model": "Arch-Function",
            "messages": [
                {
                    "role": "system",
                    "content": 'You are a helpful assistant.\n\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n{"type": "function", "function": {"name": "weather_forecast", "parameters": {"type": "object", "properties": {"city": {"type": "str"}, "days": {"type": "int"}}, "required": ["city", "days"]}}}\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{"name": <function-name>, "arguments": <args-json-object>}\n</tool_call>\n',
                },
                {"role": "user", "content": "how is the weather in seattle?"},
            ],
        },
        expected_response={
            "role": "assistant",
            "content": "{\"name\": \"weather_forecast\", \"arguments\": {\"city\": \"seattle\", \"days\": 1}}",
            "tool_calls": []
        }
    )

    if not test1_passed:
        all_tests_passed = False
        if test1_error:
            failure_details.append(test1_error)

    print()

    # Test 2: Arch-Router function calling model
    test2_passed, test2_error = run_test(
        "Arch-Router function calling model",
        ARCHFC_ENDPOINT,
        {"model": "Arch-Router", "Content-Type": "application/json"},
        {
            "model": "Arch-Router",
            "messages": [
                {
                    "role": "user",
                    "content": ' You are a helpful assistant designed to find the best suited route.\nYou are provided with route description within <routes></routes> XML tags:\n<routes>\n[{"name":"Image generation","description":"generating image"}]\n</routes>\n\n<conversation>\n[{"role":"user","content":"hi"},{"role":"assistant","content":"Hello! How can I assist you today?"},{"role":"user","content":"given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson"}]\n</conversation>\n\nYour task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:\n1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.\n2. You must analyze the route descriptions and find the best match route for user latest intent.\n3. You only response the name of the route that best matches the user\'s request, use the exact name in the <routes></routes>.\n\nBased on your analysis, provide your response in the following JSON formats if you decide to match any route:\n{"route": "route_name"}',
                }
            ],
        },
        expected_response={
            "role": "assistant",
            "content": "{'route': 'Image generation'}",
            "tool_calls": []
        }
    )

    if not test2_passed:
        all_tests_passed = False
        if test2_error:
            failure_details.append(test2_error)

    # Read current failure count
    current_failures = get_failure_count()

    if all_tests_passed:
        print("All tests passed! Resetting failure count.")
        set_failure_count(0)
    else:
        # Increment failure count
        current_failures += 1
        set_failure_count(current_failures)
        print(f"Test failure detected. Consecutive failures: {current_failures}")

        # Alert if threshold reached
        if current_failures >= MAX_CONSECUTIVE_FAILURES:
            # Build detailed alert message with failure logs
            alert_message = f"ALERT: Arch-FC monitoring has failed {current_failures} consecutive times at {datetime.now()}.\n\n"
            alert_message += "*Failure Details:*\n"
            
            for idx, error in enumerate(failure_details, 1):
                alert_message += f"\n*Test {idx}: {error.get('test_name', 'Unknown')}*\n"
                
                if 'status_code' in error:
                    alert_message += f"• Status Code: {error['status_code']}\n"
                    alert_message += f"• URL: {error['url']}\n"
                    alert_message += f"• Response: ```{error['response_text']}```\n"
                elif 'error_type' in error:
                    alert_message += f"• Error Type: {error['error_type']}\n"
                    alert_message += f"• Error Message: {error['error_message']}\n"
                    alert_message += f"• URL: {error['url']}\n"
            
            alert_message += "\nPlease investigate immediately!"
            
            print("Sending Slack alert with detailed logs...")
            send_slack_alert(alert_message)

    print(f"=== Monitoring complete at {datetime.now()} ===")
    print()


def main():
    """Main entry point."""
    initialize_failure_count()

    # Run monitor once
    monitor()

if __name__ == "__main__":
    main()
