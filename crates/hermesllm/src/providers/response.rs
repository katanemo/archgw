use crate::providers::id::ProviderId;
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fmt;
use std::convert::TryFrom;
use std::str::FromStr;

use crate::apis::openai::ChatCompletionsResponse;
use crate::clients::endpoints::SupportedAPIs;
use crate::apis::anthropic::MessagesResponse;

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ProviderResponseType {
    ChatCompletionsResponse(ChatCompletionsResponse),
    MessagesResponse(MessagesResponse),
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ProviderStreamResponseType {
    ChatCompletionsStreamResponse(crate::apis::openai::ChatCompletionsStreamResponse),
    MessagesStreamEvent(crate::apis::anthropic::MessagesStreamEvent),
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

}

// ============================================================================
// SSE EVENT CONTAINER
// ============================================================================

/// Represents a single Server-Sent Event with the complete wire format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(rename = "data")]
    pub data: String,  // The JSON payload after "data: "

    #[serde(skip_serializing, skip_deserializing)]
    pub raw_line: String,  // The complete line as received including "data: " prefix and "\n\n"

    #[serde(skip_serializing, skip_deserializing)]
    pub provider_stream_response: Option<ProviderStreamResponseType>,  // Parsed provider stream response object
}

impl SseEvent {
    /// Check if this event represents the end of the stream
    pub fn is_done(&self) -> bool {
        self.data == "[DONE]"
    }

    /// Check if this event should be skipped during processing
    /// This includes ping messages and other provider-specific events that don't contain content
    pub fn should_skip(&self) -> bool {
        // Skip ping messages (commonly used by providers for connection keep-alive)
        self.data == r#"{"type": "ping"}"#
    }

    /// Get the parsed provider response if available
    pub fn provider_response(&self) -> Option<&ProviderStreamResponseType> {
        self.provider_stream_response.as_ref()
    }

    /// Parse the data field into a ProviderStreamResponse for the given API
    pub fn to_provider_stream_response(&self, client_api: &SupportedAPIs) -> Result<Box<dyn ProviderStreamResponse>, Box<dyn std::error::Error + Send + Sync>> {
        if self.is_done() {
            return Err("Cannot parse [DONE] event as ProviderStreamResponse".into());
        }

        match client_api {
            SupportedAPIs::OpenAIChatCompletions(_) => {
                let response: crate::apis::openai::ChatCompletionsStreamResponse =
                    serde_json::from_str(&self.data)?;
                Ok(Box::new(response))
            }
            SupportedAPIs::AnthropicMessagesAPI(_) => {
                let response: crate::apis::anthropic::MessagesStreamEvent =
                    serde_json::from_str(&self.data)?;
                Ok(Box::new(response))
            }
        }
    }
}

impl FromStr for SseEvent {
    type Err = SseParseError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        if line.starts_with("data: ") {
            let data = line[6..].to_string(); // Remove "data: " prefix
            if data.is_empty() {
                return Err(SseParseError {
                    message: "Empty data field is not a valid SSE event".to_string(),
                });
            }
            // [DONE] marker is a valid SSE event that indicates end of stream
            Ok(SseEvent {
                data,
                raw_line: format!("{}\n\n", line), // Store complete SSE format
                provider_stream_response: None, // Will be populated later via TryFrom
            })
        } else {
            Err(SseParseError {
                message: format!("Line does not start with 'data: ': {}", line),
            })
        }
    }
}

impl fmt::Display for SseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw_line)
    }
}

// --- Response transformation logic for client API compatibility ---
impl TryFrom<(&[u8], &SupportedAPIs, &ProviderId)> for ProviderResponseType {
    type Error = std::io::Error;

    fn try_from((bytes, client_api, provider_id): (&[u8], &SupportedAPIs, &ProviderId)) -> Result<Self, Self::Error> {
        let upstream_api = provider_id.compatible_api_for_client(client_api);

        // Step 1: Parse bytes using upstream API format (what the provider actually sent)
        // Step 2: Return response type that matches client API format (what client expects)
        match (&upstream_api, client_api) {
            // Upstream sent OpenAI format, client expects OpenAI format - direct pass-through
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let resp: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(resp))
            }
            // Upstream sent Anthropic format, client expects Anthropic format - direct pass-through
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let resp: MessagesResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::MessagesResponse(resp))
            }
            // Upstream sent Anthropic format, client expects OpenAI format - need transformation
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                // Parse as Anthropic Messages response first
                let anthropic_resp: MessagesResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // Transform to OpenAI ChatCompletions format using the transformer
                let chat_resp: ChatCompletionsResponse = anthropic_resp.try_into()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Transformation error: {}", e)))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(chat_resp))
            }
            // Upstream sent OpenAI format, client expects Anthropic format - need transformation
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                // Parse as OpenAI ChatCompletions response first
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
impl TryFrom<(&[u8], &SupportedAPIs, &ProviderId)> for ProviderStreamResponseType {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((bytes, client_api, provider_id): (&[u8], &SupportedAPIs, &ProviderId)) -> Result<Self, Self::Error> {
        let upstream_api = provider_id.compatible_api_for_client(client_api);

        // Step 1: Parse bytes using upstream API format (what the provider actually sent)
        // Step 2: Return response type that matches client API format (what client expects)
        match (&upstream_api, client_api) {
            // Upstream sent OpenAI format, client expects OpenAI format - direct pass-through
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let resp: crate::apis::openai::ChatCompletionsStreamResponse = serde_json::from_slice(bytes)?;
                Ok(ProviderStreamResponseType::ChatCompletionsStreamResponse(resp))
            }
            // Upstream sent Anthropic format, client expects Anthropic format - direct pass-through
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let resp: crate::apis::anthropic::MessagesStreamEvent = serde_json::from_slice(bytes)?;
                Ok(ProviderStreamResponseType::MessagesStreamEvent(resp))
            }
            // Upstream sent Anthropic format, client expects OpenAI format - need transformation
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                // Parse as Anthropic Messages stream event first
                let anthropic_resp: crate::apis::anthropic::MessagesStreamEvent = serde_json::from_slice(bytes)?;

                // Transform to OpenAI ChatCompletions stream format using the transformer
                let chat_resp: crate::apis::openai::ChatCompletionsStreamResponse = anthropic_resp.try_into()?;
                Ok(ProviderStreamResponseType::ChatCompletionsStreamResponse(chat_resp))
            }
            // Upstream sent OpenAI format, client expects Anthropic format - need transformation
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                // Parse as OpenAI ChatCompletions stream response first
                let openai_resp: crate::apis::openai::ChatCompletionsStreamResponse = serde_json::from_slice(bytes)?;

                // Transform to Anthropic Messages stream format using the transformer
                let messages_resp: crate::apis::anthropic::MessagesStreamEvent = openai_resp.try_into()?;
                Ok(ProviderStreamResponseType::MessagesStreamEvent(messages_resp))
            }
        }
    }
}

// TryFrom implementation to convert raw bytes to SseEvent with parsed provider response
impl TryFrom<(&[u8], &SupportedAPIs, &ProviderId)> for SseEvent {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((bytes, client_api, provider_id): (&[u8], &SupportedAPIs, &ProviderId)) -> Result<Self, Self::Error> {
        // Convert bytes to string
        let body_str = std::str::from_utf8(bytes)?;
        let mut sse_event: SseEvent = body_str.parse()?;

        // If not [DONE], parse the data as a provider stream response (business logic layer)
        if !sse_event.is_done() {
            // Use the new ProviderStreamResponseType::try_from to parse the JSON data
            let provider_response = ProviderStreamResponseType::try_from((sse_event.data.as_bytes(), client_api, provider_id))?;
            sse_event.provider_stream_response = Some(provider_response);
        }

        Ok(sse_event)
    }
}

// TryFrom implementation for transforming SseEvent between API formats
impl TryFrom<(SseEvent, &SupportedAPIs, &SupportedAPIs)> for SseEvent {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((mut event, upstream_api, client_api): (SseEvent, &SupportedAPIs, &SupportedAPIs)) -> Result<Self, Self::Error> {
        // If APIs are the same, no transformation needed
        if std::mem::discriminant(upstream_api) == std::mem::discriminant(client_api) {
            return Ok(event);
        }

        // Handle [DONE] events - they don't need transformation
        if event.is_done() {
            return Ok(event);
        }

        // Transform the data field based on API conversion
        let transformed_data = match (upstream_api, client_api) {
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                // Parse OpenAI response and convert to Anthropic
                let openai_response: crate::apis::openai::ChatCompletionsStreamResponse =
                    serde_json::from_str(&event.data)?;
                let anthropic_response: crate::apis::anthropic::MessagesStreamEvent =
                    openai_response.try_into()?;
                serde_json::to_string(&anthropic_response)?
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                // Parse Anthropic response and convert to OpenAI
                let anthropic_response: crate::apis::anthropic::MessagesStreamEvent =
                    serde_json::from_str(&event.data)?;
                let openai_response: crate::apis::openai::ChatCompletionsStreamResponse =
                    anthropic_response.try_into()?;
                serde_json::to_string(&openai_response)?
            }
            _ => {
                return Err(format!("Unsupported API transformation: {:?} -> {:?}", upstream_api, client_api).into());
            }
        };

        // Update the event with transformed data and reconstruct raw_line
        event.data = transformed_data;
        event.raw_line = format!("data: {}", event.data);

        Ok(event)
    }
}

// Into implementation to convert SseEvent to bytes for response buffer
impl Into<Vec<u8>> for SseEvent {
    fn into(self) -> Vec<u8> {
        format!("{}\n\n", self.raw_line).into_bytes()
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
}

impl<I> SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub fn new(lines: I) -> Self {
        Self { lines }
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
        for line in &mut self.lines {
            if let Ok(event) = line.as_ref().parse::<SseEvent>() {
                // Check if this is the [DONE] marker - if so, end the stream
                if event.is_done() {
                    return None;
                }
                // Skip events that should be filtered at the transport layer
                if event.should_skip() {
                    continue;
                }
                return Some(event);
            }
        }
        None
    }
}

/// Trait for token usage information
pub trait TokenUsage {
    fn completion_tokens(&self) -> usize;
    fn prompt_tokens(&self) -> usize;
    fn total_tokens(&self) -> usize;
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
        // Claude provider receives Anthropic response but client expects OpenAI format
        // Upstream API = Anthropic, Client API = OpenAI -> parse Anthropic, convert to OpenAI
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
        let line = r#"data: {"id":"test","object":"chat.completion.chunk"}"#;
        let event: Result<SseEvent, _> = line.parse();
        assert!(event.is_ok());
        let event = event.unwrap();
        assert_eq!(event.data, r#"{"id":"test","object":"chat.completion.chunk"}"#);

        // Test conversion back to line using Display trait
        let wire_format = event.to_string();
        assert_eq!(wire_format, "data: {\"id\":\"test\",\"object\":\"chat.completion.chunk\"}\n\n");

        // Test [DONE] marker - should be valid SSE event
        let done_line = "data: [DONE]";
        let done_result: Result<SseEvent, _> = done_line.parse();
        assert!(done_result.is_ok());
        let done_event = done_result.unwrap();
        assert_eq!(done_event.data, "[DONE]");
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
            data: r#"{"id":"test","object":"chat.completion.chunk"}"#.to_string(),
            raw_line: r#"data: {"id":"test","object":"chat.completion.chunk"}

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
            data: r#"{"type": "ping"}"#.to_string(),
            raw_line: r#"data: {"type": "ping"}"#.to_string(),
            provider_stream_response: None,
        };
        assert!(ping_event.should_skip());
        assert!(!ping_event.is_done());

        // Test normal event should not be skipped
        let normal_event = SseEvent {
            data: r#"{"id": "test", "object": "chat.completion.chunk"}"#.to_string(),
            raw_line: r#"data: {"id": "test", "object": "chat.completion.chunk"}"#.to_string(),
            provider_stream_response: None,
        };
        assert!(!normal_event.should_skip());
        assert!(!normal_event.is_done());

        // Test [DONE] event should not be skipped (but is handled separately)
        let done_event = SseEvent {
            data: "[DONE]".to_string(),
            raw_line: "data: [DONE]".to_string(),
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
        assert!(event1.data.contains("msg1"));
        assert!(!event1.should_skip());

        // Second event should be msg2 (ping filtered out)
        let event2 = iter.next().unwrap();
        assert!(event2.data.contains("msg2"));
        assert!(!event2.should_skip());

        // Iterator should end at [DONE] (no more events)
        assert!(iter.next().is_none());
    }
}
