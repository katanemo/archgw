use serde::Serialize;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;

use crate::apis::amazon_bedrock::ConverseResponse;
use crate::apis::amazon_bedrock::ConverseStreamEvent;
use crate::apis::anthropic::MessagesResponse;
use crate::apis::anthropic::MessagesStreamEvent;
use crate::apis::openai::ChatCompletionsResponse;
use crate::apis::openai::ChatCompletionsStreamResponse;
use crate::apis::sse::SseEvent;
use crate::clients::endpoints::SupportedAPIs;
use crate::clients::endpoints::SupportedUpstreamAPIs;
use crate::providers::id::ProviderId;

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
    ConverseStreamEvent(ConverseStreamEvent),
}

pub trait ProviderResponse: Send + Sync {
    /// Get usage information if available - returns dynamic trait object
    fn usage(&self) -> Option<&dyn TokenUsage>;

    /// Extract token counts for metrics
    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        self.usage()
            .map(|u| (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()))
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
            ProviderStreamResponseType::ConverseStreamEvent(resp) => resp.content_delta(),
        }
    }

    fn is_final(&self) -> bool {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.is_final(),
            ProviderStreamResponseType::MessagesStreamEvent(resp) => resp.is_final(),
            ProviderStreamResponseType::ConverseStreamEvent(resp) => resp.is_final(),
        }
    }

    fn role(&self) -> Option<&str> {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.role(),
            ProviderStreamResponseType::MessagesStreamEvent(resp) => resp.role(),
            ProviderStreamResponseType::ConverseStreamEvent(resp) => resp.role(),
        }
    }

    fn event_type(&self) -> Option<&str> {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(_resp) => None, // OpenAI doesn't use event types
            ProviderStreamResponseType::MessagesStreamEvent(resp) => resp.event_type(),
            ProviderStreamResponseType::ConverseStreamEvent(resp) => resp.event_type(), // Bedrock doesn't use event types
        }
    }
}

impl Into<String> for ProviderStreamResponseType {
    fn into(self) -> String {
        match self {
            ProviderStreamResponseType::MessagesStreamEvent(event) => {
                // Use the Into<String> implementation for proper SSE formatting with event lines
                event.into()
            }
            ProviderStreamResponseType::ConverseStreamEvent(event) => {
                // Use the Into<String> implementation for proper SSE formatting with event lines
                event.into()
            }
            ProviderStreamResponseType::ChatCompletionsStreamResponse(_) => {
                // For OpenAI, use simple data line format
                let json = serde_json::to_string(&self).unwrap_or_default();
                format!("data: {}\n\n", json)
            }
        }
    }
}

// --- Response transformation logic for client API compatibility ---
impl TryFrom<(&[u8], &SupportedAPIs, &ProviderId)> for ProviderResponseType {
    type Error = std::io::Error;

    fn try_from(
        (bytes, client_api, provider_id): (&[u8], &SupportedAPIs, &ProviderId),
    ) -> Result<Self, Self::Error> {
        let upstream_api = provider_id.compatible_api_for_client(client_api, false);
        match (&upstream_api, client_api) {
            (
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
                SupportedAPIs::OpenAIChatCompletions(_),
            ) => {
                let resp: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(resp))
            }
            (
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
                SupportedAPIs::AnthropicMessagesAPI(_),
            ) => {
                let resp: MessagesResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::MessagesResponse(resp))
            }
            (
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
                SupportedAPIs::OpenAIChatCompletions(_),
            ) => {
                let anthropic_resp: MessagesResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // Transform to OpenAI ChatCompletions format using the transformer
                let chat_resp: ChatCompletionsResponse =
                    anthropic_resp.try_into().map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Transformation error: {}", e),
                        )
                    })?;
                Ok(ProviderResponseType::ChatCompletionsResponse(chat_resp))
            }
            (
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
                SupportedAPIs::AnthropicMessagesAPI(_),
            ) => {
                let openai_resp: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // Transform to Anthropic Messages format using the transformer
                let messages_resp: MessagesResponse = openai_resp.try_into().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Transformation error: {}", e),
                    )
                })?;
                Ok(ProviderResponseType::MessagesResponse(messages_resp))
            }
            // Amazon Bedrock transformations
            (
                SupportedUpstreamAPIs::AmazonBedrockConverse(_),
                SupportedAPIs::OpenAIChatCompletions(_),
            ) => {
                let bedrock_resp: ConverseResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // Transform to OpenAI ChatCompletions format using the transformer
                let chat_resp: ChatCompletionsResponse = bedrock_resp.try_into().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Transformation error: {}", e),
                    )
                })?;
                Ok(ProviderResponseType::ChatCompletionsResponse(chat_resp))
            }
            (
                SupportedUpstreamAPIs::AmazonBedrockConverse(_),
                SupportedAPIs::AnthropicMessagesAPI(_),
            ) => {
                let bedrock_resp: ConverseResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // Transform to Anthropic Messages format using the transformer
                let messages_resp: MessagesResponse = bedrock_resp.try_into().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Transformation error: {}", e),
                    )
                })?;
                Ok(ProviderResponseType::MessagesResponse(messages_resp))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported API combination for response transformation",
            )),
        }
    }
}

// Stream response transformation logic for client API compatibility
impl TryFrom<(&[u8], &SupportedAPIs, &SupportedUpstreamAPIs)> for ProviderStreamResponseType {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(
        (bytes, client_api, upstream_api): (&[u8], &SupportedAPIs, &SupportedUpstreamAPIs),
    ) -> Result<Self, Self::Error> {
        // Special case: Handle [DONE] marker for OpenAI -> Anthropic conversion
        if bytes == b"[DONE]" && matches!(client_api, SupportedAPIs::AnthropicMessagesAPI(_)) {
            return Ok(ProviderStreamResponseType::MessagesStreamEvent(
                crate::apis::anthropic::MessagesStreamEvent::MessageStop,
            ));
        }
        match (upstream_api, client_api) {
            // OpenAI upstream
            (
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
                SupportedAPIs::OpenAIChatCompletions(_),
            ) => {
                let resp = serde_json::from_slice(bytes)?;
                Ok(ProviderStreamResponseType::ChatCompletionsStreamResponse(
                    resp,
                ))
            }
            (
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
                SupportedAPIs::AnthropicMessagesAPI(_),
            ) => {
                let openai_resp: crate::apis::openai::ChatCompletionsStreamResponse =
                    serde_json::from_slice(bytes)?;
                let anthropic_resp = openai_resp.try_into()?;
                Ok(ProviderStreamResponseType::MessagesStreamEvent(
                    anthropic_resp,
                ))
            }

            // Anthropic upstream
            (
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
                SupportedAPIs::AnthropicMessagesAPI(_),
            ) => {
                let resp = serde_json::from_slice(bytes)?;
                Ok(ProviderStreamResponseType::MessagesStreamEvent(resp))
            }
            (
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
                SupportedAPIs::OpenAIChatCompletions(_),
            ) => {
                let anthropic_resp: crate::apis::anthropic::MessagesStreamEvent =
                    serde_json::from_slice(bytes)?;
                let openai_resp = anthropic_resp.try_into()?;
                Ok(ProviderStreamResponseType::ChatCompletionsStreamResponse(
                    openai_resp,
                ))
            }

            // Amazon Bedrock ConverseStream upstream
            (
                SupportedUpstreamAPIs::AmazonBedrockConverseStream(_),
                SupportedAPIs::AnthropicMessagesAPI(_),
            ) => {
                let bedrock_resp: crate::apis::amazon_bedrock::ConverseStreamEvent =
                    serde_json::from_slice(bytes)?;
                let anthropic_resp = bedrock_resp.try_into()?;
                Ok(ProviderStreamResponseType::MessagesStreamEvent(
                    anthropic_resp,
                ))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported API combination for response transformation",
            )
            .into()),
        }
    }
}

// TryFrom implementation to convert raw bytes to SseEvent with parsed provider response
impl TryFrom<(SseEvent, &SupportedAPIs, &SupportedUpstreamAPIs)> for SseEvent {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(
        (sse_event, client_api, upstream_api): (SseEvent, &SupportedAPIs, &SupportedUpstreamAPIs),
    ) -> Result<Self, Self::Error> {
        // Create a new transformed event based on the original
        let mut transformed_event = sse_event;

        // If has data, parse the data as a provider stream response (business logic layer)
        if transformed_event.data.is_some() {
            let data_str = transformed_event.data.as_ref().unwrap();
            let data_bytes = data_str.as_bytes();
            let transformed_response: ProviderStreamResponseType =
                ProviderStreamResponseType::try_from((data_bytes, client_api, upstream_api))?;

            // Convert to SSE string explicitly to avoid type ambiguity
            let sse_string: String = transformed_response.clone().into();
            transformed_event.sse_transform_buffer = sse_string;
            transformed_event.provider_stream_response = Some(transformed_response);
        }

        match (client_api, upstream_api) {
            (
                SupportedAPIs::AnthropicMessagesAPI(_),
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
            ) => {
                if let Some(provider_response) = &transformed_event.provider_stream_response {
                    if let Some(event_type) = provider_response.event_type() {
                        // This ensures the required Anthropic sequence: MessageStart → ContentBlockStart → ContentBlockDelta(s)
                        if event_type == "message_start" {
                            // Create ContentBlockStart event and format it using Into<String>
                            let content_block_start = MessagesStreamEvent::ContentBlockStart {
                                index: 0,
                                content_block: crate::apis::anthropic::MessagesContentBlock::Text {
                                    text: String::new(),
                                    cache_control: None,
                                },
                            };
                            let content_block_start_sse: String = content_block_start.into();

                            // Format as proper SSE: MessageStart first, then ContentBlockStart
                            // The sse_transform_buffer already contains the properly formatted MessageStart
                            transformed_event.sse_transform_buffer = format!(
                                "{}{}",
                                transformed_event.sse_transform_buffer, content_block_start_sse,
                            );
                        } else if event_type == "message_delta" {
                            // Create ContentBlockStop event and format it using Into<String>
                            let content_block_stop =
                                MessagesStreamEvent::ContentBlockStop { index: 0 };
                            let content_block_stop_sse: String = content_block_stop.into();

                            // Format as proper SSE: ContentBlockStop first, then MessageDelta
                            transformed_event.sse_transform_buffer = format!(
                                "{}{}",
                                content_block_stop_sse, transformed_event.sse_transform_buffer
                            );
                        }
                        // For other event types, the sse_transform_buffer already has the correct format from Into<String>
                    }
                    // If event_type is None, we just keep the data line as-is without an event line
                    // This handles cases where the transformation might not produce a valid event type
                }
            }
            (
                SupportedAPIs::OpenAIChatCompletions(_),
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
            ) => {
                if transformed_event.is_event_only() && transformed_event.event.is_some() {
                    transformed_event.sse_transform_buffer = format!("\n"); // suppress the event upstream for OpenAI
                }
            }
            _ => {
                // Other combinations can be handled here as needed
            }
        }

        Ok(transformed_event)
    }
}

// TryFrom implementation to convert AWS Event Stream DecodedFrame to ProviderStreamResponseType
impl
    TryFrom<(
        &aws_smithy_eventstream::frame::DecodedFrame,
        &SupportedAPIs,
        &SupportedUpstreamAPIs,
    )> for ProviderStreamResponseType
{
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(
        (frame, client_api, upstream_api): (
            &aws_smithy_eventstream::frame::DecodedFrame,
            &SupportedAPIs,
            &SupportedUpstreamAPIs,
        ),
    ) -> Result<Self, Self::Error> {
        use aws_smithy_eventstream::frame::DecodedFrame;

        match frame {
            DecodedFrame::Complete(_) => {
                // We have a complete frame - parse it based on upstream API
                match (upstream_api, client_api) {
                    (
                        SupportedUpstreamAPIs::AmazonBedrockConverseStream(_),
                        SupportedAPIs::AnthropicMessagesAPI(_),
                    ) => {
                        // Parse the DecodedFrame into ConverseStreamEvent
                        let bedrock_event =
                            crate::apis::amazon_bedrock::ConverseStreamEvent::try_from(frame)?;
                        let anthropic_event: crate::apis::anthropic::MessagesStreamEvent =
                            bedrock_event.try_into()?;

                        Ok(ProviderStreamResponseType::MessagesStreamEvent(
                            anthropic_event,
                        ))
                    }
                    (
                        SupportedUpstreamAPIs::AmazonBedrockConverseStream(_),
                        SupportedAPIs::OpenAIChatCompletions(_),
                    ) => {
                        // Parse the DecodedFrame into ConverseStreamEvent
                        let bedrock_event =
                            crate::apis::amazon_bedrock::ConverseStreamEvent::try_from(frame)?;
                        let openai_event: crate::apis::openai::ChatCompletionsStreamResponse =
                            bedrock_event.try_into()?;
                        Ok(ProviderStreamResponseType::ChatCompletionsStreamResponse(
                            openai_event,
                        ))
                    }
                    _ => Err("Unsupported API combination for event-stream decoding".into()),
                }
            }
            DecodedFrame::Incomplete => {
                Err("Cannot convert incomplete frame to provider response".into())
            }
        }
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
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::amazon_bedrock_binary_frame::BedrockBinaryFrameDecoder;
    use crate::apis::anthropic::AnthropicApi;
    use crate::apis::openai::OpenAIApi;
    use crate::apis::sse::SseStreamIter;
    use crate::clients::endpoints::SupportedAPIs;
    use crate::providers::id::ProviderId;
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
        let result = ProviderResponseType::try_from((
            bytes.as_slice(),
            &SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions),
            &ProviderId::OpenAI,
        ));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::ChatCompletionsResponse(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.choices.len(), 1);
            }
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
        let result = ProviderResponseType::try_from((
            bytes.as_slice(),
            &SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages),
            &ProviderId::Anthropic,
        ));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::MessagesResponse(r) => {
                assert_eq!(r.model, "claude-3-sonnet-20240229");
                assert_eq!(r.content.len(), 1);
            }
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
        let result = ProviderResponseType::try_from((
            bytes.as_slice(),
            &SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages),
            &ProviderId::OpenAI,
        ));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::MessagesResponse(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.usage.input_tokens, 10);
                assert_eq!(r.usage.output_tokens, 25);
            }
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
        let result = ProviderResponseType::try_from((
            bytes.as_slice(),
            &SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions),
            &ProviderId::Anthropic,
        ));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::ChatCompletionsResponse(r) => {
                assert_eq!(r.model, "claude-3-sonnet-20240229");
                assert_eq!(r.usage.prompt_tokens, 10);
                assert_eq!(r.usage.completion_tokens, 25);
            }
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
        assert_eq!(
            event.data,
            Some("{\"id\":\"test\",\"object\":\"chat.completion.chunk\"}\n\n".to_string())
        );

        // Test conversion back to line using Display trait
        let wire_format = event.to_string();
        assert_eq!(
            wire_format,
            "data: {\"id\":\"test\",\"object\":\"chat.completion.chunk\"}\n\n"
        );

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

        "#
            .to_string(),
            sse_transform_buffer: r#"data: {"id":"test","object":"chat.completion.chunk"}

        "#
            .to_string(),
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
            sse_transform_buffer: r#"data: {"id": "test", "object": "chat.completion.chunk"}"#
                .to_string(),
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
            "data: [DONE]".to_string(),               // This should end the stream
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
        use crate::apis::anthropic::{MessagesContentDelta, MessagesStreamEvent};
        use crate::apis::openai::ChatCompletionsStreamResponse;

        // Test Anthropic event type
        let anthropic_event = MessagesStreamEvent::ContentBlockDelta {
            index: 0,
            delta: MessagesContentDelta::TextDelta {
                text: "Hello".to_string(),
            },
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
        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(
            crate::apis::openai::OpenAIApi::ChatCompletions,
        );

        let result = ProviderStreamResponseType::try_from((
            done_bytes.as_slice(),
            &client_api,
            &upstream_api,
        ));
        assert!(result.is_ok());

        if let Ok(ProviderStreamResponseType::MessagesStreamEvent(event)) = result {
            // Verify it's a MessageStop event
            assert_eq!(event.event_type(), Some("message_stop"));
            assert!(matches!(
                event,
                crate::apis::anthropic::MessagesStreamEvent::MessageStop
            ));
        } else {
            panic!("Expected MessagesStreamEvent::MessageStop");
        }
    }

    #[test]
    fn test_bedrock_event_stream_decoder_basic() {
        use bytes::BytesMut;

        // Create a simple test with minimal data
        let mut buffer = BytesMut::new();

        // Add some arbitrary bytes (not a real event-stream frame, just for testing the decoder)
        buffer.extend_from_slice(b"test data");

        let mut decoder = BedrockBinaryFrameDecoder::new(&mut buffer);

        // The decoder should return Incomplete for incomplete/invalid data
        // This signals the caller to wait for more data
        let result = decoder.decode_frame();
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            aws_smithy_eventstream::frame::DecodedFrame::Incomplete
        ));

        // Verify we can still access the buffer
        assert!(decoder.has_remaining());
    }

    #[test]
    fn test_bedrock_event_stream_decoder_with_real_frames() {
        use bytes::BytesMut;
        use std::fs;
        use std::path::PathBuf;

        // Read the actual response.hex file from tests/e2e directory
        let test_file =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/e2e/response.hex");

        // Only run this test if the file exists
        if !test_file.exists() {
            println!("Skipping test - response.hex not found");
            return;
        }

        let response_data = fs::read(&test_file).unwrap();
        let mut buffer = BytesMut::from(&response_data[..]);

        let mut decoder = BedrockBinaryFrameDecoder::new(&mut buffer);
        let mut frame_count = 0;

        // Decode all frames
        loop {
            match decoder.decode_frame() {
                Some(aws_smithy_eventstream::frame::DecodedFrame::Complete(message)) => {
                    frame_count += 1;

                    // Verify we can access headers
                    let event_type = message
                        .headers()
                        .iter()
                        .find(|h| h.name().as_str() == ":event-type")
                        .and_then(|h| h.value().as_string().ok());

                    assert!(event_type.is_some(), "Frame should have :event-type header");
                }
                Some(aws_smithy_eventstream::frame::DecodedFrame::Incomplete) => {
                    // End of buffer, no more complete frames available
                    break;
                }
                None => {
                    // Decode error
                    panic!("Decode error encountered");
                }
            }
        }

        // We should have decoded multiple frames
        assert!(frame_count > 0, "Should have decoded at least one frame");
    }

    #[test]
    fn test_bedrock_event_stream_decoder_chunked_data() {
        use bytes::BytesMut;
        use std::fs;
        use std::path::PathBuf;

        // Read the actual response.hex file
        let test_file =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/e2e/response.hex");

        if !test_file.exists() {
            println!("Skipping test - response.hex not found");
            return;
        }

        let response_data = fs::read(&test_file).unwrap();

        // Simulate chunked network arrivals with realistic chunk sizes
        // Using varying chunk sizes to test partial frame handling
        let mut buffer = BytesMut::new();
        let chunk_size_pattern = vec![500, 1000, 750, 1200, 800, 1500];
        let mut offset = 0;
        let mut total_frames = 0;
        let mut chunk_num = 0;

        // CRITICAL: Create ONE decoder and reuse it across chunks
        // The MessageFrameDecoder maintains state about partial frames
        let mut decoder = BedrockBinaryFrameDecoder::new(&mut buffer);

        // Process all data in chunks
        while offset < response_data.len() {
            let chunk_size = chunk_size_pattern[chunk_num % chunk_size_pattern.len()];
            chunk_num += 1;

            let end = (offset + chunk_size).min(response_data.len());
            let chunk = &response_data[offset..end];

            // Add new data to the buffer (accessing via buffer_mut())
            decoder.buffer_mut().extend_from_slice(chunk);
            offset = end;

            // Process all available complete frames from this chunk
            loop {
                match decoder.decode_frame() {
                    Some(aws_smithy_eventstream::frame::DecodedFrame::Complete(_)) => {
                        total_frames += 1;
                    }
                    Some(aws_smithy_eventstream::frame::DecodedFrame::Incomplete) => {
                        // Need more data - wait for next chunk
                        break;
                    }
                    None => {
                        // Decode error
                        panic!("Decode error in chunked test");
                    }
                }
            }
        }

        assert!(
            total_frames > 0,
            "Should have decoded frames from chunked data"
        );
    }

    #[test]
    fn test_bedrock_decoded_frame_to_provider_response() {
        test_bedrock_conversion(false);
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --nocapture
    fn test_bedrock_decoded_frame_to_provider_response_verbose() {
        test_bedrock_conversion(true);
    }

    #[test]
    fn test_bedrock_decoded_frame_with_tool_use() {
        test_bedrock_conversion_with_tools(false);
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --nocapture
    fn test_bedrock_decoded_frame_with_tool_use_verbose() {
        test_bedrock_conversion_with_tools(true);
    }

    fn test_bedrock_conversion(verbose: bool) {
        use bytes::BytesMut;
        use std::fs;
        use std::path::PathBuf;

        // Read the actual response.hex file from tests/e2e directory
        let test_file =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/e2e/response.hex");

        // Only run this test if the file exists
        if !test_file.exists() {
            println!("Skipping test - response.hex not found");
            return;
        }

        let response_data = fs::read(&test_file).unwrap();
        let mut buffer = BytesMut::from(&response_data[..]);

        let mut decoder = BedrockBinaryFrameDecoder::new(&mut buffer);

        let client_api =
            SupportedAPIs::AnthropicMessagesAPI(crate::apis::anthropic::AnthropicApi::Messages);
        let upstream_api = SupportedUpstreamAPIs::AmazonBedrockConverseStream(
            crate::apis::amazon_bedrock::AmazonBedrockApi::ConverseStream,
        );

        let mut conversion_count = 0;
        let mut message_start_seen = false;

        // Decode and convert frames
        loop {
            match decoder.decode_frame() {
                Some(frame @ aws_smithy_eventstream::frame::DecodedFrame::Complete(_)) => {
                    // Convert DecodedFrame to ProviderStreamResponseType
                    let result =
                        ProviderStreamResponseType::try_from((&frame, &client_api, &upstream_api));

                    match result {
                        Ok(provider_response) => {
                            conversion_count += 1;

                            // Verify we got a MessagesStreamEvent
                            assert!(matches!(
                                provider_response,
                                ProviderStreamResponseType::MessagesStreamEvent(_)
                            ));

                            if verbose {
                                // Print the SSE string output
                                let sse_string: String = provider_response.clone().into();
                                println!("{}", sse_string);
                            }

                            // Check for MessageStart event
                            if let ProviderStreamResponseType::MessagesStreamEvent(ref event) =
                                provider_response
                            {
                                if matches!(
                                    event,
                                    crate::apis::anthropic::MessagesStreamEvent::MessageStart { .. }
                                ) {
                                    message_start_seen = true;
                                }
                            }
                        }
                        Err(e) => {
                            println!("Conversion error (frame {}): {}", conversion_count, e);
                        }
                    }
                }
                Some(aws_smithy_eventstream::frame::DecodedFrame::Incomplete) => {
                    // End of buffer
                    break;
                }
                None => {
                    panic!("Decode error");
                }
            }
        }

        assert!(
            conversion_count > 0,
            "Should have converted at least one frame"
        );
        assert!(message_start_seen, "Should have seen MessageStart event");
    }

    fn test_bedrock_conversion_with_tools(verbose: bool) {
        use bytes::BytesMut;
        use std::fs;
        use std::path::PathBuf;

        // Read the actual response_with_tools.hex file from tests/e2e directory
        let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/e2e/response_with_tools.hex");

        // Only run this test if the file exists
        if !test_file.exists() {
            println!("Skipping test - response_with_tools.hex not found");
            return;
        }

        let response_data = fs::read(&test_file).unwrap();
        let mut buffer = BytesMut::from(&response_data[..]);

        let mut decoder = BedrockBinaryFrameDecoder::new(&mut buffer);

        let client_api =
            SupportedAPIs::AnthropicMessagesAPI(crate::apis::anthropic::AnthropicApi::Messages);
        let upstream_api = SupportedUpstreamAPIs::AmazonBedrockConverseStream(
            crate::apis::amazon_bedrock::AmazonBedrockApi::ConverseStream,
        );

        let mut conversion_count = 0;
        let mut message_start_seen = false;
        let mut content_block_start_seen = false;
        let mut content_block_delta_tool_use_seen = false;

        // Decode and convert frames
        loop {
            match decoder.decode_frame() {
                Some(frame @ aws_smithy_eventstream::frame::DecodedFrame::Complete(_)) => {
                    // Convert DecodedFrame to ProviderStreamResponseType
                    let result =
                        ProviderStreamResponseType::try_from((&frame, &client_api, &upstream_api));

                    match result {
                        Ok(provider_response) => {
                            conversion_count += 1;

                            // Verify we got a MessagesStreamEvent
                            assert!(matches!(
                                provider_response,
                                ProviderStreamResponseType::MessagesStreamEvent(_)
                            ));

                            if verbose {
                                // Print the SSE string output
                                let sse_string: String = provider_response.clone().into();
                                println!("{}", sse_string);
                            }

                            // Check for specific events related to tool use
                            if let ProviderStreamResponseType::MessagesStreamEvent(ref event) =
                                provider_response
                            {
                                match event {
                                    crate::apis::anthropic::MessagesStreamEvent::MessageStart { .. } => {
                                        message_start_seen = true;
                                    }
                                    crate::apis::anthropic::MessagesStreamEvent::ContentBlockStart { .. } => {
                                        content_block_start_seen = true;
                                    }
                                    crate::apis::anthropic::MessagesStreamEvent::ContentBlockDelta { delta, .. } => {
                                        if matches!(delta, crate::apis::anthropic::MessagesContentDelta::InputJsonDelta { .. }) {
                                            content_block_delta_tool_use_seen = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => {
                            println!("Conversion error (frame {}): {}", conversion_count, e);
                        }
                    }
                }
                Some(aws_smithy_eventstream::frame::DecodedFrame::Incomplete) => {
                    // End of buffer
                    break;
                }
                None => {
                    panic!("Decode error");
                }
            }
        }

        assert!(
            conversion_count > 0,
            "Should have converted at least one frame"
        );
        assert!(message_start_seen, "Should have seen MessageStart event");
        assert!(
            content_block_start_seen,
            "Should have seen ContentBlockStart event for tool use"
        );
        assert!(
            content_block_delta_tool_use_seen,
            "Should have seen ContentBlockDelta with ToolUseDelta"
        );
    }

    #[test]
    fn test_sse_event_transformation_openai_to_anthropic_message_start() {
        use crate::apis::anthropic::AnthropicApi;
        use crate::apis::openai::OpenAIApi;

        // Create an OpenAI stream response that represents a role start (which becomes message_start in Anthropic)
        let openai_stream_chunk = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"role": "assistant"},
                "finish_reason": null
            }]
        });

        // Create SSE event with this data
        let sse_event = SseEvent {
            data: Some(openai_stream_chunk.to_string()),
            event: None,
            raw_line: format!("data: {}", openai_stream_chunk.to_string()),
            sse_transform_buffer: format!("data: {}", openai_stream_chunk.to_string()),
            provider_stream_response: None,
        };

        let client_api = SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);
        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Transform the event
        let result = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        assert!(result.is_ok());

        let transformed = result.unwrap();

        // Verify the transformation includes both message_start and content_block_start
        let buffer = transformed.sse_transform_buffer;
        assert!(
            buffer.contains("event: message_start"),
            "Should contain message_start event"
        );
        assert!(
            buffer.contains("event: content_block_start"),
            "Should contain content_block_start event"
        );

        // Verify proper SSE format with event lines before data lines
        assert!(buffer.find("event: message_start").unwrap() < buffer.find("data:").unwrap());
        assert!(buffer.find("content_block_start").is_some());
    }

    #[test]
    fn test_sse_event_transformation_openai_to_anthropic_message_delta() {
        use crate::apis::anthropic::AnthropicApi;
        use crate::apis::openai::OpenAIApi;

        // Create an OpenAI stream response with finish_reason (which becomes message_delta in Anthropic)
        let openai_stream_chunk = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 25,
                "total_tokens": 35
            }
        });

        // Create SSE event with this data
        let sse_event = SseEvent {
            data: Some(openai_stream_chunk.to_string()),
            event: None,
            raw_line: format!("data: {}", openai_stream_chunk.to_string()),
            sse_transform_buffer: format!("data: {}", openai_stream_chunk.to_string()),
            provider_stream_response: None,
        };

        let client_api = SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);
        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Transform the event
        let result = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        assert!(result.is_ok());

        let transformed = result.unwrap();

        // Verify the transformation includes both content_block_stop and message_delta
        let buffer = transformed.sse_transform_buffer;
        assert!(
            buffer.contains("event: content_block_stop"),
            "Should contain content_block_stop event"
        );
        assert!(
            buffer.contains("event: message_delta"),
            "Should contain message_delta event"
        );

        // Verify content_block_stop comes before message_delta
        let stop_pos = buffer.find("content_block_stop").unwrap();
        let delta_pos = buffer.find("message_delta").unwrap();
        assert!(
            stop_pos < delta_pos,
            "content_block_stop should come before message_delta"
        );
    }

    #[test]
    fn test_sse_event_transformation_openai_to_anthropic_content_delta() {
        use crate::apis::anthropic::AnthropicApi;
        use crate::apis::openai::OpenAIApi;

        // Create an OpenAI stream response with content (which becomes content_block_delta in Anthropic)
        let openai_stream_chunk = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        });

        // Create SSE event with this data
        let sse_event = SseEvent {
            data: Some(openai_stream_chunk.to_string()),
            event: None,
            raw_line: format!("data: {}", openai_stream_chunk.to_string()),
            sse_transform_buffer: format!("data: {}", openai_stream_chunk.to_string()),
            provider_stream_response: None,
        };

        let client_api = SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);
        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Transform the event
        let result = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        assert!(result.is_ok());

        let transformed = result.unwrap();

        // Verify the transformation is a content_block_delta (no extra events injected)
        let buffer = transformed.sse_transform_buffer;
        assert!(
            buffer.contains("event: content_block_delta"),
            "Should contain content_block_delta event"
        );
        assert!(
            !buffer.contains("content_block_start"),
            "Should not inject content_block_start for content delta"
        );
        assert!(
            !buffer.contains("content_block_stop"),
            "Should not inject content_block_stop for content delta"
        );

        // Verify the content is preserved
        assert!(buffer.contains("Hello"), "Should preserve the content text");
    }

    #[test]
    fn test_sse_event_transformation_anthropic_to_openai_suppresses_event_lines() {
        use crate::apis::anthropic::AnthropicApi;
        use crate::apis::openai::OpenAIApi;

        // Create an Anthropic event-only SSE line (no data)
        let sse_event = SseEvent {
            data: None,
            event: Some("message_start".to_string()),
            raw_line: "event: message_start".to_string(),
            sse_transform_buffer: "event: message_start".to_string(),
            provider_stream_response: None,
        };

        let client_api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);
        let upstream_api = SupportedUpstreamAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);

        // Transform the event
        let result = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        assert!(result.is_ok());

        let transformed = result.unwrap();

        // Verify the event line is suppressed (replaced with just newline)
        assert_eq!(
            transformed.sse_transform_buffer, "\n",
            "Event-only lines should be suppressed to newline for OpenAI"
        );
        assert!(
            transformed.is_event_only(),
            "Should still be marked as event-only"
        );
    }

    #[test]
    fn test_sse_event_transformation_anthropic_to_openai_preserves_data() {
        use crate::apis::anthropic::AnthropicApi;
        use crate::apis::openai::OpenAIApi;

        // Create an Anthropic message_start event with data
        let anthropic_event = json!({
            "type": "message_start",
            "message": {
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": "claude-3-sonnet",
                "stop_reason": null,
                "usage": {"input_tokens": 10, "output_tokens": 0}
            }
        });

        let sse_event = SseEvent {
            data: Some(anthropic_event.to_string()),
            event: None,
            raw_line: format!("data: {}", anthropic_event.to_string()),
            sse_transform_buffer: format!("data: {}", anthropic_event.to_string()),
            provider_stream_response: None,
        };

        let client_api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);
        let upstream_api = SupportedUpstreamAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);

        // Transform the event
        let result = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        assert!(result.is_ok());

        let transformed = result.unwrap();

        // Verify data is transformed to OpenAI format
        let buffer = transformed.sse_transform_buffer;
        assert!(buffer.starts_with("data: "), "Should have data: prefix");
        assert!(
            !buffer.contains("event:"),
            "Should not have event: lines for OpenAI"
        );

        // Verify provider response was parsed
        assert!(transformed.provider_stream_response.is_some());
    }

    #[test]
    fn test_sse_event_transformation_no_change_for_matching_apis() {
        use crate::apis::openai::OpenAIApi;

        // Create an OpenAI stream response
        let openai_stream_chunk = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        });

        let original_data = openai_stream_chunk.to_string();
        let sse_event = SseEvent {
            data: Some(original_data.clone()),
            event: None,
            raw_line: format!("data: {}", original_data),
            sse_transform_buffer: format!("data: {}\n\n", original_data),
            provider_stream_response: None,
        };

        let client_api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);
        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Transform the event
        let result = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        assert!(result.is_ok());

        let transformed = result.unwrap();

        // Verify minimal transformation - just SSE formatting, no API conversion
        let buffer = transformed.sse_transform_buffer;
        assert!(buffer.starts_with("data: "), "Should preserve data: prefix");
        assert!(!buffer.contains("event:"), "Should not add event: lines");

        // Verify provider response was parsed
        assert!(transformed.provider_stream_response.is_some());
    }

    #[test]
    fn test_sse_event_transformation_preserves_provider_response() {
        use crate::apis::anthropic::AnthropicApi;
        use crate::apis::openai::OpenAIApi;

        // Create an OpenAI stream response
        let openai_stream_chunk = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"content": "Test"},
                "finish_reason": null
            }]
        });

        let sse_event = SseEvent {
            data: Some(openai_stream_chunk.to_string()),
            event: None,
            raw_line: format!("data: {}", openai_stream_chunk.to_string()),
            sse_transform_buffer: format!("data: {}", openai_stream_chunk.to_string()),
            provider_stream_response: None,
        };

        let client_api = SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);
        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Transform the event
        let result = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        assert!(result.is_ok());

        let transformed = result.unwrap();

        // Verify provider_stream_response is populated
        assert!(
            transformed.provider_stream_response.is_some(),
            "Should parse and store provider response"
        );

        // Verify we can access the provider response
        let provider_response = transformed.provider_response();
        assert!(
            provider_response.is_ok(),
            "Should be able to access provider response"
        );

        // Verify the content delta is accessible
        let content = provider_response.unwrap().content_delta();
        assert_eq!(content, Some("Test"), "Should preserve content delta");
    }
}
