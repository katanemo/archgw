//! Provider traits for generic request/response handling
//!
//! This module defines the core traits that enable provider-agnostic
//! handling of LLM requests and responses in the gateway.

use std::error::Error;
use crate::Provider;

/// Trait for provider-specific request types
pub trait ProviderRequest: Sized {
    type Error: Error + Send + Sync + 'static;

    /// Parse request from raw bytes
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Convert to provider-specific format
    fn to_provider_bytes(&self, provider: Provider) -> Result<Vec<u8>, Self::Error>;

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
    fn try_from_bytes(bytes: &[u8], provider: &Provider) -> Result<Self, Self::Error>;

    /// Get usage information if available
    fn usage(&self) -> Option<&Self::Usage>;
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
    fn try_from_bytes(bytes: &[u8], provider: &Provider) -> Result<Self, Self::Error>;
}

/// Main provider interface trait
pub trait ProviderInterface {
    type Request: ProviderRequest;
    type Response: ProviderResponse;
    type StreamingResponse: StreamingResponse;
    type Usage: TokenUsage;
}
