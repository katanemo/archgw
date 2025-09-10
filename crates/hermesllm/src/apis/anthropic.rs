use crate::providers::response::TokenUsage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::collections::HashMap;

use super::ApiDefinition;
use crate::providers::request::{ProviderRequest, ProviderRequestError};
use crate::providers::response::{ProviderResponse, ProviderStreamResponse};
use crate::clients::transformer::ExtractText;
use crate::{MESSAGES_PATH};

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
            AnthropicApi::Messages => MESSAGES_PATH,
        }
    }

    fn from_endpoint(endpoint: &str) -> Option<Self> {
        match endpoint {
            MESSAGES_PATH => Some(AnthropicApi::Messages),
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

impl ExtractText for Vec<MessagesContentBlock> {
    fn extract_text(&self) -> String {
        self.iter()
            .filter_map(|block| match block {
                MessagesContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
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

impl ExtractText for MessagesMessageContent {
    fn extract_text(&self) -> String {
        match self {
            MessagesMessageContent::Single(text) => text.clone(),
            MessagesMessageContent::Blocks(parts) => parts.extract_text()
        }
    }
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

impl TryFrom<&[u8]> for MessagesRequest {
    type Error = serde_json::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}

impl TokenUsage for MessagesResponse {
    fn completion_tokens(&self) -> usize {
        self.usage.output_tokens as usize
    }
    fn prompt_tokens(&self) -> usize {
        self.usage.input_tokens as usize
    }
    fn total_tokens(&self) -> usize {
        (self.usage.input_tokens + self.usage.output_tokens) as usize
    }
}

impl ProviderResponse for MessagesResponse {
    fn usage(&self) -> Option<&dyn TokenUsage> {
        Some(self)
    }
    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        Some((self.usage.input_tokens as usize, self.usage.output_tokens as usize, (self.usage.input_tokens + self.usage.output_tokens) as usize))
    }
}

impl ProviderRequest for MessagesRequest {
    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: String) {
        self.model = model;
    }

    fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(false)
    }

    fn extract_messages_text(&self) -> String {
        let mut text_parts = Vec::new();

        // Include system prompt if present
        if let Some(system) = &self.system {
            match system {
                MessagesSystemPrompt::Single(s) => text_parts.push(s.clone()),
                MessagesSystemPrompt::Blocks(blocks) => {
                    for block in blocks {
                        if let MessagesContentBlock::Text { text } = block {
                            text_parts.push(text.clone());
                        }
                    }
                }
            }
        }

        // Extract text from all messages
        for message in &self.messages {
            match &message.content {
                MessagesMessageContent::Single(text) => text_parts.push(text.clone()),
                MessagesMessageContent::Blocks(blocks) => {
                    for block in blocks {
                        if let MessagesContentBlock::Text { text } = block {
                            text_parts.push(text.clone());
                        }
                    }
                }
            }
        }

        text_parts.join(" ")
    }

    fn get_recent_user_message(&self) -> Option<String> {
        // Find the most recent user message
        for message in self.messages.iter().rev() {
            if message.role == MessagesRole::User {
                match &message.content {
                    MessagesMessageContent::Single(text) => return Some(text.clone()),
                    MessagesMessageContent::Blocks(blocks) => {
                        for block in blocks {
                            if let MessagesContentBlock::Text { text } = block {
                                return Some(text.clone());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn to_bytes(&self) -> Result<Vec<u8>, ProviderRequestError> {
        serde_json::to_vec(self).map_err(|e| ProviderRequestError {
            message: format!("Failed to serialize MessagesRequest: {}", e),
            source: Some(Box::new(e)),
        })
    }

    fn metadata(&self) -> &Option<HashMap<String, Value>> {
       return  &self.metadata;
    }

    fn remove_metadata_key(&mut self, key: &str) -> bool {
        if let Some(ref mut metadata) = self.metadata {
            metadata.remove(key).is_some()
        } else {
            false
        }
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

impl MessagesRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessagesRole::User => "user",
            MessagesRole::Assistant => "assistant",
        }
    }
}

// Implement ProviderStreamResponse for MessagesStreamEvent
impl ProviderStreamResponse for MessagesStreamEvent {
    fn content_delta(&self) -> Option<&str> {
        match self {
            MessagesStreamEvent::ContentBlockDelta { delta, .. } => {
                if let MessagesContentDelta::TextDelta { text } = delta {
                    Some(text)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn is_final(&self) -> bool {
        matches!(self, MessagesStreamEvent::MessageStop)
    }

    fn role(&self) -> Option<&str> {
        match self {
            MessagesStreamEvent::MessageStart { message } => Some(message.role.as_str()),
            _ => None,
        }
    }

    fn event_type(&self) -> Option<&str> {
        Some(match self {
            MessagesStreamEvent::MessageStart { .. } => "message_start",
            MessagesStreamEvent::ContentBlockStart { .. } => "content_block_start",
            MessagesStreamEvent::ContentBlockDelta { .. } => "content_block_delta",
            MessagesStreamEvent::ContentBlockStop { .. } => "content_block_stop",
            MessagesStreamEvent::MessageDelta { .. } => "message_delta",
            MessagesStreamEvent::MessageStop => "message_stop",
            MessagesStreamEvent::Ping => "ping",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_anthropic_required_fields() {
        // Create a JSON object with only required fields
        let original_json = json!({
            "model": "claude-3-sonnet-20240229",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 100
        });

        // Deserialize JSON into MessagesRequest
        let deserialized_request: MessagesRequest = serde_json::from_value(original_json.clone()).unwrap();

        // Validate required fields are properly set
        assert_eq!(deserialized_request.model, "claude-3-sonnet-20240229");
        assert_eq!(deserialized_request.messages.len(), 1);
        assert_eq!(deserialized_request.max_tokens, 100);

        let message = &deserialized_request.messages[0];
        assert_eq!(message.role, MessagesRole::User);
        if let MessagesMessageContent::Single(content) = &message.content {
            assert_eq!(content, "Hello");
        } else {
            panic!("Expected single content");
        }

        // Validate optional fields are None
        assert!(deserialized_request.system.is_none());
        assert!(deserialized_request.container.is_none());
        assert!(deserialized_request.mcp_servers.is_none());
        assert!(deserialized_request.service_tier.is_none());
        assert!(deserialized_request.thinking.is_none());
        assert!(deserialized_request.temperature.is_none());
        assert!(deserialized_request.top_p.is_none());
        assert!(deserialized_request.top_k.is_none());
        assert!(deserialized_request.stream.is_none());
        assert!(deserialized_request.stop_sequences.is_none());
        assert!(deserialized_request.tools.is_none());
        assert!(deserialized_request.tool_choice.is_none());
        assert!(deserialized_request.metadata.is_none());

        // Serialize back to JSON and compare
        let serialized_json = serde_json::to_value(&deserialized_request).unwrap();
        assert_eq!(original_json, serialized_json);
    }

    #[test]
    fn test_anthropic_optional_fields() {
        // Create a JSON object with optional fields set
        let original_json = json!({
            "model": "claude-3-sonnet-20240229",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 100,
            "temperature": 0.7,
            "top_p": 0.9,
            "system": "You are a helpful assistant",
            "service_tier": "auto",
            "thinking": {
                "enabled": true
            },
            "metadata": {
                "user_id": "123"
            }
        });

        // Deserialize JSON into MessagesRequest
        let deserialized_request: MessagesRequest = serde_json::from_value(original_json.clone()).unwrap();

        // Validate required fields
        assert_eq!(deserialized_request.model, "claude-3-sonnet-20240229");
        assert_eq!(deserialized_request.messages.len(), 1);
        assert_eq!(deserialized_request.max_tokens, 100);

        // Validate optional fields are properly set
        assert!((deserialized_request.temperature.unwrap() - 0.7).abs() < 1e-6);
        assert!((deserialized_request.top_p.unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(deserialized_request.service_tier, Some(ServiceTier::Auto));

        if let Some(MessagesSystemPrompt::Single(system)) = &deserialized_request.system {
            assert_eq!(system, "You are a helpful assistant");
        } else {
            panic!("Expected single system prompt");
        }

        if let Some(thinking) = &deserialized_request.thinking {
            assert_eq!(thinking.enabled, true);
        } else {
            panic!("Expected thinking config");
        }

        assert!(deserialized_request.metadata.is_some());

        // Validate fields not in JSON are None
        assert!(deserialized_request.container.is_none());
        assert!(deserialized_request.mcp_servers.is_none());
        assert!(deserialized_request.top_k.is_none());
        assert!(deserialized_request.stream.is_none());
        assert!(deserialized_request.stop_sequences.is_none());
        assert!(deserialized_request.tools.is_none());
        assert!(deserialized_request.tool_choice.is_none());

        // Serialize back to JSON and compare (handle floating point precision)
        let serialized_json = serde_json::to_value(&deserialized_request).unwrap();

        // Compare all fields except floating point ones
        assert_eq!(serialized_json["model"], original_json["model"]);
        assert_eq!(serialized_json["messages"], original_json["messages"]);
        assert_eq!(serialized_json["max_tokens"], original_json["max_tokens"]);
        assert_eq!(serialized_json["system"], original_json["system"]);
        assert_eq!(serialized_json["service_tier"], original_json["service_tier"]);
        assert_eq!(serialized_json["thinking"], original_json["thinking"]);
        assert_eq!(serialized_json["metadata"], original_json["metadata"]);

        // Handle floating point fields with tolerance
        let original_temp = original_json["temperature"].as_f64().unwrap();
        let serialized_temp = serialized_json["temperature"].as_f64().unwrap();
        assert!((original_temp - serialized_temp).abs() < 1e-6);

        let original_top_p = original_json["top_p"].as_f64().unwrap();
        let serialized_top_p = serialized_json["top_p"].as_f64().unwrap();
        assert!((original_top_p - serialized_top_p).abs() < 1e-6);
    }

    #[test]
    fn test_anthropic_nested_types() {
        // Create a comprehensive JSON object with nested types - a MessagesRequest with complex message content and tools
        let original_json = json!({
            "model": "claude-3-sonnet-20240229",
            "max_tokens": 1000,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "What can you see in this image and what's the weather like?"
                        },
                        {
                            "type": "image",
                            "source": {
                                "base64": {
                                    "media_type": "image/jpeg",
                                    "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
                                }
                            }
                        }
                    ]
                },
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "thinking",
                            "text": "Let me analyze the image and then check the weather..."
                        },
                        {
                            "type": "text",
                            "text": "I can see the image. Let me check the weather for you."
                        },
                        {
                            "type": "tool_use",
                            "id": "toolu_weather123",
                            "name": "get_weather",
                            "input": {
                                "location": "San Francisco, CA"
                            }
                        }
                    ]
                }
            ],
            "tools": [
                {
                    "name": "get_weather",
                    "description": "Get current weather information for a location",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "The city and state, e.g. San Francisco, CA"
                            }
                        },
                        "required": ["location"]
                    }
                }
            ],
            "tool_choice": {
                "type": "auto"
            },
            "system": [
                {
                    "type": "text",
                    "text": "You are a helpful assistant that can analyze images and provide weather information."
                }
            ]
        });

        // Deserialize JSON into MessagesRequest
        let deserialized_request: MessagesRequest = serde_json::from_value(original_json.clone()).unwrap();

        // Validate top-level fields
        assert_eq!(deserialized_request.model, "claude-3-sonnet-20240229");
        assert_eq!(deserialized_request.max_tokens, 1000);
        assert_eq!(deserialized_request.messages.len(), 2);

        // Validate first message (user with text and image content)
        let user_message = &deserialized_request.messages[0];
        assert_eq!(user_message.role, MessagesRole::User);
        if let MessagesMessageContent::Blocks(ref content_blocks) = user_message.content {
            assert_eq!(content_blocks.len(), 2);

            // Validate text content block
            if let MessagesContentBlock::Text { text } = &content_blocks[0] {
                assert_eq!(text, "What can you see in this image and what's the weather like?");
            } else {
                panic!("Expected text content block");
            }

            // Validate image content block
            if let MessagesContentBlock::Image { ref source } = content_blocks[1] {
                if let MessagesImageSource::Base64 { media_type, data } = source {
                    assert_eq!(media_type, "image/jpeg");
                    assert_eq!(data, "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==");
                } else {
                    panic!("Expected base64 image source");
                }
            } else {
                panic!("Expected image content block");
            }
        } else {
            panic!("Expected content blocks for user message");
        }

        // Validate second message (assistant with thinking, text, and tool use)
        let assistant_message = &deserialized_request.messages[1];
        assert_eq!(assistant_message.role, MessagesRole::Assistant);
        if let MessagesMessageContent::Blocks(ref content_blocks) = assistant_message.content {
            assert_eq!(content_blocks.len(), 3);

            // Validate thinking content block
            if let MessagesContentBlock::Thinking { text } = &content_blocks[0] {
                assert_eq!(text, "Let me analyze the image and then check the weather...");
            } else {
                panic!("Expected thinking content block");
            }

            // Validate text content block
            if let MessagesContentBlock::Text { text } = &content_blocks[1] {
                assert_eq!(text, "I can see the image. Let me check the weather for you.");
            } else {
                panic!("Expected text content block");
            }

            // Validate tool use content block
            if let MessagesContentBlock::ToolUse { ref id, ref name, ref input } = content_blocks[2] {
                assert_eq!(id, "toolu_weather123");
                assert_eq!(name, "get_weather");
                assert_eq!(input["location"], "San Francisco, CA");
            } else {
                panic!("Expected tool use content block");
            }
        } else {
            panic!("Expected content blocks for assistant message");
        }

        // Validate tools array
        assert!(deserialized_request.tools.is_some());
        let tools = deserialized_request.tools.as_ref().unwrap();
        assert_eq!(tools.len(), 1);

        let tool = &tools[0];
        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.description, Some("Get current weather information for a location".to_string()));
        assert_eq!(tool.input_schema["type"], "object");
        assert!(tool.input_schema["properties"]["location"].is_object());

        // Validate tool choice
        assert!(deserialized_request.tool_choice.is_some());
        let tool_choice = deserialized_request.tool_choice.as_ref().unwrap();
        assert_eq!(tool_choice.kind, MessagesToolChoiceType::Auto);
        assert!(tool_choice.name.is_none());

        // Validate system prompt with content blocks
        assert!(deserialized_request.system.is_some());
        if let Some(MessagesSystemPrompt::Blocks(ref system_blocks)) = deserialized_request.system {
            assert_eq!(system_blocks.len(), 1);
            if let MessagesContentBlock::Text { text } = &system_blocks[0] {
                assert_eq!(text, "You are a helpful assistant that can analyze images and provide weather information.");
            } else {
                panic!("Expected text content block in system prompt");
            }
        } else {
            panic!("Expected system prompt with content blocks");
        }

        // Serialize back to JSON and compare
        let serialized_json = serde_json::to_value(&deserialized_request).unwrap();
        assert_eq!(original_json, serialized_json);
    }

    #[test]
    fn test_anthropic_mcp_server_configuration() {
        // Test MCP Server configuration with JSON-first approach
        let mcp_server_json = json!({
            "name": "test-server",
            "type": "url",
            "url": "https://example.com/mcp",
            "authorization_token": "secret-token",
            "tool_configuration": {
                "allowed_tools": ["tool1", "tool2"],
                "enabled": true
            }
        });

        let deserialized_mcp: McpServer = serde_json::from_value(mcp_server_json.clone()).unwrap();
        assert_eq!(deserialized_mcp.name, "test-server");
        assert_eq!(deserialized_mcp.server_type, McpServerType::Url);
        assert_eq!(deserialized_mcp.url, "https://example.com/mcp");
        assert_eq!(deserialized_mcp.authorization_token, Some("secret-token".to_string()));

        if let Some(tool_config) = &deserialized_mcp.tool_configuration {
            assert_eq!(tool_config.allowed_tools, Some(vec!["tool1".to_string(), "tool2".to_string()]));
            assert_eq!(tool_config.enabled, Some(true));
        } else {
            panic!("Expected tool configuration");
        }

        let serialized_mcp_json = serde_json::to_value(&deserialized_mcp).unwrap();
        assert_eq!(mcp_server_json, serialized_mcp_json);

        // Test MCP Server with minimal configuration (optional fields as None)
        let minimal_mcp_json = json!({
            "name": "minimal-server",
            "type": "url",
            "url": "https://minimal.com/mcp"
        });

        let deserialized_minimal: McpServer = serde_json::from_value(minimal_mcp_json.clone()).unwrap();
        assert_eq!(deserialized_minimal.name, "minimal-server");
        assert_eq!(deserialized_minimal.server_type, McpServerType::Url);
        assert_eq!(deserialized_minimal.url, "https://minimal.com/mcp");
        assert!(deserialized_minimal.authorization_token.is_none());
        assert!(deserialized_minimal.tool_configuration.is_none());

        let serialized_minimal_json = serde_json::to_value(&deserialized_minimal).unwrap();
        assert_eq!(minimal_mcp_json, serialized_minimal_json);
    }

    #[test]
    fn test_anthropic_response_types() {
        // Test MessagesResponse deserialization
        let response_json = json!({
            "id": "msg_01ABC123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello! How can I help you today?"
                }
            ],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25,
                "cache_creation_input_tokens": 5,
                "cache_read_input_tokens": 3
            }
        });

        let deserialized_response: MessagesResponse = serde_json::from_value(response_json.clone()).unwrap();
        assert_eq!(deserialized_response.id, "msg_01ABC123");
        assert_eq!(deserialized_response.obj_type, "message");
        assert_eq!(deserialized_response.role, MessagesRole::Assistant);
        assert_eq!(deserialized_response.model, "claude-3-sonnet-20240229");
        assert_eq!(deserialized_response.stop_reason, MessagesStopReason::EndTurn);
        assert!(deserialized_response.stop_sequence.is_none());
        assert!(deserialized_response.container.is_none());

        // Check content
        assert_eq!(deserialized_response.content.len(), 1);
        if let MessagesContentBlock::Text { text } = &deserialized_response.content[0] {
            assert_eq!(text, "Hello! How can I help you today?");
        } else {
            panic!("Expected text content block");
        }

        // Check usage
        assert_eq!(deserialized_response.usage.input_tokens, 10);
        assert_eq!(deserialized_response.usage.output_tokens, 25);
        assert_eq!(deserialized_response.usage.cache_creation_input_tokens, Some(5));
        assert_eq!(deserialized_response.usage.cache_read_input_tokens, Some(3));

        let serialized_response_json = serde_json::to_value(&deserialized_response).unwrap();
        assert_eq!(response_json, serialized_response_json);

        // Test streaming event
        let stream_event_json = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": " How"
            }
        });

        let deserialized_event: MessagesStreamEvent = serde_json::from_value(stream_event_json.clone()).unwrap();
        if let MessagesStreamEvent::ContentBlockDelta { index, ref delta } = deserialized_event {
            assert_eq!(index, 0);
            if let MessagesContentDelta::TextDelta { text } = delta {
                assert_eq!(text, " How");
            } else {
                panic!("Expected text delta");
            }
        } else {
            panic!("Expected content block delta event");
        }

        let serialized_event_json = serde_json::to_value(&deserialized_event).unwrap();
        assert_eq!(stream_event_json, serialized_event_json);
    }

    #[test]
    fn test_anthropic_tool_use_content() {
        // Test tool use and tool result content blocks
        let tool_use_json = json!({
            "type": "tool_use",
            "id": "toolu_01ABC123",
            "name": "get_weather",
            "input": {
                "location": "San Francisco, CA"
            }
        });

        let deserialized_tool_use: MessagesContentBlock = serde_json::from_value(tool_use_json.clone()).unwrap();
        if let MessagesContentBlock::ToolUse { ref id, ref name, ref input } = deserialized_tool_use {
            assert_eq!(id, "toolu_01ABC123");
            assert_eq!(name, "get_weather");
            assert_eq!(input["location"], "San Francisco, CA");
        } else {
            panic!("Expected tool use content block");
        }

        let serialized_tool_use_json = serde_json::to_value(&deserialized_tool_use).unwrap();
        assert_eq!(tool_use_json, serialized_tool_use_json);

        // Test tool result content block
        let tool_result_json = json!({
            "type": "tool_result",
            "tool_use_id": "toolu_01ABC123",
            "content": [
                {
                    "type": "text",
                    "text": "The weather in San Francisco is sunny, 72°F"
                }
            ]
        });

        let deserialized_tool_result: MessagesContentBlock = serde_json::from_value(tool_result_json.clone()).unwrap();
        if let MessagesContentBlock::ToolResult { ref tool_use_id, ref is_error, ref content } = deserialized_tool_result {
            assert_eq!(tool_use_id, "toolu_01ABC123");
            assert!(is_error.is_none());
            assert_eq!(content.len(), 1);
            if let MessagesContentBlock::Text { text } = &content[0] {
                assert_eq!(text, "The weather in San Francisco is sunny, 72°F");
            } else {
                panic!("Expected text content in tool result");
            }
        } else {
            panic!("Expected tool result content block");
        }

        let serialized_tool_result_json = serde_json::to_value(&deserialized_tool_result).unwrap();
        assert_eq!(tool_result_json, serialized_tool_result_json);
    }

    #[test]
    fn test_anthropic_api_provider_trait_implementation() {
        // Test that AnthropicApi implements ApiDefinition trait correctly
        let api = AnthropicApi::Messages;

        // Test trait methods
        assert_eq!(api.endpoint(), MESSAGES_PATH);
        assert!(api.supports_streaming());
        assert!(api.supports_tools());
        assert!(api.supports_vision());

        // Test from_endpoint trait method
        let found_api = AnthropicApi::from_endpoint(MESSAGES_PATH);
        assert_eq!(found_api, Some(AnthropicApi::Messages));

        let not_found = AnthropicApi::from_endpoint("/v1/unknown");
        assert_eq!(not_found, None);

        // Test all_variants
        let all_variants = AnthropicApi::all_variants();
        assert_eq!(all_variants.len(), 1);
        assert_eq!(all_variants[0], AnthropicApi::Messages);
    }
}
