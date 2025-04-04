import pytest
import time
from src.commons.globals import handler_map
from src.core.utils.model_utils import ChatMessage, Message


# define function
get_weather_api = {
    "type": "function",
    "function": {
        "name": "get_current_weather",
        "description": "Get current weather at a location.",
        "parameters": {
            "type": "object",
            "properties": {
                "location": {
                    "type": "str",
                    "description": "The location to get the weather for",
                    "format": "City, State",
                },
                "unit": {
                    "type": "str",
                    "description": "The unit to return the weather in.",
                    "enum": ["celsius", "fahrenheit"],
                    "default": "celsius",
                },
                "days": {
                    "type": "str",
                    "description": "the number of days for the request.",
                },
            },
            "required": ["location", "days"],
        },
    },
}

# get_data class return request, intent, hallucination, parameter_gathering


def get_hallucination_data():
    # Create instances of the Message class
    message1 = Message(role="user", content="How is the weather in Seattle in days?")

    # Create a list of tools
    tools = [get_weather_api]

    # Create an instance of the ChatMessage class
    req = ChatMessage(messages=[message1], tools=tools)

    # first token will not be tool call
    return req, False, True


def get_success_tool_call_data():
    # Create instances of the Message class
    message1 = Message(role="user", content="How is the weather in Seattle in 7 days?")

    # Create a list of tools
    tools = [get_weather_api]

    # Create an instance of the ChatMessage class
    req = ChatMessage(messages=[message1], tools=tools)

    return req, True, False


def get_irrelevant_data():
    # Create instances of the Message class
    message1 = Message(role="user", content="What is 1+1?")

    # Create a list of tools
    tools = [get_weather_api]

    # Create an instance of the ChatMessage class
    req = ChatMessage(messages=[message1], tools=tools)

    return req, False, False


def get_greeting_data():
    # Create instances of the Message class
    message1 = Message(role="user", content="Hello how are you?")

    # Create a list of tools
    tools = [get_weather_api]

    # Create an instance of the ChatMessage class
    req = ChatMessage(messages=[message1], tools=tools)

    return req, False, False


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "get_data_func",
    [
        get_hallucination_data,
        get_greeting_data,
        get_irrelevant_data,
        get_success_tool_call_data,
    ],
)
async def test_function_calling(get_data_func):
    req, intent, hallucination = get_data_func()
    handler_name = "Arch-Function"
    use_agent_orchestrator = False
    model_handler: ArchFunctionHandler = handler_map[handler_name]

    start_time = time.perf_counter()
    final_response = await model_handler.chat_completion(req)
    latency = time.perf_counter() - start_time

    assert intent == (len(final_response.choices[0].message.tool_calls) >= 1)

    assert hallucination == model_handler.hallucination_state.hallucination
