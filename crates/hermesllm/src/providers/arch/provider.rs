//! Arch provider implementation

use crate::providers::{ProviderInterface, ConversionMode};
use crate::apis::openai::{ChatCompletionsRequest, ChatCompletionsResponse, Usage};
use crate::providers::traits::{ProviderRequest, ProviderResponse, StreamingResponse};
use crate::providers::openai::provider::{OpenAIProvider, OpenAIStreamingResponse, OpenAIApiError};

/// Arch provider implementation
#[derive(Debug, Clone)]
pub struct ArchProvider;

// Trait implementations that delegate to OpenAI
impl ProviderRequest for ArchProvider {
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
}

impl ProviderResponse for ArchProvider {
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

impl StreamingResponse for ArchProvider {
    type Error = OpenAIApiError;
    type StreamingIter = OpenAIStreamingResponse;

    fn try_from_bytes(&self, bytes: &[u8], provider: &super::super::ProviderId, mode: ConversionMode) -> Result<Self::StreamingIter, Self::Error> {
        let openai_provider = OpenAIProvider;
        StreamingResponse::try_from_bytes(&openai_provider, bytes, provider, mode)
    }
}

impl ProviderInterface for ArchProvider {
    fn has_compatible_api(&self, api_path: &str) -> bool {
        matches!(api_path, "/v1/chat/completions")
    }

    fn supported_apis(&self) -> Vec<&'static str> {
        vec!["/v1/chat/completions"]
    }

    fn parse_request(&self, bytes: &[u8]) -> Result<ChatCompletionsRequest, Box<dyn std::error::Error + Send + Sync>> {
        match ProviderRequest::try_from_bytes(self, bytes) {
            Ok(req) => Ok(req),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn parse_response(&self, bytes: &[u8], provider_id: super::super::ProviderId, mode: ConversionMode) -> Result<ChatCompletionsResponse, Box<dyn std::error::Error + Send + Sync>> {
        match ProviderResponse::try_from_bytes(self, bytes, &provider_id, mode) {
            Ok(resp) => Ok(resp),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn request_to_bytes(&self, request: &ChatCompletionsRequest, provider_id: super::super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        match ProviderRequest::to_provider_bytes(self, request, provider_id, mode) {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(Box::new(e)),
        }
    }
}
