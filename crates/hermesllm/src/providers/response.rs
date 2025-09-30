use crate::providers::id::ProviderId;
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fmt;
use std::convert::TryFrom;
use std::str::FromStr;

use crate::apis::openai::ChatCompletionsResponse;
use crate::apis::openai::ChatCompletionsStreamResponse;
use crate::apis::anthropic::MessagesStreamEvent;
use crate::clients::endpoints::SupportedAPIs;
use crate::apis::anthropic::MessagesResponse;

/// Trait for token usage information
pub trait TokenUsage {
    fn completion_tokens(&self) -> usize;
    fn prompt_tokens(&self) -> usize;
    fn total_tokens(&self) -> usize;
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ProviderResponseType {
    ChatCompletionsResponse(ChatCompletionsResponse),
    MessagesResponse(MessagesResponse),
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ProviderStreamResponseType {
    ChatCompletionsStreamResponse(ChatCompletionsStreamResponse),
    MessagesStreamEvent(MessagesStreamEvent),
}

pub trait ProviderResponse: Send + Sync {
    /// Get usage information if available - returns dynamic trait object
    fn usage(&self) -> Option<&dyn TokenUsage>;

    /// Extract token counts for metrics
    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        self.usage().map(|u| (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()))
    }
}

impl ProviderResponse for ProviderResponseType {
    fn usage(&self) -> Option<&dyn TokenUsage> {
        match self {
            ProviderResponseType::ChatCompletionsResponse(resp) => resp.usage(),
            ProviderResponseType::MessagesResponse(resp) => resp.usage(),
        }
    }

    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        match self {
            ProviderResponseType::ChatCompletionsResponse(resp) => resp.extract_usage_counts(),
            ProviderResponseType::MessagesResponse(resp) => resp.extract_usage_counts(),
        }
    }
}

pub trait ProviderStreamResponse: Send + Sync {
    /// Get the content delta for this chunk
    fn content_delta(&self) -> Option<&str>;

    /// Check if this is the final chunk in the stream
    fn is_final(&self) -> bool;

    /// Get role information if available
    fn role(&self) -> Option<&str>;

    /// Get event type for SSE streaming (used by Anthropic)
    fn event_type(&self) -> Option<&str>;
}

impl ProviderStreamResponse for ProviderStreamResponseType {
    fn content_delta(&self) -> Option<&str> {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.content_delta(),
            ProviderStreamResponseType::MessagesStreamEvent(resp) => resp.content_delta(),
        }
    }

    fn is_final(&self) -> bool {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.is_final(),
            ProviderStreamResponseType::MessagesStreamEvent(resp) => resp.is_final(),
        }
    }

    fn role(&self) -> Option<&str> {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.role(),
            ProviderStreamResponseType::MessagesStreamEvent(resp) => resp.role(),
        }
    }

    fn event_type(&self) -> Option<&str> {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(_resp) => None, // OpenAI doesn't use event types
            ProviderStreamResponseType::MessagesStreamEvent(resp) => resp.event_type(),
        }
    }
}

// ============================================================================
// SSE EVENT CONTAINER
// ============================================================================

/// Represents a single Server-Sent Event with the complete wire format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(rename = "data")]
    pub data: Option<String>,  // The JSON payload after "data: "

    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,  // Optional event type (e.g., "message_start", "content_block_delta")

    #[serde(skip_serializing, skip_deserializing)]
    pub raw_line: String,  // The complete line as received including "data: " prefix and "\n\n"

     #[serde(skip_serializing, skip_deserializing)]
    pub sse_transform_buffer: String,  // The complete line as received including "data: " prefix and "\n\n"

    #[serde(skip_serializing, skip_deserializing)]
    pub provider_stream_response: Option<ProviderStreamResponseType>,  // Parsed provider stream response object
}

impl SseEvent {
    /// Check if this event represents the end of the stream
    pub fn is_done(&self) -> bool {
        self.data == Some("[DONE]".into())
    }

    /// Check if this event should be skipped during processing
    /// This includes ping messages and other provider-specific events that don't contain content
    pub fn should_skip(&self) -> bool {
        // Skip ping messages (commonly used by providers for connection keep-alive)
        self.data == Some(r#"{"type": "ping"}"#.into())
    }

    /// Check if this is an event-only SSE event (no data payload)
    pub fn is_event_only(&self) -> bool {
        self.event.is_some() && self.data.is_none()
    }

    /// Get the parsed provider response if available
    pub fn provider_response(&self) -> Result<&dyn ProviderStreamResponse, std::io::Error> {
        self.provider_stream_response.as_ref()
            .map(|resp| resp as &dyn ProviderStreamResponse)
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Provider response not found")
            })
    }

}

impl FromStr for SseEvent {
    type Err = SseParseError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        if line.starts_with("data: ") {
            let data: String = line[6..].to_string(); // Remove "data: " prefix
            if data.is_empty() {
                return Err(SseParseError {
                    message: "Empty data field is not a valid SSE event".to_string(),
                });
            }
            Ok(SseEvent {
                data: Some(data),
                event: None,
                raw_line: line.to_string(),
                sse_transform_buffer: line.to_string(),
                provider_stream_response: None,
            })
        } else if line.starts_with("event: ") { //used by Anthropic
            let event_type = line[7..].to_string();
            if event_type.is_empty() {
                return Err(SseParseError {
                    message: "Empty event field is not a valid SSE event".to_string(),
                });
            }
            Ok(SseEvent {
                data: None,
                event: Some(event_type),
                raw_line: line.to_string(),
                sse_transform_buffer: line.to_string(),
                provider_stream_response: None,
            })
        } else {
            Err(SseParseError {
                message: format!("Line does not start with 'data: ' or 'event: ': {}", line),
            })
        }
    }
}

impl fmt::Display for SseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sse_transform_buffer)
    }
}

// Into implementation to convert SseEvent to bytes for response buffer
impl Into<Vec<u8>> for SseEvent {
    fn into(self) -> Vec<u8> {
        format!("{}\n\n", self.sse_transform_buffer).into_bytes()
    }
}


// --- Response transformation logic for client API compatibility ---
impl TryFrom<(&[u8], &SupportedAPIs, &ProviderId)> for ProviderResponseType {
    type Error = std::io::Error;

    fn try_from((bytes, client_api, provider_id): (&[u8], &SupportedAPIs, &ProviderId)) -> Result<Self, Self::Error> {
        let upstream_api = provider_id.compatible_api_for_client(client_api);
        match (&upstream_api, client_api) {
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let resp: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(resp))
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let resp: MessagesResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::MessagesResponse(resp))
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let anthropic_resp: MessagesResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // Transform to OpenAI ChatCompletions format using the transformer
                let chat_resp: ChatCompletionsResponse = anthropic_resp.try_into()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Transformation error: {}", e)))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(chat_resp))
            }
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let openai_resp: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // Transform to Anthropic Messages format using the transformer
                let messages_resp: MessagesResponse = openai_resp.try_into()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Transformation error: {}", e)))?;
                Ok(ProviderResponseType::MessagesResponse(messages_resp))
            }
        }
    }
}

// Stream response transformation logic for client API compatibility
impl TryFrom<(&[u8], &SupportedAPIs, &SupportedAPIs)> for ProviderStreamResponseType {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((bytes, client_api, upstream_api): (&[u8], &SupportedAPIs, &SupportedAPIs)) -> Result<Self, Self::Error> {
        match (upstream_api, client_api) {
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let resp: crate::apis::openai::ChatCompletionsStreamResponse = serde_json::from_slice(bytes)?;
                Ok(ProviderStreamResponseType::ChatCompletionsStreamResponse(resp))
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let resp: crate::apis::anthropic::MessagesStreamEvent = serde_json::from_slice(bytes)?;
                Ok(ProviderStreamResponseType::MessagesStreamEvent(resp))
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let anthropic_resp: crate::apis::anthropic::MessagesStreamEvent = serde_json::from_slice(bytes)?;

                // Transform to OpenAI ChatCompletions stream format using the transformer
                let chat_resp: crate::apis::openai::ChatCompletionsStreamResponse = anthropic_resp.try_into()?;
                Ok(ProviderStreamResponseType::ChatCompletionsStreamResponse(chat_resp))
            }
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                // Special case: Handle [DONE] marker for OpenAI -> Anthropic conversion
                if bytes == b"[DONE]" {
                    return Ok(ProviderStreamResponseType::MessagesStreamEvent(
                        crate::apis::anthropic::MessagesStreamEvent::MessageStop
                    ));
                }

                let openai_resp: crate::apis::openai::ChatCompletionsStreamResponse = serde_json::from_slice(bytes)?;

                // Transform to Anthropic Messages stream format using the transformer
                let messages_resp: crate::apis::anthropic::MessagesStreamEvent = openai_resp.try_into()?;
                Ok(ProviderStreamResponseType::MessagesStreamEvent(messages_resp))
            }
        }
    }
}

// TryFrom implementation to convert raw bytes to SseEvent with parsed provider response
impl TryFrom<(SseEvent, &SupportedAPIs, &SupportedAPIs)> for SseEvent {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((sse_event, client_api, upstream_api): (SseEvent, &SupportedAPIs, &SupportedAPIs)) -> Result<Self, Self::Error> {
        // Create a new transformed event based on the original
        let mut transformed_event = sse_event;

        // If has data, parse the data as a provider stream response (business logic layer)
        if transformed_event.data.is_some() {
            let data_str = transformed_event.data.as_ref().unwrap();
            let data_bytes = data_str.as_bytes();
            let transformed_response = ProviderStreamResponseType::try_from((data_bytes, client_api, upstream_api))?;
            let transformed_json = serde_json::to_string(&transformed_response)?;
            transformed_event.sse_transform_buffer = format!("data: {}\n\n", transformed_json);
            transformed_event.provider_stream_response = Some(transformed_response);
        }

        match (client_api, upstream_api) {
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                // No transformation needed
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                // No transformation needed
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                if let Some(provider_response) = &transformed_event.provider_stream_response {
                    if let Some(event_type) = provider_response.event_type() {
                        // This ensures the required Anthropic sequence: MessageStart → ContentBlockStart → ContentBlockDelta(s)
                        if event_type == "message_start" {
                            let content_block_start_json = serde_json::json!({
                                "type": "content_block_start",
                                "index": 0,
                                "content_block": {
                                    "type": "text",
                                    "text": ""
                                }
                            });
                            // Format as proper SSE: MessageStart first, then ContentBlockStart
                            transformed_event.sse_transform_buffer = format!(
                                "event: {}\n{}\nevent: content_block_start\ndata: {}\n\n",
                                event_type,
                                transformed_event.sse_transform_buffer,
                                content_block_start_json,
                            );
                        } else if event_type == "message_delta" {
                            let content_block_stop_json = serde_json::json!({
                                "type": "content_block_stop",
                                "index": 0
                            });
                            // Format as proper SSE: ContentBlockStop first, then MessageDelta
                            transformed_event.sse_transform_buffer = format!(
                                "event: content_block_stop\ndata: {}\n\nevent: {}\n{}",
                                content_block_stop_json,
                                event_type,
                                transformed_event.sse_transform_buffer
                            );
                        } else {
                            transformed_event.sse_transform_buffer = format!("event: {}\n{}", event_type, transformed_event.sse_transform_buffer);
                        }
                    }
                    // If event_type is None, we just keep the data line as-is without an event line
                    // This handles cases where the transformation might not produce a valid event type
                }
            }
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                if transformed_event.is_event_only() && transformed_event.event.is_some() {
                    transformed_event.sse_transform_buffer = format!("\n"); // suppress the event upstream for OpenAI
                }
            }
        }

        Ok(transformed_event)
    }
}

#[derive(Debug)]
pub struct SseParseError {
    pub message: String,
}

impl fmt::Display for SseParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SSE parse error: {}", self.message)
    }
}

impl Error for SseParseError {}

// ============================================================================
// GENERIC SSE STREAMING ITERATOR (Container Only)
// ============================================================================

/// Generic SSE (Server-Sent Events) streaming iterator container
/// Parses raw SSE lines into SseEvent objects
pub struct SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub lines: I,
    pub done_seen: bool,
}

impl<I> SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub fn new(lines: I) -> Self {
        Self { lines, done_seen: false }
    }
}

// TryFrom implementation to parse bytes into SseStreamIter
impl TryFrom<&[u8]> for SseStreamIter<std::vec::IntoIter<String>> {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
        Ok(SseStreamIter::new(lines.into_iter()))
    }
}

impl<I> Iterator for SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    type Item = SseEvent;

    fn next(&mut self) -> Option<Self::Item> {
        // If we already returned [DONE], terminate the stream
        if self.done_seen {
            return None;
        }

        for line in &mut self.lines {
            let line_str = line.as_ref();

            // Try to parse as either data: or event: line
            if let Ok(event) = line_str.parse::<SseEvent>() {
                // For data: lines, check if this is the [DONE] marker
                if event.data.is_some() && event.is_done() {
                    self.done_seen = true;
                    return Some(event); // Return [DONE] event for transformation
                }
                // For data: lines, skip events that should be filtered at the transport layer
                if event.data.is_some() && event.should_skip() {
                    continue;
                }
                return Some(event);
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct ProviderResponseError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}


impl fmt::Display for ProviderResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider response error: {}", self.message)
    }
}

impl Error for ProviderResponseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::endpoints::SupportedAPIs;
    use crate::providers::id::ProviderId;
    use crate::apis::openai::OpenAIApi;
    use crate::apis::anthropic::AnthropicApi;
    use serde_json::json;

    #[test]
    fn test_openai_response_from_bytes() {
        let resp = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": { "role": "assistant", "content": "Hello!" },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 5, "completion_tokens": 7, "total_tokens": 12 },
            "system_fingerprint": null
        });
        let bytes = serde_json::to_vec(&resp).unwrap();
        let result = ProviderResponseType::try_from((bytes.as_slice(), &SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions), &ProviderId::OpenAI));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::ChatCompletionsResponse(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.choices.len(), 1);
            },
            _ => panic!("Expected ChatCompletionsResponse variant"),
        }
    }

    #[test]
    fn test_anthropic_response_from_bytes() {
        let resp = json!({
            "id": "msg_01ABC123",
            "type": "message",
            "role": "assistant",
            "content": [
                { "type": "text", "text": "Hello! How can I help you today?" }
            ],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 10, "output_tokens": 25, "cache_creation_input_tokens": 5, "cache_read_input_tokens": 3 }
        });
        let bytes = serde_json::to_vec(&resp).unwrap();
        let result = ProviderResponseType::try_from((bytes.as_slice(), &SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages), &ProviderId::Anthropic));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::MessagesResponse(r) => {
                assert_eq!(r.model, "claude-3-sonnet-20240229");
                assert_eq!(r.content.len(), 1);
            },
            _ => panic!("Expected MessagesResponse variant"),
        }
    }

    #[test]
    fn test_anthropic_response_from_bytes_with_openai_provider() {
        // OpenAI provider receives OpenAI response but client expects Anthropic format
        // Upstream API = OpenAI, Client API = Anthropic -> parse OpenAI, convert to Anthropic
        let resp = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": { "role": "assistant", "content": "Hello! How can I help you today?" },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 10, "completion_tokens": 25, "total_tokens": 35 }
        });
        let bytes = serde_json::to_vec(&resp).unwrap();
        let result = ProviderResponseType::try_from((bytes.as_slice(), &SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages), &ProviderId::OpenAI));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::MessagesResponse(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.usage.input_tokens, 10);
                assert_eq!(r.usage.output_tokens, 25);
            },
            _ => panic!("Expected MessagesResponse variant"),
        }
    }

    #[test]
    fn test_openai_response_from_bytes_with_claude_provider() {
        // Claude provider using OpenAI-compatible API returns OpenAI format response
        // Client API = OpenAI, Provider = Anthropic -> Anthropic returns OpenAI format via their compatible API
        let resp = json!({
            "id": "chatcmpl-01ABC123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "claude-3-sonnet-20240229",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 25,
                "total_tokens": 35
            }
        });
        let bytes = serde_json::to_vec(&resp).unwrap();
        let result = ProviderResponseType::try_from((bytes.as_slice(), &SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions), &ProviderId::Anthropic));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::ChatCompletionsResponse(r) => {
                assert_eq!(r.model, "claude-3-sonnet-20240229");
                assert_eq!(r.usage.prompt_tokens, 10);
                assert_eq!(r.usage.completion_tokens, 25);
            },
            _ => panic!("Expected ChatCompletionsResponse variant"),
        }
    }

    #[test]
    fn test_sse_event_parsing() {
        // Test valid SSE data line
        let line = "data: {\"id\":\"test\",\"object\":\"chat.completion.chunk\"}\n\n";
        let event: Result<SseEvent, _> = line.parse();
        assert!(event.is_ok());
        let event = event.unwrap();
        assert_eq!(event.data, Some("{\"id\":\"test\",\"object\":\"chat.completion.chunk\"}\n\n".to_string()));

        // Test conversion back to line using Display trait
        let wire_format = event.to_string();
        assert_eq!(wire_format, "data: {\"id\":\"test\",\"object\":\"chat.completion.chunk\"}\n\n");

        // Test [DONE] marker - should be valid SSE event
        let done_line = "data: [DONE]";
        let done_result: Result<SseEvent, _> = done_line.parse();
        assert!(done_result.is_ok());
        let done_event = done_result.unwrap();
        assert_eq!(done_event.data, Some("[DONE]".to_string()));
        assert!(done_event.is_done()); // Test the helper method

        // Test non-DONE event
        assert!(!event.is_done());

        // Test empty data - should return error
        let empty_line = "data: ";
        let empty_result: Result<SseEvent, _> = empty_line.parse();
        assert!(empty_result.is_err());

        // Test non-data line - should return error
        let comment_line = ": this is a comment";
        let comment_result: Result<SseEvent, _> = comment_line.parse();
        assert!(comment_result.is_err());
    }

    #[test]
    fn test_sse_event_serde() {
        // Test serialization and deserialization with serde
        let event = SseEvent {
            data: Some(r#"{"id":"test","object":"chat.completion.chunk"}"#.to_string()),
            event: None,
            raw_line: r#"data: {"id":"test","object":"chat.completion.chunk"}

        "#.to_string(),
            sse_transform_buffer: r#"data: {"id":"test","object":"chat.completion.chunk"}

        "#.to_string(),
            provider_stream_response: None,
        };

        // Test JSON serialization - raw_line should be skipped
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("chat.completion.chunk"));
        assert!(!json.contains("raw_line")); // Should be excluded from serialization

        // Test JSON deserialization
        let deserialized: SseEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.data, event.data);
        assert_eq!(deserialized.raw_line, ""); // Should be empty since it's skipped

        // Test round trip for data field only
        assert_eq!(event.data, deserialized.data);
    }

    #[test]
    fn test_sse_event_should_skip() {
        // Test ping message should be skipped
        let ping_event = SseEvent {
            data: Some(r#"{"type": "ping"}"#.to_string()),
            event: None,
            raw_line: r#"data: {"type": "ping"}"#.to_string(),
            sse_transform_buffer: r#"data: {"type": "ping"}"#.to_string(),
            provider_stream_response: None,
        };
        assert!(ping_event.should_skip());
        assert!(!ping_event.is_done());

        // Test normal event should not be skipped
        let normal_event = SseEvent {
            data: Some(r#"{"id": "test", "object": "chat.completion.chunk"}"#.to_string()),
            event: Some("content_block_delta".to_string()),
            raw_line: r#"data: {"id": "test", "object": "chat.completion.chunk"}"#.to_string(),
            sse_transform_buffer: r#"data: {"id": "test", "object": "chat.completion.chunk"}"#.to_string(),
            provider_stream_response: None,
        };
        assert!(!normal_event.should_skip());
        assert!(!normal_event.is_done());

        // Test [DONE] event should not be skipped (but is handled separately)
        let done_event = SseEvent {
            data: Some("[DONE]".to_string()),
            event: None,
            raw_line: "data: [DONE]".to_string(),
            sse_transform_buffer: "data: [DONE]".to_string(),
            provider_stream_response: None,
        };
        assert!(!done_event.should_skip());
        assert!(done_event.is_done());
    }

    #[test]
    fn test_sse_stream_iter_filters_ping_messages() {
        // Create test data with ping messages mixed in
        let test_lines = vec![
            "data: {\"id\": \"msg1\", \"object\": \"chat.completion.chunk\"}".to_string(),
            "data: {\"type\": \"ping\"}".to_string(), // This should be filtered out
            "data: {\"id\": \"msg2\", \"object\": \"chat.completion.chunk\"}".to_string(),
            "data: {\"type\": \"ping\"}".to_string(), // This should be filtered out
            "data: [DONE]".to_string(), // This should end the stream
        ];

        let mut iter = SseStreamIter::new(test_lines.into_iter());

        // First event should be msg1 (ping filtered out)
        let event1 = iter.next().unwrap();
        assert!(event1.data.as_ref().unwrap().contains("msg1"));
        assert!(!event1.should_skip());

        // Second event should be msg2 (ping filtered out)
        let event2 = iter.next().unwrap();
        assert!(event2.data.as_ref().unwrap().contains("msg2"));
        assert!(!event2.should_skip());

        // Third event should be [DONE]
        let done_event = iter.next().unwrap();
        assert!(done_event.is_done());

        // Iterator should end after [DONE]
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_sse_stream_iter_handles_anthropic_events() {
        // Create test data with Anthropic-style event/data pairs
        let test_lines = vec![
            "event: message_start".to_string(),
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\"}}".to_string(),
            "event: content_block_delta".to_string(),
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}".to_string(),
            "data: [DONE]".to_string(),
        ];

        let mut iter = SseStreamIter::new(test_lines.into_iter());

        // First event should be the event: line
        let event1 = iter.next().unwrap();
        assert!(event1.is_event_only());
        assert_eq!(event1.event, Some("message_start".to_string()));
        assert_eq!(event1.data, None);

        // Second event should be the data: line
        let event2 = iter.next().unwrap();
        assert!(!event2.is_event_only());
        assert_eq!(event2.event, None);
        assert!(event2.data.as_ref().unwrap().contains("message_start"));

        // Third event should be another event: line
        let event3 = iter.next().unwrap();
        assert!(event3.is_event_only());
        assert_eq!(event3.event, Some("content_block_delta".to_string()));

        // Fourth event should be the content delta data
        let event4 = iter.next().unwrap();
        assert!(!event4.is_event_only());
        assert!(event4.data.as_ref().unwrap().contains("Hello"));

        // Fifth event should be [DONE]
        let done_event = iter.next().unwrap();
        assert!(done_event.is_done());

        // Iterator should end after [DONE]
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_provider_stream_response_event_type() {
        use crate::apis::anthropic::{MessagesStreamEvent, MessagesContentDelta};
        use crate::apis::openai::ChatCompletionsStreamResponse;

        // Test Anthropic event type
        let anthropic_event = MessagesStreamEvent::ContentBlockDelta {
            index: 0,
            delta: MessagesContentDelta::TextDelta { text: "Hello".to_string() },
        };
        let provider_type = ProviderStreamResponseType::MessagesStreamEvent(anthropic_event);
        assert_eq!(provider_type.event_type(), Some("content_block_delta"));

        // Test OpenAI event type (should be None)
        let openai_event = ChatCompletionsStreamResponse {
            id: "test".to_string(),
            object: Some("chat.completion.chunk".to_string()),
            created: 123456789,
            model: "gpt-4".to_string(),
            choices: vec![],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        };
        let provider_type = ProviderStreamResponseType::ChatCompletionsStreamResponse(openai_event);
        assert_eq!(provider_type.event_type(), None);
    }

    #[test]
    fn test_done_marker_handled_in_stream_response_transformation() {
        use crate::apis::anthropic::AnthropicApi;

        // Test that [DONE] marker is properly converted to MessageStop in the transformation layer
        let done_bytes = b"[DONE]";
        let client_api = SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);
        let upstream_api = SupportedAPIs::OpenAIChatCompletions(crate::apis::openai::OpenAIApi::ChatCompletions);

        let result = ProviderStreamResponseType::try_from((done_bytes.as_slice(), &client_api, &upstream_api));
        assert!(result.is_ok());

        if let Ok(ProviderStreamResponseType::MessagesStreamEvent(event)) = result {
            // Verify it's a MessageStop event
            assert_eq!(event.event_type(), Some("message_stop"));
            assert!(matches!(event, crate::apis::anthropic::MessagesStreamEvent::MessageStop));
        } else {
            panic!("Expected MessagesStreamEvent::MessageStop");
        }
    }
}
