use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::collections::HashMap;

use super::ApiDefinition;

// Enum for all supported Anthropic APIs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnthropicApi {
    Messages,
    // Future APIs can be added here:
    // Embeddings,
    // etc.
}

impl ApiDefinition for AnthropicApi {
    fn endpoint(&self) -> &'static str {
        match self {
            AnthropicApi::Messages => "/v1/messages",
        }
    }

    fn from_endpoint(endpoint: &str) -> Option<Self> {
        match endpoint {
            "/v1/messages" => Some(AnthropicApi::Messages),
            _ => None,
        }
    }

    fn supports_streaming(&self) -> bool {
        match self {
            AnthropicApi::Messages => true,
        }
    }

    fn supports_tools(&self) -> bool {
        match self {
            AnthropicApi::Messages => true,
        }
    }

    fn supports_vision(&self) -> bool {
        match self {
            AnthropicApi::Messages => true,
        }
    }

    fn all_variants() -> Vec<Self> {
        vec![
            AnthropicApi::Messages,
        ]
    }
}

// Service tier enum for request priority
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    StandardOnly,
}

// Thinking configuration
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThinkingConfig {
    pub enabled: bool,
}

// MCP Server types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum McpServerType {
    Url,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpToolConfiguration {
    pub allowed_tools: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpServer {
    pub name: String,
    #[serde(rename = "type")]
    pub server_type: McpServerType,
    pub url: String,
    pub authorization_token: Option<String>,
    pub tool_configuration: Option<McpToolConfiguration>,
}


#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesRequest {
    pub model: String,
    pub messages: Vec<MessagesMessage>,
    pub max_tokens: u32,
    pub container: Option<String>,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub system: Option<MessagesSystemPrompt>,
    pub metadata: Option<HashMap<String, Value>>,
    pub service_tier: Option<ServiceTier>,
    pub thinking: Option<ThinkingConfig>,

    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub stream: Option<bool>,
    pub stop_sequences: Option<Vec<String>>,
    pub tools: Option<Vec<MessagesTool>>,
    pub tool_choice: Option<MessagesToolChoice>,

}


// Messages API specific types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessagesRole {
    User,
    Assistant,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum MessagesContentBlock {
    Text {
        text: String,
    },
    Thinking {
        text: String,
    },
    Image {
        source: MessagesImageSource,
    },
    Document {
        source: MessagesDocumentSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        is_error: Option<bool>,
        content: Vec<MessagesContentBlock>,
    },
    ServerToolUse {
        id: String,
        name: String,
        input: Value,
    },
    WebSearchToolResult {
        tool_use_id: String,
        is_error: Option<bool>,
        content: Vec<MessagesContentBlock>,
    },
    CodeExecutionToolResult {
        tool_use_id: String,
        is_error: Option<bool>,
        content: Vec<MessagesContentBlock>,
    },
    McpToolUse {
        id: String,
        name: String,
        input: Value,
    },
    McpToolResult {
        tool_use_id: String,
        is_error: Option<bool>,
        content: Vec<MessagesContentBlock>,
    },
    ContainerUpload {
        id: String,
        name: String,
        media_type: String,
        data: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MessagesImageSource {
    Base64 {
        media_type: String,
        data: String,
    },
    Url {
        url: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MessagesDocumentSource {
    Base64 {
        media_type: String,
        data: String,
    },
    Url {
        url: String,
    },
    File {
        file_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MessagesMessageContent {
    Single(String),
    Blocks(Vec<MessagesContentBlock>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MessagesSystemPrompt {
    Single(String),
    Blocks(Vec<MessagesContentBlock>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesMessage {
    pub role: MessagesRole,
    pub content: MessagesMessageContent,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessagesToolChoiceType {
    Auto,
    Any,
    Tool,
    None,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesToolChoice {
    #[serde(rename = "type")]
    pub kind: MessagesToolChoiceType,
    pub name: Option<String>,
    pub disable_parallel_tool_use: Option<bool>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessagesStopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    PauseTurn,
    Refusal,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
}

// Container response object
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesContainer {
    pub id: String,
    #[serde(rename = "type")]
    pub container_type: String,
    pub name: String,
    pub status: String,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub obj_type: String,
    pub role: MessagesRole,
    pub content: Vec<MessagesContentBlock>,
    pub model: String,
    pub stop_reason: MessagesStopReason,
    pub stop_sequence: Option<String>,
    pub usage: MessagesUsage,
    pub container: Option<MessagesContainer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum MessagesStreamEvent {
    MessageStart {
        message: MessagesStreamMessage,
    },
    ContentBlockStart {
        index: u32,
        content_block: MessagesContentBlock,
    },
    ContentBlockDelta {
        index: u32,
        delta: MessagesContentDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: MessagesMessageDelta,
        usage: MessagesUsage,
    },
    MessageStop,
    Ping,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesStreamMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub obj_type: String,
    pub role: MessagesRole,
    pub content: Vec<Value>, // Initially empty
    pub model: String,
    pub stop_reason: Option<MessagesStopReason>,
    pub stop_sequence: Option<String>,
    pub usage: MessagesUsage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MessagesContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagesMessageDelta {
    pub stop_reason: MessagesStopReason,
    pub stop_sequence: Option<String>,
}

// Helper functions for API detection and conversion
impl MessagesRequest {
    pub fn api_type() -> AnthropicApi {
        AnthropicApi::Messages
    }
}

impl MessagesResponse {
    pub fn api_type() -> AnthropicApi {
        AnthropicApi::Messages
    }
}

impl MessagesStreamEvent {
    pub fn api_type() -> AnthropicApi {
        AnthropicApi::Messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_anthropic_skip_serializing_none_annotations() {
        // Test that skip_serializing_none works correctly for MessagesRequest
        let request = MessagesRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            system: None,  // Should be skipped
            messages: vec![MessagesMessage {
                role: MessagesRole::User,
                content: MessagesMessageContent::Single("Hello".to_string()),
            }],
            max_tokens: 100,
            container: None,  // Should be skipped
            mcp_servers: None,  // Should be skipped
            service_tier: None,  // Should be skipped
            thinking: None,  // Should be skipped
            temperature: None,  // Should be skipped
            top_p: Some(0.9),   // Should be included
            top_k: None,        // Should be skipped
            stream: None,       // Should be skipped
            stop_sequences: None,  // Should be skipped
            tools: None,        // Should be skipped
            tool_choice: None,  // Should be skipped
            metadata: None,     // Should be skipped
        };

        let json = serde_json::to_value(&request).unwrap();
        let obj = json.as_object().unwrap();

        // Verify that None fields are not present in the JSON
        assert!(!obj.contains_key("system"));
        assert!(!obj.contains_key("container"));
        assert!(!obj.contains_key("mcp_servers"));
        assert!(!obj.contains_key("service_tier"));
        assert!(!obj.contains_key("thinking"));
        assert!(!obj.contains_key("temperature"));
        assert!(!obj.contains_key("top_k"));
        assert!(!obj.contains_key("stream"));
        assert!(!obj.contains_key("stop_sequences"));
        assert!(!obj.contains_key("tools"));
        assert!(!obj.contains_key("tool_choice"));
        assert!(!obj.contains_key("metadata"));

        // Verify that required fields and Some fields are present
        assert!(obj.contains_key("model"));
        assert!(obj.contains_key("messages"));
        assert!(obj.contains_key("max_tokens"));
        assert!(obj.contains_key("top_p"));  // This was Some(0.9)
    }

    #[test]
    fn test_anthropic_tool_serialization() {
        // Test MessagesTool with skip_serializing_none
        let tool = MessagesTool {
            name: "get_weather".to_string(),
            description: None,  // Should be skipped
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }),
        };

        let json = serde_json::to_value(&tool).unwrap();
        let obj = json.as_object().unwrap();

        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("input_schema"));
        assert!(!obj.contains_key("description"));  // Should be skipped

        // Test with description present
        let tool_with_desc = MessagesTool {
            name: "get_weather".to_string(),
            description: Some("Get weather information".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        };

        let json_with_desc = serde_json::to_value(&tool_with_desc).unwrap();
        let obj_with_desc = json_with_desc.as_object().unwrap();

        assert!(obj_with_desc.contains_key("description"));  // Should be included
    }

    #[test]
    fn test_mcp_server_serialization() {
        // Test McpServer with skip_serializing_none
        let mcp_server = McpServer {
            name: "test-server".to_string(),
            server_type: McpServerType::Url,
            url: "https://example.com/mcp".to_string(),
            authorization_token: None,  // Should be skipped
            tool_configuration: Some(McpToolConfiguration {
                allowed_tools: Some(vec!["tool1".to_string(), "tool2".to_string()]),
                enabled: None,  // Should be skipped
            }),
        };

        let json = serde_json::to_value(&mcp_server).unwrap();
        let obj = json.as_object().unwrap();

        // Verify required fields are present
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("type"));
        assert!(obj.contains_key("url"));
        assert!(obj.contains_key("tool_configuration"));

        // Verify None fields are not present
        assert!(!obj.contains_key("authorization_token"));

        // Check tool_configuration
        let tool_config = obj.get("tool_configuration").unwrap().as_object().unwrap();
        assert!(tool_config.contains_key("allowed_tools"));
        assert!(!tool_config.contains_key("enabled"));  // Should be skipped

        // Verify type serialization
        assert_eq!(obj.get("type").unwrap().as_str().unwrap(), "url");
    }

    #[test]
    fn test_service_tier_and_thinking_serialization() {
        // Test with service_tier and thinking enabled
        let request_with_fields = MessagesRequest {
            model: "claude-3-sonnet".to_string(),
            system: None,
            messages: vec![MessagesMessage {
                role: MessagesRole::User,
                content: MessagesMessageContent::Single("Hello".to_string()),
            }],
            max_tokens: 100,
            container: None,
            mcp_servers: None,
            service_tier: Some(ServiceTier::Auto),
            thinking: Some(ThinkingConfig { enabled: true }),
            temperature: None,
            top_p: None,
            top_k: None,
            stream: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            metadata: None,
        };

        let json = serde_json::to_value(&request_with_fields).unwrap();
        let obj = json.as_object().unwrap();

        // Verify that Some fields are present
        assert!(obj.contains_key("service_tier"));
        assert!(obj.contains_key("thinking"));

        // Verify service_tier serialization
        assert_eq!(obj.get("service_tier").unwrap().as_str().unwrap(), "auto");

        // Verify thinking serialization
        let thinking = obj.get("thinking").unwrap().as_object().unwrap();
        assert!(thinking.contains_key("enabled"));
        assert_eq!(thinking.get("enabled").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_anthropic_api_provider_trait_implementation() {
        use super::ApiDefinition;

        // Test that AnthropicApi implements ApiDefinition trait correctly
        let api = AnthropicApi::Messages;

        // Test trait methods
        assert_eq!(ApiDefinition::endpoint(&api), "/v1/messages");
        assert!(ApiDefinition::supports_streaming(&api));
        assert!(ApiDefinition::supports_tools(&api));
        assert!(ApiDefinition::supports_vision(&api));

        // Test from_endpoint trait method
        let found_api = AnthropicApi::from_endpoint("/v1/messages");
        assert_eq!(found_api, Some(AnthropicApi::Messages));

        let not_found = AnthropicApi::from_endpoint("/v1/unknown");
        assert_eq!(not_found, None);
    }
}
