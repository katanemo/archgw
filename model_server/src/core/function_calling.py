import ast
import copy
import json
import random
import builtins
import src.commons.utils as utils

from openai import OpenAI
from typing import Any, Dict, List
from overrides import override
from src.core.utils.hallucination_utils import HallucinationState
from src.core.utils.model_utils import (
    Message,
    ChatMessage,
    Choice,
    ChatCompletionResponse,
    ArchBaseHandler,
)


logger = utils.get_model_server_logger()


# ==============================================================================================================================================


class ArchFunctionConfig:
    TASK_PROMPT = (
        "You are a helpful assistant designed to assist with the user query by making one or more function calls if needed."
        "\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n{tools}\n</tools>"
        "\n\nYour task is to decide which functions are needed and collect missing parameters if necessary."
    )

    FORMAT_PROMPT = (
        "\n\nBased on your analysis, provide your response in one of the following JSON formats:"
        '\n1. If no functions are needed:\n```json\n{"response": "Your response text here"}\n```'
        '\n2. If functions are needed but some required parameters are missing:\n```json\n{"required_functions": ["func_name1", "func_name2", ...], "clarification": "Text asking for missing parameters"}\n```'
        '\n3. If functions are needed and all required parameters are available:\n```json\n{"tool_calls": [{"name": "func_name1", "arguments": {"argument1": "value1", "argument2": "value2"}},... (more tool calls as required)]}\n```'
    )

    GENERATION_PARAMS = {
        "temperature": 0.1,
        "top_p": 1.0,
        "top_k": 10,
        "max_tokens": 1024,
        "stop_token_ids": [151645],
        "logprobs": True,
        "top_logprobs": 10,
    }

    SUPPORT_DATA_TYPES = ["int", "float", "bool", "str", "list", "tuple", "set", "dict"]


class ArchFunctionHandler(ArchBaseHandler):
    def __init__(
        self,
        client: OpenAI,
        model_name: str,
        config: ArchFunctionConfig,
    ):
        """
        Initializes the function handler.

        Args:
            client (OpenAI): An OpenAI client instance.
            model_name (str): Name of the model to use.
            config (ArchFunctionConfig): The configuration for Arch-Function
        """

        super().__init__(
            client,
            model_name,
            config.TASK_PROMPT,
            config.FORMAT_PROMPT,
            config.GENERATION_PARAMS,
        )

        self.generation_params = self.generation_params | {
            "continue_final_message": True,
            "add_generation_prompt": False,
        }

        self.default_prefix = '```json\n{"'
        self.clarify_prefix = '```json\n{"required_functions":'

        self.hallucination_state = None

        # Predefine data types for verification. Only support Python for now.
        # TODO: Extend the list of support data types
        self.support_data_types = {
            type_name: getattr(builtins, type_name)
            for type_name in config.SUPPORT_DATA_TYPES
        }

    @override
    def _convert_tools(self, tools: List[Dict[str, Any]]) -> str:
        """
        Converts a list of tools into JSON format.

        Args:
            tools (List[Dict[str, Any]]): A list of tools represented as dictionaries.

        Returns:
            str: A string representation of converted tools.
        """

        converted = [json.dumps(tool["function"], ensure_ascii=False) for tool in tools]
        return "\n".join(converted)

    def _fix_json_string(self, json_str: str) -> str:
        """
        Fixes malformed JSON strings by ensuring proper bracket matching.

        Args:
            json_str (str): A JSON string that might be malformed.

        Returns:
            str: A corrected JSON string.
        """

        # Remove any leading or trailing whitespace or newline characters
        json_str = json_str.strip()

        # Stack to keep track of brackets
        stack = []

        # Clean string to collect valid characters
        fixed_str = ""

        # Dictionary for matching brackets
        matching_bracket = {")": "(", "}": "{", "]": "["}

        # Dictionary for the opposite of matching_bracket
        opening_bracket = {v: k for k, v in matching_bracket.items()}

        for char in json_str:
            if char in "{[(":
                stack.append(char)
                fixed_str += char
            elif char in "}])":
                if stack and stack[-1] == matching_bracket[char]:
                    stack.pop()
                    fixed_str += char
                else:
                    # Ignore the unmatched closing brackets
                    continue
            else:
                fixed_str += char

        # If there are unmatched opening brackets left in the stack, add corresponding closing brackets
        while stack:
            unmatched_opening = stack.pop()
            fixed_str += opening_bracket[unmatched_opening]

        try:
            fixed_str = json.loads(fixed_str)
        except Exception:
            fixed_str = json.loads(fixed_str.replace("'", '"'))

        return json.dumps(fixed_str)

    def _parse_model_response(self, content: str) -> Dict[str, any]:
        """
        Extracts tool call information from a given string.

        Args:
            content (str): The content string containing potential tool call information.

        Returns:
            Dict: A dictionary of extraction, including:
                - "required_functions": A list of detected intents.
                - "clarification": Text to collect missing parameters
                - "tool_calls": A list of tool call dictionaries.
                - "is_valid": A boolean indicating if the extraction was valid.
                - "error_message": An error message or exception if parsing failed.
        """

        response_dict = {
            "raw_response": [],
            "response": [],
            "required_functions": [],
            "clarification": "",
            "tool_calls": [],
            "is_valid": True,
            "error_message": "",
        }

        try:
            if content.startswith("```") and content.endswith("```"):
                content = content.strip("```").strip()
                if content.startswith("json"):
                    content = content[4:].strip()

            content = self._fix_json_string(content)
            response_dict["raw_response"] = f"```json\n{content}\n```"

            model_response = json.loads(content)
            response_dict["response"] = model_response.get("response", "")
            response_dict["required_functions"] = model_response.get(
                "required_functions", []
            )
            response_dict["clarification"] = model_response.get("clarification", "")

            for tool_call in model_response.get("tool_calls", []):
                response_dict["tool_calls"].append(
                    {
                        "id": f"call_{random.randint(1000, 10000)}",
                        "type": "function",
                        "function": {
                            "name": tool_call.get("name", ""),
                            "arguments": tool_call.get("arguments", {}),
                        },
                    }
                )
        except Exception as e:
            response_dict["is_valid"] = False
            response_dict["error_message"] = f"Fail to parse model responses: {e}"

        return response_dict

    def _convert_data_type(self, value: str, target_type: str):
        # TODO: Add more conversion rules as needed
        try:
            if target_type is float and isinstance(value, int):
                return float(value)
            elif target_type is list and isinstance(value, str):
                return ast.literal_eval(value)
            elif target_type is str and not isinstance(value, str):
                return str(value)
        except (ValueError, TypeError, json.JSONDecodeError):
            pass
        return value

    def _verify_tool_calls(
        self, tools: List[Dict[str, Any]], tool_calls: List[Dict[str, Any]]
    ) -> Dict[str, any]:
        """
        Verifies the validity of extracted tool calls against the provided tools.

        Args:
            tools (List[Dict[str, Any]]): A list of available tools.
            tool_calls (List[Dict[str, Any]]): A list of tool calls to verify.

        Returns:
            Dict: A dictionary of verification, including:
                - "status": A boolean indicating if the tool calls are valid.
                - "invalid_tool_call": A dictionary of the invalid tool call if any.
                - "message": An error message.
        """

        verification_dict = {
            "is_valid": True,
            "invalid_tool_call": {},
            "error_message": "",
        }

        functions = {}
        for tool in tools:
            functions[tool["function"]["name"]] = tool["function"]["parameters"]

        for tool_call in tool_calls:
            if not verification_dict["is_valid"]:
                break

            func_name = tool_call["function"]["name"]
            func_args = tool_call["function"]["arguments"]

            # Check whether the function is available or not
            if func_name not in functions:
                verification_dict["is_valid"] = False
                verification_dict["invalid_tool_call"] = tool_call
                verification_dict["error_message"] = f"{func_name} is not available!"
            else:
                # Check if all the requried parameters can be found in the tool calls
                for required_param in functions[func_name].get("required", []):
                    if required_param not in func_args:
                        verification_dict["is_valid"] = False
                        verification_dict["invalid_tool_call"] = tool_call
                        verification_dict[
                            "error_message"
                        ] = f"`{required_param}` is required by the function `{func_name}` but not found in the tool call!"
                        break

                # Verify the data type of each parameter in the tool calls
                function_properties = functions[func_name]["properties"]

                logger.info("== func_args ==")
                logger.info(func_args)
                for param_name in func_args:
                    if param_name not in function_properties:
                        verification_dict["is_valid"] = False
                        verification_dict["invalid_tool_call"] = tool_call
                        verification_dict[
                            "error_message"
                        ] = f"Parameter `{param_name}` is not defined in the function `{func_name}`."
                        break
                    else:
                        param_value = func_args[param_name]
                        target_type = function_properties[param_name]["type"]

                        if target_type in self.support_data_types:
                            data_type = self.support_data_types[target_type]

                            if not isinstance(param_value, data_type):
                                param_value = self._convert_data_type(
                                    param_value, data_type
                                )
                                if not isinstance(param_value, data_type):
                                    verification_dict["is_valid"] = False
                                    verification_dict["invalid_tool_call"] = tool_call
                                    verification_dict[
                                        "error_message"
                                    ] = f"Parameter `{param_name}` is expected to have the data type `{data_type}`, got `{type(param_value)}`."
                                    break
                        else:
                            verification_dict["is_valid"] = False
                            verification_dict["invalid_tool_call"] = tool_call
                            verification_dict[
                                "error_message"
                            ] = f"Data type `{target_type}` is not supported."

        return verification_dict

    def _prefill_message(self, messages: List[Dict[str, str]], prefill_message):
        """
        Update messages and generation params for prompt prefilling

        Args:
            messages (List[Dict[str, str]]): A list of messages.

        Returns:
            prefill_messages (List[Dict[str, str]]): A list of messages.
        """
        return messages + [{"role": "assistant", "content": prefill_message}]

    @override
    async def chat_completion(self, req: ChatMessage) -> ChatCompletionResponse:
        """
        Generates a chat completion response for a given request.

        Args:
            req (ChatMessage): A chat message request object.
            enable_prefilling (bool, optional): Whether to enable prefill responses. Defaults to True.
        Returns:
            ChatCompletionResponse: The model's response to the chat request.

        Note:
            Currently only support vllm inference
        """
        logger.info("[Arch-Function] - ChatCompletion")

        messages = self._process_messages(
            req.messages, req.tools, metadata=req.metadata
        )

        logger.info(
            f"[request to arch-fc]: model: {self.model_name}, extra_body: {self.generation_params}, body: {json.dumps(messages)}"
        )

        # always enable `stream=True` to collect model responses
        response = self.client.chat.completions.create(
            messages=self._prefill_message(messages, self.default_prefix),
            model=self.model_name,
            stream=True,
            extra_body=self.generation_params,
        )

        use_agent_orchestrator = req.metadata.get("use_agent_orchestrator", False)
        model_response = ""
        if use_agent_orchestrator:
            for chunk in response:
                if len(chunk.choices) > 0 and chunk.choices[0].delta.content:
                    model_response += chunk.choices[0].delta.content
            logger.info(f"[Agent Orchestrator]: response received: {model_response}")
        else:
            # initialize the hallucination handler, which is an iterator
            self.hallucination_state = HallucinationState(
                response_iterator=response, function=req.tools
            )

            has_tool_calls, has_hallucination = None, False
            for _ in self.hallucination_state:
                # check if moodel response starts with tool calls, we do it after 5 tokens because we only check the first part of the response.
                if len(self.hallucination_state.tokens) > 5 and has_tool_calls is None:
                    content = "".join(self.hallucination_state.tokens)
                    if "tool_calls" in content:
                        has_tool_calls = True
                    else:
                        has_tool_calls = False

                # if the model is hallucinating, start parameter gathering
                if self.hallucination_state.hallucination is True:
                    has_hallucination = True
                    break

            if has_tool_calls and has_hallucination:
                # start prompt prefilling if hallcuination is found in tool calls
                logger.info(
                    f"[Hallucination]: {self.hallucination_state.error_message}"
                )
                response = self.client.chat.completions.create(
                    messages=self._prefill_message(messages, self.clarify_prefix),
                    model=self.model_name,
                    stream=False,
                    extra_body=self.generation_params,
                )
                model_response = response.choices[0].message.content
            else:
                model_response = "".join(self.hallucination_state.tokens)

        # Extract tool calls from model response
        response_dict = self._parse_model_response(model_response)
        logger.info(f"[arch-fc]: raw model response: {response_dict['raw_response']}")

        # General model response
        if response_dict.get("response", ""):
            model_message = Message(content="", tool_calls=[])
        # Parameter gathering
        elif response_dict.get("required_functions", []):
            if not use_agent_orchestrator:
                clarification = response_dict.get("clarification", "")
                model_message = Message(content=clarification, tool_calls=[])
            else:
                model_message = Message(content="", tool_calls=[])
        # Function Calling
        elif response_dict.get("tool_calls", []):
            if response_dict["is_valid"]:
                if not use_agent_orchestrator:
                    verification_dict = self._verify_tool_calls(
                        tools=req.tools, tool_calls=response_dict["tool_calls"]
                    )

                    if verification_dict["is_valid"]:
                        logger.info(
                            f"[Tool calls]: {json.dumps([tool_call['function'] for tool_call in response_dict['tool_calls']])}"
                        )
                        model_message = Message(
                            content="", tool_calls=response_dict["tool_calls"]
                        )
                    else:
                        logger.error(
                            f"Invalid tool call - {verification_dict['error_message']}"
                        )
                        model_message = Message(content="", tool_calls=[])
                else:
                    # skip tool call verification if using agent orchestrator
                    logger.info(
                        f"[Tool calls]: {json.dumps([tool_call['function'] for tool_call in response_dict['tool_calls']])}"
                    )
                    model_message = Message(
                        content="", tool_calls=response_dict["tool_calls"]
                    )

            else:
                # Response with tool calls but invalid
                model_message = Message(content="", tool_calls=[])
        # Response not in the desired format
        else:
            logger.error(f"Invalid model response - {model_response}")
            model_message = Message(content="", tool_calls=[])

        chat_completion_response = ChatCompletionResponse(
            choices=[Choice(message=model_message)],
            model=self.model_name,
            metadata={"x-arch-fc-model-response": response_dict["raw_response"]},
            role="assistant",
        )

        logger.info(
            f"[response arch-fc]: {json.dumps(chat_completion_response.model_dump(exclude_none=True))}"
        )

        return chat_completion_response


# ==============================================================================================================================================


class ArchAgentConfig(ArchFunctionConfig):
    GENERATION_PARAMS = {
        "temperature": 0.01,
        "top_p": 1.0,
        "top_k": 10,
        "max_tokens": 1024,
        "stop_token_ids": [151645],
        "logprobs": True,
        "top_logprobs": 10,
    }


class ArchAgentHandler(ArchFunctionHandler):
    def __init__(self, client: OpenAI, model_name: str, config: ArchAgentConfig):
        super().__init__(client, model_name, config)

    @override
    def _convert_tools(self, tools: List[Dict[str, Any]]) -> str:
        """
        Converts a list of tools into JSON format.

        Args:
            tools (List[Dict[str, Any]]): A list of tools represented as dictionaries.

        Returns:
            str: A string representation of converted tools.
        """

        converted = []
        # delete parameters key if its empty in tool
        for tool in tools:
            if (
                "parameters" in tool["function"]
                and "properties" in tool["function"]["parameters"]
                and not tool["function"]["parameters"]["properties"]
            ):
                tool_copy = copy.deepcopy(tool)
                del tool_copy["function"]["parameters"]
                converted.append(json.dumps(tool_copy["function"], ensure_ascii=False))
            else:
                converted.append(json.dumps(tool["function"], ensure_ascii=False))
        return "\n".join(converted)
