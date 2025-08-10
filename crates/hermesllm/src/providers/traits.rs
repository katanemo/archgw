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
pub trait ProviderRequest: Sized {
    type Error: Error + Send + Sync + 'static;

    /// Parse request from raw bytes
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Convert to provider-specific format
    fn to_provider_bytes(&self, provider: super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Self::Error>;

    /// Extract the model name from the request
    fn extract_model(&self) -> &str;

    /// Check if this is a streaming request
    fn is_streaming(&self) -> bool;

    /// Set streaming options (e.g., include_usage)
    fn set_streaming_options(&mut self);

    /// Extract text content from messages for token counting
    fn extract_messages_text(&self) -> String;
}

/// Trait for token usage information
pub trait TokenUsage {
    fn completion_tokens(&self) -> usize;
    fn prompt_tokens(&self) -> usize;
    fn total_tokens(&self) -> usize;
}

/// Trait for provider-specific response types
pub trait ProviderResponse: Sized {
    type Error: Error + Send + Sync + 'static;
    type Usage: TokenUsage;

    /// Parse response from raw bytes
    fn try_from_bytes(bytes: &[u8], provider: &super::ProviderId, mode: ConversionMode) -> Result<Self, Self::Error>;

    /// Get usage information if available
    fn usage(&self) -> Option<&Self::Usage>;

    /// Extract token counts for metrics
    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        self.usage().map(|u| (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()))
    }
}

/// Helper trait for stream context integration
pub trait StreamContextHelpers: ProviderRequest {
    /// Get the model name for routing and metrics
    fn get_model_for_routing(&self) -> String {
        self.extract_model().to_string()
    }

    /// Get text for token counting and rate limiting
    fn get_text_for_tokenization(&self) -> String {
        self.extract_messages_text()
    }

    /// Prepare for streaming by setting appropriate options
    fn prepare_for_streaming(&mut self) {
        if self.is_streaming() {
            self.set_streaming_options();
        }
    }
}

/// Trait for streaming response chunks
pub trait StreamChunk {
    type Usage: TokenUsage;

    /// Get usage information if available
    fn usage(&self) -> Option<&Self::Usage>;
}

/// Trait for streaming response iterators
pub trait StreamingResponse: Iterator<Item = Result<Self::Chunk, Self::Error>> + Sized {
    type Error: Error + Send + Sync + 'static;
    type Chunk: StreamChunk;

    /// Parse streaming response from raw bytes
    fn try_from_bytes(bytes: &[u8], provider: &super::ProviderId, mode: ConversionMode) -> Result<Self, Self::Error>;
}

/// Main provider interface trait
pub trait ProviderInterface {
    /// Check if this provider has a compatible API with the client request
    fn has_compatible_api(&self, api_path: &str) -> bool;

    /// Get the interface implementation for this provider
    /// passthrough: if true, use provider-specific format; if false, use compatible format
    fn get_interface(&self, passthrough: bool) -> ConversionMode {
        if passthrough {
            ConversionMode::Passthrough
        } else {
            ConversionMode::Compatible
        }
    }

    /// Parse a request from raw bytes - returns concrete ChatCompletionsRequest
    fn parse_request(&self, bytes: &[u8]) -> Result<crate::apis::openai::ChatCompletionsRequest, Box<dyn std::error::Error + Send + Sync>>;

    /// Parse a response from raw bytes - returns concrete ChatCompletionsResponse
    fn parse_response(&self, bytes: &[u8], provider_id: super::ProviderId, mode: ConversionMode) -> Result<crate::apis::openai::ChatCompletionsResponse, Box<dyn std::error::Error + Send + Sync>>;

    /// Convert a request to bytes for sending to upstream API
    fn request_to_bytes(&self, request: &crate::apis::openai::ChatCompletionsRequest, provider_id: super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;

    /// Extract model name from request for routing (convenience method for stream_context)
    fn extract_model_from_request(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> String {
        use ProviderRequest;
        request.extract_model().to_string()
    }

    /// Check if request is streaming (convenience method for stream_context)
    fn is_request_streaming(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> bool {
        use ProviderRequest;
        request.is_streaming()
    }

    /// Prepare request for streaming (convenience method for stream_context)
    fn prepare_request_for_streaming(&self, request: &mut crate::apis::openai::ChatCompletionsRequest) {
        use ProviderRequest;
        if request.is_streaming() {
            request.set_streaming_options();
        }
    }

    /// Extract text for tokenization (convenience method for stream_context)
    fn extract_text_for_tokenization(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> String {
        use ProviderRequest;
        request.extract_messages_text()
    }

    /// Extract usage information from response (convenience method for stream_context)
    fn extract_usage_from_response(&self, response: &crate::apis::openai::ChatCompletionsResponse) -> Option<(usize, usize, usize)> {
        use ProviderResponse;
        response.extract_usage_counts()
    }

    /// Get supported API endpoints for this provider
    fn supported_apis(&self) -> Vec<&'static str>;
}
