//! OpenAI provider interface implementations

use crate::apis::openai::*;
use crate::providers::traits::*;
use crate::Provider;

// Simple error type for OpenAI API operations
#[derive(Debug, thiserror::Error)]
pub enum OpenAIApiError {
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("UTF-8 parsing error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("Invalid streaming data: {0}")]
    InvalidStreamingData(String),
    #[error("Request conversion error: {0}")]
    RequestConversionError(String),
}

// ============================================================================
// OpenAI Provider Definition
// ============================================================================

pub struct OpenAIProvider;

// Create a concrete streaming response type to avoid lifetime issues
pub struct OpenAIStreamingResponse {
    lines: Vec<String>,
    current_index: usize,
}

impl OpenAIStreamingResponse {
    fn new(data: String) -> Self {
        let lines: Vec<String> = data.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            current_index: 0,
        }
    }
}

impl Iterator for OpenAIStreamingResponse {
    type Item = Result<ChatCompletionsStreamResponse, OpenAIApiError>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_index < self.lines.len() {
            let line = &self.lines[self.current_index];
            self.current_index += 1;

            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data == "[DONE]" {
                    return None;
                }

                if data == r#"{"type": "ping"}"# {
                    continue; // Skip ping messages
                }

                return Some(
                    serde_json::from_str::<ChatCompletionsStreamResponse>(data).map_err(|e| {
                        OpenAIApiError::InvalidStreamingData(format!("Error parsing: {}, data: {}", e, data))
                    }),
                );
            }
        }
        None
    }
}

impl ProviderInterface for OpenAIProvider {
    type Request = ChatCompletionsRequest;
    type Response = ChatCompletionsResponse;
    type StreamingResponse = OpenAIStreamingResponse;
    type Usage = Usage;
}

// ============================================================================
// Trait Implementations for OpenAI Types
// ============================================================================

impl ProviderRequest for ChatCompletionsRequest {
    type Error = OpenAIApiError;

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        Ok(serde_json::from_str(s)?)
    }

    fn to_provider_bytes(&self, _provider: Provider) -> Result<Vec<u8>, Self::Error> {
        Ok(serde_json::to_vec(self)?)
    }

    fn extract_model(&self) -> &str {
        &self.model
    }

    fn is_streaming(&self) -> bool {
        self.stream.unwrap_or_default()
    }

    fn set_streaming_options(&mut self) {
        if self.stream_options.is_none() {
            self.stream_options = Some(StreamOptions {
                include_usage: Some(true),
            });
        }
    }

    fn extract_messages_text(&self) -> String {
        self.messages
            .iter()
            .fold(String::new(), |acc, m| {
                acc + " " + &match &m.content {
                    MessageContent::Text(text) => text.clone(),
                    MessageContent::Parts(parts) => {
                        parts.iter().map(|part| match part {
                            ContentPart::Text { text } => text.clone(),
                            ContentPart::ImageUrl { .. } => "[Image]".to_string(),
                        }).collect::<Vec<_>>().join(" ")
                    }
                }
            })
    }
}

impl TokenUsage for Usage {
    fn completion_tokens(&self) -> usize {
        self.completion_tokens as usize
    }

    fn prompt_tokens(&self) -> usize {
        self.prompt_tokens as usize
    }

    fn total_tokens(&self) -> usize {
        self.total_tokens as usize
    }
}

impl ProviderResponse for ChatCompletionsResponse {
    type Error = OpenAIApiError;
    type Usage = Usage;

    fn try_from_bytes(bytes: &[u8], _provider: &Provider) -> Result<Self, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        Ok(serde_json::from_str(s)?)
    }

    fn usage(&self) -> Option<&Self::Usage> {
        Some(&self.usage)
    }
}

impl StreamChunk for ChatCompletionsStreamResponse {
    type Usage = Usage;

    fn usage(&self) -> Option<&Self::Usage> {
        self.usage.as_ref()
    }
}

impl StreamingResponse for OpenAIStreamingResponse {
    type Error = OpenAIApiError;
    type Chunk = ChatCompletionsStreamResponse;

    fn try_from_bytes(bytes: &[u8], _provider: &Provider) -> Result<Self, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        Ok(OpenAIStreamingResponse::new(s.to_string()))
    }
}
