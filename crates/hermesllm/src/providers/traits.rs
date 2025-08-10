//! Provider traits for generic request/response handling
//!
//! This module defines the core traits that enable provider-agnostic
//! handling of LLM requests and responses in the gateway.

use std::error::Error;

/// Conversion mode for provider requests/responses
#[derive(Debug, Clone, Copy)]
pub enum ConversionMode {
    /// Compatible: Convert between different provider formats to ensure compatibility
    Compatible,
    /// Passthrough: Pass requests/responses through with minimal modification
    Passthrough,
}

/// Trait for provider-specific request types
pub trait ProviderRequest {
    type Error: Error + Send + Sync + 'static;

    /// Parse request from raw bytes
    fn try_from_bytes(&self, bytes: &[u8]) -> Result<crate::apis::openai::ChatCompletionsRequest, Self::Error>;

    /// Convert to provider-specific format
    fn to_provider_bytes(&self, request: &crate::apis::openai::ChatCompletionsRequest, provider: super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Self::Error>;

    /// Extract the model name from the request
    fn extract_model<'a>(&self, request: &'a crate::apis::openai::ChatCompletionsRequest) -> &'a str;

    /// Check if this is a streaming request
    fn is_streaming(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> bool;

    /// Set streaming options (e.g., include_usage)
    fn set_streaming_options(&self, request: &mut crate::apis::openai::ChatCompletionsRequest);

    /// Extract text content from messages for token counting
    fn extract_messages_text(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> String;

    /// Extract the user message for tracing/logging purposes
    fn extract_user_message(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> Option<String>;
}

/// Trait for token usage information
pub trait TokenUsage {
    fn completion_tokens(&self) -> usize;
    fn prompt_tokens(&self) -> usize;
    fn total_tokens(&self) -> usize;
}

/// Trait for provider-specific response types
pub trait ProviderResponse {
    type Error: Error + Send + Sync + 'static;
    type Usage: TokenUsage;

    /// Parse response from raw bytes
    fn try_from_bytes(&self, bytes: &[u8], provider: &super::ProviderId, mode: ConversionMode) -> Result<crate::apis::openai::ChatCompletionsResponse, Self::Error>;

    /// Get usage information if available
    fn usage<'a>(&self, response: &'a crate::apis::openai::ChatCompletionsResponse) -> Option<&'a Self::Usage>;

    /// Extract token counts for metrics
    fn extract_usage_counts(&self, response: &crate::apis::openai::ChatCompletionsResponse) -> Option<(usize, usize, usize)> {
        self.usage(response).map(|u| (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()))
    }
}

/// Trait for streaming response chunks
pub trait StreamChunk {
    type Usage: TokenUsage;

    /// Get usage information if available
    fn usage(&self) -> Option<&Self::Usage>;
}

/// Trait for streaming response iterators
pub trait StreamingResponse {
    type Error: Error + Send + Sync + 'static;
    type StreamingIter: Iterator<Item = Result<crate::apis::openai::ChatCompletionsStreamResponse, Self::Error>>;

    /// Parse streaming response from raw bytes
    fn try_from_bytes(&self, bytes: &[u8], provider: &super::ProviderId, mode: ConversionMode) -> Result<Self::StreamingIter, Self::Error>;
}

/// Main provider interface trait - simplified to only essential methods
pub trait ProviderInterface: ProviderRequest + ProviderResponse + StreamingResponse {
    /// Check if this provider has a compatible API with the client request
    fn has_compatible_api(&self, api_path: &str) -> bool;

    /// Get supported API endpoints for this provider
    fn supported_apis(&self) -> Vec<&'static str>;
}
