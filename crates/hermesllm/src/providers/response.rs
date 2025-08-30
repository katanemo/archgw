use crate::providers::id::ProviderId;
use serde::Serialize;
use std::error::Error;
use std::fmt;
use std::convert::TryFrom;

use crate::apis::openai::ChatCompletionsResponse;
use crate::apis::OpenAISseIter;
use crate::clients::endpoints::SupportedAPIs;
use crate::apis::anthropic::AnthropicSseIter;
use crate::apis::anthropic::MessagesResponse;

#[derive(Serialize)]
#[serde(untagged)]
pub enum ProviderResponseType {
    ChatCompletionsResponse(ChatCompletionsResponse),
    MessagesResponse(MessagesResponse),
}



pub enum ProviderStreamResponseIter {
    ChatCompletionsStream(OpenAISseIter<std::vec::IntoIter<String>>),
    MessagesStream(AnthropicSseIter<std::vec::IntoIter<String>>),
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
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let resp: MessagesResponse = serde_json::from_slice(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::MessagesResponse(resp))
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let resp: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(resp))
            }
        }
    }
}

impl TryFrom<(&[u8], &SupportedAPIs, &ProviderId)> for ProviderStreamResponseIter {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((bytes, client_api, provider_id): (&[u8], &SupportedAPIs, &ProviderId)) -> Result<Self, Self::Error> {
        let upstream_api = provider_id.compatible_api_for_client(client_api);
        match (&upstream_api, client_api) {
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let s = std::str::from_utf8(bytes)?;
                let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
                let sse_container = crate::providers::response::SseStreamIter::new(lines.into_iter());
                let iter = crate::apis::openai::OpenAISseIter::new(sse_container);
                Ok(ProviderStreamResponseIter::ChatCompletionsStream(iter))
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let s = std::str::from_utf8(bytes)?;
                let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
                let sse_container = crate::providers::response::SseStreamIter::new(lines.into_iter());
                let iter = crate::apis::anthropic::AnthropicSseIter::new(sse_container);
                Ok(ProviderStreamResponseIter::MessagesStream(iter))
            }
            (SupportedAPIs::OpenAIChatCompletions(_), SupportedAPIs::AnthropicMessagesAPI(_)) => {
                let s = std::str::from_utf8(bytes)?;
                let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
                let sse_container = crate::providers::response::SseStreamIter::new(lines.into_iter());
                let iter = crate::apis::anthropic::AnthropicSseIter::new(sse_container);
                Ok(ProviderStreamResponseIter::MessagesStream(iter))
            }
            (SupportedAPIs::AnthropicMessagesAPI(_), SupportedAPIs::OpenAIChatCompletions(_)) => {
                let s = std::str::from_utf8(bytes)?;
                let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
                let sse_container = crate::providers::response::SseStreamIter::new(lines.into_iter());
                let iter = crate::apis::openai::OpenAISseIter::new(sse_container);
                Ok(ProviderStreamResponseIter::ChatCompletionsStream(iter))
            }
        }
    }
}

impl Iterator for ProviderStreamResponseIter {
    type Item = Result<Box<dyn ProviderStreamResponse>, Box<dyn std::error::Error + Send + Sync>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ProviderStreamResponseIter::ChatCompletionsStream(iter) => iter.next(),
            ProviderStreamResponseIter::MessagesStream(iter) => iter.next(),
        }
    }
}
pub trait ProviderResponse: Send + Sync {
    /// Get usage information if available - returns dynamic trait object
    fn usage(&self) -> Option<&dyn TokenUsage>;

    /// Extract token counts for metrics
    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        self.usage().map(|u| (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()))
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
// GENERIC SSE STREAMING ITERATOR (Container Only)
// ============================================================================

/// Generic SSE (Server-Sent Events) streaming iterator container
/// This is just a simple wrapper - actual Iterator implementation is delegated to provider-specific modules
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

// Implement Send + Sync for the enum to match the original trait requirements
unsafe impl Send for ProviderStreamResponseIter {}
unsafe impl Sync for ProviderStreamResponseIter {}

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
        let result = ProviderResponseType::try_from((bytes.as_slice(), &SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages), &ProviderId::Claude));
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
        // Simulate Anthropic response with OpenAI provider (should parse as MessagesResponse)
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
        let result = ProviderResponseType::try_from((bytes.as_slice(), &SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages), &ProviderId::OpenAI));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::MessagesResponse(r) => {
                assert_eq!(r.model, "claude-3-sonnet-20240229");
            },
            _ => panic!("Expected MessagesResponse variant"),
        }
    }

    #[test]
    fn test_openai_response_from_bytes_with_claude_provider() {
        // Simulate OpenAI response with Claude provider (should parse as ChatCompletionsResponse)
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
        let result = ProviderResponseType::try_from((bytes.as_slice(), &SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions), &ProviderId::Claude));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderResponseType::ChatCompletionsResponse(r) => {
                assert_eq!(r.model, "gpt-4");
            },
            _ => panic!("Expected ChatCompletionsResponse variant"),
        }
    }
}
