use common::{
    configuration::LlmRoute,
    consts::{SYSTEM_ROLE, TOOL_ROLE, USER_ROLE},
};
use hermesllm::providers::openai::types::{ChatCompletionsRequest, ContentType, Message};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use super::router_model::{RouterModel, RoutingModelError};

pub const MAX_TOKEN_LEN: usize = 2048; // Default max token length for the routing model
pub const ARCH_ROUTER_V1_SYSTEM_PROMPT: &str = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
{routes}
</routes>

<conversation>
{conversation}
</conversation>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
2. You must analyze the route descriptions and find the best match route for user latest intent.
3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}
"#;

pub type Result<T> = std::result::Result<T, RoutingModelError>;
pub struct RouterModelV1 {
    llm_route_json_str: String,
    routing_model: String,
    max_token_length: usize,
}
impl RouterModelV1 {
    pub fn new(llm_routes: Vec<LlmRoute>, routing_model: String, max_token_length: usize) -> Self {
        let llm_route_json_str =
            serde_json::to_string(&llm_routes).unwrap_or_else(|_| "[]".to_string());
        RouterModelV1 {
            routing_model,
            max_token_length,
            llm_route_json_str,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmRouterResponse {
    pub route: Option<String>,
}

const TOKEN_LENGTH_DIVISOR: usize = 4; // Approximate token length divisor for UTF-8 characters

impl RouterModel for RouterModelV1 {
    fn generate_request(&self, messages: &[Message]) -> ChatCompletionsRequest {
        // remove system prompt, tool calls, tool call response and messages without content
        // if content is empty its likely a tool call
        // when role == tool its tool call response
        let messages_vec = messages
            .iter()
            .filter(|m| m.role != SYSTEM_ROLE && m.role != TOOL_ROLE && m.content.is_some())
            .collect::<Vec<&Message>>();

        // Following code is to ensure that the conversation does not exceed max token length
        // Note: we use a simple heuristic to estimate token count based on character length to optimize for performance
        let mut token_count = ARCH_ROUTER_V1_SYSTEM_PROMPT.len() / TOKEN_LENGTH_DIVISOR;
        let mut selected_messages_list_reversed: Vec<&Message> = vec![];
        for (selected_messsage_count, message) in messages_vec.iter().rev().enumerate() {
            let message_token_count = message
                .content
                .as_ref()
                .unwrap_or(&ContentType::Text("".to_string()))
                .to_string()
                .len()
                / TOKEN_LENGTH_DIVISOR;
            token_count += message_token_count;
            if token_count > self.max_token_length {
                debug!(
                      "RouterModelV1: token count {} exceeds max token length {}, truncating conversation, selected message count {}, total message count: {}",
                      token_count,
                      self.max_token_length
                      , selected_messsage_count,
                      messages_vec.len()
                  );
                if message.role == USER_ROLE {
                    // If message that exceeds max token length is from user, we need to keep it
                    selected_messages_list_reversed.push(message);
                }
                break;
            }
            // If we are here, it means that the message is within the max token length
            selected_messages_list_reversed.push(message);
        }

        if selected_messages_list_reversed.is_empty() {
            debug!(
                "RouterModelV1: no messages selected, using the last message in the conversation"
            );
            if let Some(last_message) = messages_vec.last() {
                selected_messages_list_reversed.push(last_message);
            }
        }

        // ensure that first and last selected message is from user
        if let Some(first_message) = selected_messages_list_reversed.first() {
            if first_message.role != USER_ROLE {
                warn!("RouterModelV1: last message in the conversation is not from user, this may lead to incorrect routing");
            }
        }
        if let Some(last_message) = selected_messages_list_reversed.last() {
            if last_message.role != USER_ROLE {
                warn!("RouterModelV1: first message in the conversation is not from user, this may lead to incorrect routing");
            }
        }

        // Reverse the selected messages to maintain the conversation order
        let selected_conversation_list = selected_messages_list_reversed
            .iter()
            .rev()
            .map(|message| {
                Message {
                    role: message.role.clone(),
                    // we can unwrap here because we have already filtered out messages without content
                    content: Some(ContentType::Text(
                        message.content.as_ref().unwrap().to_string(),
                    )),
                }
            })
            .collect::<Vec<Message>>();

        let messages_content = ARCH_ROUTER_V1_SYSTEM_PROMPT
            .replace("{routes}", &self.llm_route_json_str)
            .replace(
                "{conversation}",
                &serde_json::to_string(&selected_conversation_list).unwrap_or_default(),
            );

        ChatCompletionsRequest {
            model: self.routing_model.clone(),
            messages: vec![Message {
                content: Some(ContentType::Text(messages_content)),
                role: USER_ROLE.to_string(),
            }],
            ..Default::default()
        }
    }

    fn parse_response(&self, content: &str) -> Result<Option<String>> {
        if content.is_empty() {
            return Ok(None);
        }
        let router_resp_fixed = fix_json_response(content);
        let router_response: LlmRouterResponse = serde_json::from_str(router_resp_fixed.as_str())?;

        let selected_llm = router_response.route.unwrap_or_default().to_string();

        if selected_llm.is_empty() {
            return Ok(None);
        }

        Ok(Some(selected_llm))
    }

    fn get_model_name(&self) -> String {
        self.routing_model.clone()
    }
}

fn fix_json_response(body: &str) -> String {
    let mut updated_body = body.to_string();

    updated_body = updated_body.replace("'", "\"");

    if updated_body.contains("\\n") {
        updated_body = updated_body.replace("\\n", "");
    }

    if updated_body.starts_with("```json") {
        updated_body = updated_body
            .strip_prefix("```json")
            .unwrap_or(&updated_body)
            .to_string();
    }

    if updated_body.ends_with("```") {
        updated_body = updated_body
            .strip_suffix("```")
            .unwrap_or(&updated_body)
            .to_string();
    }

    updated_body
}

impl std::fmt::Debug for dyn RouterModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RouterModel")
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::tracing::init_tracer;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_system_prompt_format() {
        let expected_prompt = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
[{"name":"Image generation","description":"generating image"},{"name":"image conversion","description":"convert images to provided format"},{"name":"image search","description":"search image"},{"name":"Audio Processing","description":"Analyzing and interpreting audio input including speech, music, and environmental sounds"},{"name":"Speech Recognition","description":"Converting spoken language into written text"}]
</routes>

<conversation>
[{"role":"user","content":"hi"},{"role":"assistant","content":"Hello! How can I assist you today?"},{"role":"user","content":"given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson"}]
</conversation>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
2. You must analyze the route descriptions and find the best match route for user latest intent.
3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}
"#;
        let routes_str = r#"
          [
              {"name": "Image generation", "description": "generating image"},
              {"name": "image conversion", "description": "convert images to provided format"},
              {"name": "image search", "description": "search image"},
              {"name": "Audio Processing", "description": "Analyzing and interpreting audio input including speech, music, and environmental sounds"},
              {"name": "Speech Recognition", "description": "Converting spoken language into written text"}
          ]
        "#;
        let llm_routes = serde_json::from_str::<Vec<LlmRoute>>(routes_str).unwrap();
        let routing_model = "test-model".to_string();
        let router = RouterModelV1::new(llm_routes, routing_model.clone(), usize::MAX);

        let conversation_str = r#"
                    [
                        {
                            "role": "user",
                            "content": "hi"
                        },
                        {
                            "role": "assistant",
                            "content": "Hello! How can I assist you today?"
                        },
                        {
                            "role": "user",
                            "content": "given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson"
                        }
                    ]
        "#;
        let conversation: Vec<Message> = serde_json::from_str(conversation_str).unwrap();

        let req = router.generate_request(&conversation);

        let prompt = req.messages[0].content.as_ref().unwrap();

        assert_eq!(expected_prompt, prompt.to_string());
    }

    #[test]
    fn test_conversation_exceed_token_count() {
        let _tracer = init_tracer();
        let expected_prompt = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
[{"name":"Image generation","description":"generating image"},{"name":"image conversion","description":"convert images to provided format"},{"name":"image search","description":"search image"},{"name":"Audio Processing","description":"Analyzing and interpreting audio input including speech, music, and environmental sounds"},{"name":"Speech Recognition","description":"Converting spoken language into written text"}]
</routes>

<conversation>
[{"role":"user","content":"given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson"}]
</conversation>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
2. You must analyze the route descriptions and find the best match route for user latest intent.
3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}
"#;

        let routes_str = r#"
          [
              {"name": "Image generation", "description": "generating image"},
              {"name": "image conversion", "description": "convert images to provided format"},
              {"name": "image search", "description": "search image"},
              {"name": "Audio Processing", "description": "Analyzing and interpreting audio input including speech, music, and environmental sounds"},
              {"name": "Speech Recognition", "description": "Converting spoken language into written text"}
          ]
        "#;
        let llm_routes = serde_json::from_str::<Vec<LlmRoute>>(routes_str).unwrap();
        let routing_model = "test-model".to_string();
        let router = RouterModelV1::new(llm_routes, routing_model.clone(), 235);

        let conversation_str = r#"
                    [
                        {
                            "role": "user",
                            "content": "hi"
                        },
                        {
                            "role": "assistant",
                            "content": "Hello! How can I assist you today?"
                        },
                        {
                            "role": "user",
                            "content": "given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson"
                        }
                    ]
        "#;

        let conversation: Vec<Message> = serde_json::from_str(conversation_str).unwrap();

        let req = router.generate_request(&conversation);

        let prompt = req.messages[0].content.as_ref().unwrap();

        assert_eq!(expected_prompt, prompt.to_string());
    }

    #[test]
    fn test_conversation_exceed_token_count_large_single_message() {
        let _tracer = init_tracer();
        let expected_prompt = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
[{"name":"Image generation","description":"generating image"},{"name":"image conversion","description":"convert images to provided format"},{"name":"image search","description":"search image"},{"name":"Audio Processing","description":"Analyzing and interpreting audio input including speech, music, and environmental sounds"},{"name":"Speech Recognition","description":"Converting spoken language into written text"}]
</routes>

<conversation>
[{"role":"user","content":"given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson and this is a very long message that exceeds the max token length of the routing model, so it should be truncated and only the last user message should be included in the conversation for routing."}]
</conversation>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
2. You must analyze the route descriptions and find the best match route for user latest intent.
3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}
"#;

        let routes_str = r#"
          [
              {"name": "Image generation", "description": "generating image"},
              {"name": "image conversion", "description": "convert images to provided format"},
              {"name": "image search", "description": "search image"},
              {"name": "Audio Processing", "description": "Analyzing and interpreting audio input including speech, music, and environmental sounds"},
              {"name": "Speech Recognition", "description": "Converting spoken language into written text"}
          ]
        "#;
        let llm_routes = serde_json::from_str::<Vec<LlmRoute>>(routes_str).unwrap();
        let routing_model = "test-model".to_string();
        let router = RouterModelV1::new(llm_routes, routing_model.clone(), 200);

        let conversation_str = r#"
                    [
                        {
                            "role": "user",
                            "content": "hi"
                        },
                        {
                            "role": "assistant",
                            "content": "Hello! How can I assist you today?"
                        },
                        {
                            "role": "user",
                            "content": "given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson and this is a very long message that exceeds the max token length of the routing model, so it should be truncated and only the last user message should be included in the conversation for routing."
                        }
                    ]
        "#;

        let conversation: Vec<Message> = serde_json::from_str(conversation_str).unwrap();

        let req = router.generate_request(&conversation);

        let prompt = req.messages[0].content.as_ref().unwrap();

        assert_eq!(expected_prompt, prompt.to_string());
    }

    #[test]
    fn test_conversation_trim_upto_user_message() {
        let _tracer = init_tracer();
        let expected_prompt = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
[{"name":"Image generation","description":"generating image"},{"name":"image conversion","description":"convert images to provided format"},{"name":"image search","description":"search image"},{"name":"Audio Processing","description":"Analyzing and interpreting audio input including speech, music, and environmental sounds"},{"name":"Speech Recognition","description":"Converting spoken language into written text"}]
</routes>

<conversation>
[{"role":"user","content":"given the image In style of Andy Warhol"},{"role":"assistant","content":"ok here is the image"},{"role":"user","content":"pls give me another image about Bart and Lisa"}]
</conversation>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
2. You must analyze the route descriptions and find the best match route for user latest intent.
3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}
"#;

        let routes_str = r#"
          [
              {"name": "Image generation", "description": "generating image"},
              {"name": "image conversion", "description": "convert images to provided format"},
              {"name": "image search", "description": "search image"},
              {"name": "Audio Processing", "description": "Analyzing and interpreting audio input including speech, music, and environmental sounds"},
              {"name": "Speech Recognition", "description": "Converting spoken language into written text"}
          ]
        "#;
        let llm_routes = serde_json::from_str::<Vec<LlmRoute>>(routes_str).unwrap();
        let routing_model = "test-model".to_string();
        let router = RouterModelV1::new(llm_routes, routing_model.clone(), 230);

        let conversation_str = r#"
                    [
                        {
                            "role": "user",
                            "content": "hi"
                        },
                        {
                            "role": "assistant",
                            "content": "Hello! How can I assist you today?"
                        },
                        {
                            "role": "user",
                            "content": "given the image In style of Andy Warhol"
                        },
                        {
                            "role": "assistant",
                            "content": "ok here is the image"
                        },
                        {
                            "role": "user",
                            "content": "pls give me another image about Bart and Lisa"
                        }
                    ]
        "#;

        let conversation: Vec<Message> = serde_json::from_str(conversation_str).unwrap();

        let req = router.generate_request(&conversation);

        let prompt = req.messages[0].content.as_ref().unwrap();

        assert_eq!(expected_prompt, prompt.to_string());
    }

    #[test]
    fn test_non_text_input() {
        let expected_prompt = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
[{"name":"Image generation","description":"generating image"},{"name":"image conversion","description":"convert images to provided format"},{"name":"image search","description":"search image"},{"name":"Audio Processing","description":"Analyzing and interpreting audio input including speech, music, and environmental sounds"},{"name":"Speech Recognition","description":"Converting spoken language into written text"}]
</routes>

<conversation>
[{"role":"user","content":"hi"},{"role":"assistant","content":"Hello! How can I assist you today?"},{"role":"user","content":"given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson"}]
</conversation>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
2. You must analyze the route descriptions and find the best match route for user latest intent.
3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}
"#;
        let routes_str = r#"
          [
              {"name": "Image generation", "description": "generating image"},
              {"name": "image conversion", "description": "convert images to provided format"},
              {"name": "image search", "description": "search image"},
              {"name": "Audio Processing", "description": "Analyzing and interpreting audio input including speech, music, and environmental sounds"},
              {"name": "Speech Recognition", "description": "Converting spoken language into written text"}
          ]
        "#;
        let llm_routes = serde_json::from_str::<Vec<LlmRoute>>(routes_str).unwrap();
        let routing_model = "test-model".to_string();
        let router = RouterModelV1::new(llm_routes, routing_model.clone(), usize::MAX);

        let conversation_str = r#"
                    [
                        {
                            "role": "user",
                            "content": [
                              {
                                "type": "text",
                                "text": "hi"
                              },
                              {
                                "type": "image_url",
                                "image_url": {
                                  "url": "https://example.com/image.png"
                                }
                              }
                            ]
                        },
                        {
                            "role": "assistant",
                            "content": "Hello! How can I assist you today?"
                        },
                        {
                            "role": "user",
                            "content": "given the image In style of Andy Warhol, portrait of Bart and Lisa Simpson"
                        }
                    ]
        "#;
        let conversation: Vec<Message> = serde_json::from_str(conversation_str).unwrap();

        let req = router.generate_request(&conversation);

        let prompt = req.messages[0].content.as_ref().unwrap();

        assert_eq!(expected_prompt, prompt.to_string());
    }

    #[test]
    fn test_skip_tool_call() {
        let expected_prompt = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
[{"name":"Image generation","description":"generating image"},{"name":"image conversion","description":"convert images to provided format"},{"name":"image search","description":"search image"},{"name":"Audio Processing","description":"Analyzing and interpreting audio input including speech, music, and environmental sounds"},{"name":"Speech Recognition","description":"Converting spoken language into written text"}]
</routes>

<conversation>
[{"role":"user","content":"What's the weather like in Tokyo?"},{"role":"assistant","content":"The current weather in Tokyo is 22°C and sunny."},{"role":"user","content":"What about in New York?"}]
</conversation>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
2. You must analyze the route descriptions and find the best match route for user latest intent.
3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}
"#;
        let routes_str = r#"
          [
              {"name": "Image generation", "description": "generating image"},
              {"name": "image conversion", "description": "convert images to provided format"},
              {"name": "image search", "description": "search image"},
              {"name": "Audio Processing", "description": "Analyzing and interpreting audio input including speech, music, and environmental sounds"},
              {"name": "Speech Recognition", "description": "Converting spoken language into written text"}
          ]
        "#;
        let llm_routes = serde_json::from_str::<Vec<LlmRoute>>(routes_str).unwrap();
        let routing_model = "test-model".to_string();
        let router = RouterModelV1::new(llm_routes, routing_model.clone(), usize::MAX);

        let conversation_str = r#"
                                                [
                                                  {
                                                    "role": "user",
                                                    "content": "What's the weather like in Tokyo?"
                                                  },
                                                  {
                                                    "role": "assistant",
                                                    "content": null,
                                                    "tool_calls": [
                                                      {
                                                        "id": "toolcall-abc123",
                                                        "type": "function",
                                                        "function": {
                                                          "name": "get_weather",
                                                          "arguments": { "location": "Tokyo" }
                                                        }
                                                      }
                                                    ]
                                                  },
                                                  {
                                                    "role": "tool",
                                                    "tool_call_id": "toolcall-abc123",
                                                    "content": "{ \"temperature\": \"22°C\", \"condition\": \"Sunny\" }"
                                                  },
                                                  {
                                                    "role": "assistant",
                                                    "content": "The current weather in Tokyo is 22°C and sunny."
                                                  },
                                                  {
                                                    "role": "user",
                                                    "content": "What about in New York?"
                                                  }
                                                ]
        "#;

        // expects conversation to look like this

        // [
        //   {
        //     "role": "user",
        //     "content": "What's the weather like in Tokyo?"
        //   },
        //   {
        //     "role": "assistant",
        //     "content": "The current weather in Tokyo is 22°C and sunny."
        //   },
        //   {
        //     "role": "user",
        //     "content": "What about in New York?"
        //   }
        // ]

        let conversation: Vec<Message> = serde_json::from_str(conversation_str).unwrap();

        let req = router.generate_request(&conversation);

        let prompt = req.messages[0].content.as_ref().unwrap();

        assert_eq!(expected_prompt, prompt.to_string());
    }

    #[test]
    fn test_parse_response() {
        let routes_str = r#"
[
    {"name": "Image generation", "description": "generating image"},
    {"name": "image conversion", "description": "convert images to provided format"},
    {"name": "image search", "description": "search image"},
    {"name": "Audio Processing", "description": "Analyzing and interpreting audio input including speech, music, and environmental sounds"},
    {"name": "Speech Recognition", "description": "Converting spoken language into written text"}
]
"#;
        let llm_routes = serde_json::from_str::<Vec<LlmRoute>>(routes_str).unwrap();

        let router = RouterModelV1::new(llm_routes, "test-model".to_string(), 2000);

        // Case 1: Valid JSON with non-empty route
        let input = r#"{"route": "route1"}"#;
        let result = router.parse_response(input).unwrap();
        assert_eq!(result, Some("route1".to_string()));

        // Case 2: Valid JSON with empty route
        let input = r#"{"route": ""}"#;
        let result = router.parse_response(input).unwrap();
        assert_eq!(result, None);

        // Case 3: Valid JSON with null route
        let input = r#"{"route": null}"#;
        let result = router.parse_response(input).unwrap();
        assert_eq!(result, None);

        // Case 4: JSON missing route field
        let input = r#"{}"#;
        let result = router.parse_response(input).unwrap();
        assert_eq!(result, None);

        // Case 4.1: empty string
        let input = r#""#;
        let result = router.parse_response(input).unwrap();
        assert_eq!(result, None);

        // Case 5: Malformed JSON
        let input = r#"{"route": "route1""#; // missing closing }
        let result = router.parse_response(input);
        assert!(result.is_err());

        // Case 6: Single quotes and \n in JSON
        let input = "{'route': 'route2'}\\n";
        let result = router.parse_response(input).unwrap();
        assert_eq!(result, Some("route2".to_string()));

        // Case 7: Code block marker
        let input = "```json\n{\"route\": \"route1\"}\n```";
        let result = router.parse_response(input).unwrap();
        assert_eq!(result, Some("route1".to_string()));
    }
}
