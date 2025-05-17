use common::{
    api::open_ai::{ChatCompletionsRequest, Message},
    consts::{SYSTEM_ROLE, USER_ROLE},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::router_model::{RouterModel, RoutingModelError};

pub const ARCH_ROUTER_V1_SYSTEM_PROMPT: &str = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
{routes}
</routes>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant, response with empty route {"route": ""}.
2. If the user request is full fill and user thank or ending the conversation , response with empty route {"route": ""}.
3. Understand user latest intent and find the best match route in <routes></routes> xml tags.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}


<conversation>
{conversation}
</conversation>
"#;

pub type Result<T> = std::result::Result<T, RoutingModelError>;

pub struct RouterModelV1 {
    llm_providers_with_usage_yaml: String,
    routing_model: String,
}

impl RouterModelV1 {
    pub fn new(llm_providers_with_usage_yaml: String, routing_model: String) -> Self {
        RouterModelV1 {
            llm_providers_with_usage_yaml,
            routing_model,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmRouterResponse {
    pub route: Option<String>,
}

impl RouterModel for RouterModelV1 {
    fn generate_request(&self, messages: &[Message]) -> ChatCompletionsRequest {
        let messages_str = messages
            .iter()
            .filter(|m| m.role != SYSTEM_ROLE)
            .map(|m| {
                let content_json_str = serde_json::to_string(&m.content).unwrap_or_default();
                format!("{}: {}", m.role, content_json_str)
            })
            .collect::<Vec<String>>()
            .join("\n");

        let message = ARCH_ROUTER_V1_SYSTEM_PROMPT
            .replace("{routes}", &self.llm_providers_with_usage_yaml)
            .replace("{conversation}", messages_str.as_str());

        ChatCompletionsRequest {
            model: self.routing_model.clone(),
            messages: vec![Message {
                content: Some(message),
                role: USER_ROLE.to_string(),
                model: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            stream: false,
            stream_options: None,
            metadata: None,
        }
    }

    fn parse_response(&self, content: &str) -> Result<Option<String>> {
        if content.is_empty() {
            return Ok(None);
        }
        let router_resp_fixed = fix_json_response(content);
        info!(
            "router response (fixed): {}",
            router_resp_fixed.replace("\n", "\\n")
        );
        let router_response: LlmRouterResponse = serde_json::from_str(router_resp_fixed.as_str())?;

        let selected_llm = router_response.route.unwrap_or_default().to_string();

        if selected_llm.is_empty() {
            return Ok(None);
        }

        Ok(Some(selected_llm))
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
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_system_prompt_format() {
        let expected_prompt = r#"
You are a helpful assistant designed to find the best suited route.
You are provided with route description within <routes></routes> XML tags:
<routes>
route1: description1
route2: description2
</routes>

Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
1. If the latest intent from user is irrelevant, response with empty route {"route": ""}.
2. If the user request is full fill and user thank or ending the conversation , response with empty route {"route": ""}.
3. Understand user latest intent and find the best match route in <routes></routes> xml tags.

Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
{"route": "route_name"}


<conversation>
user: "Hello, I want to book a flight."
assistant: "Sure, where would you like to go?"
user: "seattle"
</conversation>
"#;

        let routes_yaml = "route1: description1\nroute2: description2";
        let routing_model = "test-model".to_string();
        let router = RouterModelV1::new(routes_yaml.to_string(), routing_model.clone());

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: Some("You are a helpful assistant.".to_string()),
                ..Default::default()
            },
            Message {
                role: "user".to_string(),
                content: Some("Hello, I want to book a flight.".to_string()),
                ..Default::default()
            },
            Message {
                role: "assistant".to_string(),
                content: Some("Sure, where would you like to go?".to_string()),
                ..Default::default()
            },
            Message {
                role: "user".to_string(),
                content: Some("seattle".to_string()),
                ..Default::default()
            },
        ];

        let req = router.generate_request(&messages);

        let prompt = req.messages[0].content.as_ref().unwrap();

        println!("Prompt: {}", prompt);

        assert_eq!(expected_prompt, prompt);
    }
}

#[test]
fn test_parse_response() {
    let router = RouterModelV1::new(
        "route1: description1\nroute2: description2".to_string(),
        "test-model".to_string(),
    );

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
