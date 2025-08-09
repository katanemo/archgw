//! Provider interface trait definitions
//!
//! This module defines the core traits that all LLM providers must implement.
//! The interface is designed around v1/chat/completions API for simplicity.

use std::error::Error;

/// Conversion mode for provider requests/responses
#[derive(Debug, Clone, Copy)]
pub enum ConversionMode {
    /// Compatible: Convert between different provider formats to ensure compatibility
    Compatible,
    /// Passthrough: Pass requests/responses through with minimal modification
    Passthrough,
}

/// Token usage information
pub trait TokenUsage {
    fn completion_tokens(&self) -> usize;
    fn prompt_tokens(&self) -> usize;
    fn total_tokens(&self) -> usize;
}

/// Error type for provider operations
pub trait ProviderError: Error + Send + Sync + 'static {}

/// Request type that can be converted to/from provider-specific formats
pub trait ProviderRequest: Sized {
    type Error: ProviderError;

    /// Parse request from raw bytes (typically JSON)
    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Convert to bytes for sending to upstream API
    fn to_bytes(&self, mode: ConversionMode) -> Result<Vec<u8>, Self::Error>;

    /// Extract the model name from the request
    fn model(&self) -> &str;

    /// Check if this is a streaming request
    fn is_streaming(&self) -> bool;

    /// Set streaming options (e.g., include_usage)
    fn set_streaming_options(&mut self);

    /// Extract text content from messages for token counting
    fn extract_text(&self) -> String;
}

/// Response type that can be converted to/from provider-specific formats
pub trait ProviderResponse: Sized {
    type Error: ProviderError;
    type Usage: TokenUsage;

    /// Parse response from raw bytes (typically JSON)
    fn from_bytes(bytes: &[u8], mode: ConversionMode) -> Result<Self, Self::Error>;

    /// Convert to bytes for sending to client
    fn to_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    /// Get usage information if available
    fn usage(&self) -> Option<&Self::Usage>;
}

/// Streaming response chunk
pub trait StreamChunk: Sized {
    type Error: ProviderError;
    type Usage: TokenUsage;

    /// Parse chunk from a line of streaming data
    fn from_line(line: &str, mode: ConversionMode) -> Result<Option<Self>, Self::Error>;

    /// Convert to line for sending to client
    fn to_line(&self) -> Result<String, Self::Error>;

    /// Get usage information if available (usually only in final chunk)
    fn usage(&self) -> Option<&Self::Usage>;

    /// Check if this is the final chunk in the stream
    fn is_final(&self) -> bool;
}

/// Main provider interface
pub trait LLMProvider {
    type Request: ProviderRequest;
    type Response: ProviderResponse;
    type StreamChunk: StreamChunk;
    type Error: ProviderError;

    /// Create a new instance of this provider
    fn new() -> Self;

    /// Get the supported API endpoints for this provider
    fn supported_apis(&self) -> Vec<&'static str>;

    /// Check if the provider supports v1/chat/completions API
    fn supports_chat_completions(&self) -> bool {
        self.supported_apis().contains(&"/v1/chat/completions")
    }

    /// Parse a request from raw bytes
    fn parse_request(&self, bytes: &[u8]) -> Result<Self::Request, Self::Error>;

    /// Parse a response from raw bytes
    fn parse_response(&self, bytes: &[u8], mode: ConversionMode) -> Result<Self::Response, Self::Error>;

    /// Parse streaming response chunks from raw data
    fn parse_stream_chunk(&self, line: &str, mode: ConversionMode) -> Result<Option<Self::StreamChunk>, Self::Error>;
}
