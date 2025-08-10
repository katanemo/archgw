//! OpenAI provider interface implementations

use crate::apis::openai::*;
use crate::providers::traits::*;

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
    fn has_compatible_api(&self, api_path: &str) -> bool {
        api_path == "/v1/chat/completions"
    }

    fn supported_apis(&self) -> Vec<&'static str> {
        vec!["/v1/chat/completions"]
    }
}

// Direct trait implementations on OpenAIProvider
impl ProviderRequest for OpenAIProvider {
    type Error = OpenAIApiError;

    fn try_from_bytes(&self, bytes: &[u8]) -> Result<ChatCompletionsRequest, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        Ok(serde_json::from_str(s)?)
    }

    fn to_provider_bytes(&self, request: &ChatCompletionsRequest, _provider: super::super::ProviderId, _mode: ConversionMode) -> Result<Vec<u8>, Self::Error> {
        Ok(serde_json::to_vec(request)?)
    }

    fn extract_model<'a>(&self, request: &'a ChatCompletionsRequest) -> &'a str {
        &request.model
    }

    fn is_streaming(&self, request: &ChatCompletionsRequest) -> bool {
        request.stream.unwrap_or_default()
    }

    fn set_streaming_options(&self, request: &mut ChatCompletionsRequest) {
        if request.stream_options.is_none() {
            request.stream_options = Some(StreamOptions {
                include_usage: Some(true),
            });
        }
    }

    fn extract_messages_text(&self, request: &ChatCompletionsRequest) -> String {
        request.messages
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

    fn extract_user_message(&self, request: &ChatCompletionsRequest) -> Option<String> {
        request.messages.last().and_then(|msg| {
            match &msg.content {
                MessageContent::Text(text) => Some(text.clone()),
                MessageContent::Parts(parts) => {
                    // Extract text from content parts, ignoring images
                    let text_parts: Vec<String> = parts
                        .iter()
                        .filter_map(|part| match part {
                            ContentPart::Text { text } => Some(text.clone()),
                            ContentPart::ImageUrl { .. } => None,
                        })
                        .collect();
                    if text_parts.is_empty() {
                        None
                    } else {
                        Some(text_parts.join(" "))
                    }
                }
            }
        })
    }
}

impl ProviderResponse for OpenAIProvider {
    type Error = OpenAIApiError;
    type Usage = Usage;

    fn try_from_bytes(&self, bytes: &[u8], _provider: &super::super::ProviderId, _mode: ConversionMode) -> Result<ChatCompletionsResponse, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        Ok(serde_json::from_str(s)?)
    }

    fn usage<'a>(&self, response: &'a ChatCompletionsResponse) -> Option<&'a Self::Usage> {
        Some(&response.usage)
    }

    fn extract_usage_counts(&self, response: &ChatCompletionsResponse) -> Option<(usize, usize, usize)> {
        Some((
            response.usage.prompt_tokens as usize,
            response.usage.completion_tokens as usize,
            response.usage.total_tokens as usize,
        ))
    }
}

impl StreamingResponse for OpenAIProvider {
    type Error = OpenAIApiError;
    type StreamingIter = OpenAIStreamingResponse;

    fn try_from_bytes(&self, bytes: &[u8], _provider: &super::super::ProviderId, _mode: ConversionMode) -> Result<Self::StreamingIter, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        Ok(OpenAIStreamingResponse::new(s.to_string()))
    }
}

// ============================================================================
// Trait Implementations for OpenAI Types (Keep for TokenUsage only)
// ============================================================================

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

impl StreamChunk for ChatCompletionsStreamResponse {
    type Usage = Usage;

    fn usage(&self) -> Option<&Self::Usage> {
        self.usage.as_ref()
    }
}

impl StreamingResponse for OpenAIStreamingResponse {
    type Error = OpenAIApiError;
    type StreamingIter = OpenAIStreamingResponse;

    fn try_from_bytes(&self, bytes: &[u8], _provider: &super::super::ProviderId, _mode: ConversionMode) -> Result<Self::StreamingIter, Self::Error> {
        let s = std::str::from_utf8(bytes)?;
        Ok(OpenAIStreamingResponse::new(s.to_string()))
    }
}
