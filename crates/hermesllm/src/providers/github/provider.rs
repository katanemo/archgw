//! GitHub provider implementation
//!
//! This module contains the GitHub provider that handles GitHub API format
//! requests in OpenAI-compatible format.

use crate::providers::{ProviderInterface, ConversionMode};
use crate::apis::openai::{ChatCompletionsRequest, ChatCompletionsResponse, Usage};
use crate::providers::traits::{ProviderRequest, ProviderResponse, StreamingResponse};
use crate::providers::openai::provider::{OpenAIProvider, OpenAIStreamingResponse, OpenAIApiError};

/// GitHub provider implementation
#[derive(Debug, Clone)]
pub struct GitHubProvider;

// Trait implementations that delegate to OpenAI
impl ProviderRequest for GitHubProvider {
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

impl ProviderResponse for GitHubProvider {
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

impl StreamingResponse for GitHubProvider {
    type Error = OpenAIApiError;
    type StreamingIter = OpenAIStreamingResponse;

    fn try_from_bytes(&self, bytes: &[u8], provider: &super::super::ProviderId, mode: ConversionMode) -> Result<Self::StreamingIter, Self::Error> {
        let openai_provider = OpenAIProvider;
        StreamingResponse::try_from_bytes(&openai_provider, bytes, provider, mode)
    }
}

impl ProviderInterface for GitHubProvider {
    fn has_compatible_api(&self, api_path: &str) -> bool {
        // TODO: Update when GitHub API is fully implemented
        matches!(api_path, "/v1/chat/completions" | "/models")
    }

    fn supported_apis(&self) -> Vec<&'static str> {
        // TODO: Update when GitHub API is fully implemented
        vec!["/models"]
    }
}
