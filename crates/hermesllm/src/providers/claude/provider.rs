//! Claude provider implementation
//!
//! TODO: Implement Claude-specific API format (/v1/messages) when needed
//! For now, uses OpenAI-compatible format as fallback

use crate::providers::{ProviderInterface, ConversionMode};
use crate::apis::openai::{ChatCompletionsRequest, ChatCompletionsResponse, Usage};
use crate::providers::traits::{ProviderRequest, ProviderResponse, StreamingResponse};
use crate::providers::openai::provider::{OpenAIProvider, OpenAIStreamingResponse, OpenAIApiError};

/// Claude provider implementation
#[derive(Debug, Clone)]
pub struct ClaudeProvider;

// Trait implementations that delegate to OpenAI
impl ProviderRequest for ClaudeProvider {
    type Error = OpenAIApiError;

    fn try_from_bytes(&self, bytes: &[u8]) -> Result<ChatCompletionsRequest, Self::Error> {
        let openai_provider = OpenAIProvider;
        ProviderRequest::try_from_bytes(&openai_provider, bytes)
    }

    fn to_provider_bytes(&self, request: &ChatCompletionsRequest, provider: super::super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Self::Error> {
        let openai_provider = OpenAIProvider;
        ProviderRequest::to_provider_bytes(&openai_provider, request, provider, mode)
    }

    fn extract_model<'a>(&self, request: &'a ChatCompletionsRequest) -> &'a str {
        &request.model
    }

    fn is_streaming(&self, request: &ChatCompletionsRequest) -> bool {
        request.stream.unwrap_or_default()
    }

    fn set_streaming_options(&self, request: &mut ChatCompletionsRequest) {
        let openai_provider = OpenAIProvider;
        ProviderRequest::set_streaming_options(&openai_provider, request)
    }

    fn extract_messages_text(&self, request: &ChatCompletionsRequest) -> String {
        let openai_provider = OpenAIProvider;
        ProviderRequest::extract_messages_text(&openai_provider, request)
    }

    fn extract_user_message(&self, request: &ChatCompletionsRequest) -> Option<String> {
        let openai_provider = OpenAIProvider;
        ProviderRequest::extract_user_message(&openai_provider, request)
    }
}

impl ProviderResponse for ClaudeProvider {
    type Error = OpenAIApiError;
    type Usage = Usage;

    fn try_from_bytes(&self, bytes: &[u8], provider: &super::super::ProviderId, mode: ConversionMode) -> Result<ChatCompletionsResponse, Self::Error> {
        let openai_provider = OpenAIProvider;
        ProviderResponse::try_from_bytes(&openai_provider, bytes, provider, mode)
    }

    fn usage<'a>(&self, response: &'a ChatCompletionsResponse) -> Option<&'a Self::Usage> {
        Some(&response.usage)
    }

    fn extract_usage_counts(&self, response: &ChatCompletionsResponse) -> Option<(usize, usize, usize)> {
        let openai_provider = OpenAIProvider;
        ProviderResponse::extract_usage_counts(&openai_provider, response)
    }
}

impl StreamingResponse for ClaudeProvider {
    type Error = OpenAIApiError;
    type StreamChunk = crate::apis::openai::ChatCompletionsStreamResponse;
    type StreamingIter = OpenAIStreamingResponse;

    fn try_from_bytes(&self, bytes: &[u8], provider: &super::super::ProviderId, mode: ConversionMode) -> Result<Self::StreamingIter, Self::Error> {
        let openai_provider = OpenAIProvider;
        StreamingResponse::try_from_bytes(&openai_provider, bytes, provider, mode)
    }
}

impl ProviderInterface for ClaudeProvider {
    fn has_compatible_api(&self, api_path: &str) -> bool {
        // TODO: Update when Claude API is fully implemented
        matches!(api_path, "/v1/chat/completions" | "/v1/messages")
    }

    fn supported_apis(&self) -> Vec<&'static str> {
        // TODO: Update when Claude API is fully implemented
        vec!["/v1/messages"]
    }
}
