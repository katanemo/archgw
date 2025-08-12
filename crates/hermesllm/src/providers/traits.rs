//! Provider traits for generic request/response handling
//!
//! This module defines the core traits that enable provider-agnostic
//! handling of LLM requests and responses in the gateway.

use std::error::Error;
use std::fmt;

/// Trait for provider-specific request types
pub trait ProviderRequest: Send + Sync {
    /// Extract the model name from the request
    fn model(&self) -> &str;

    /// Set the model name for the request
    fn set_model(&mut self, model: String);

    /// Check if this is a streaming request
    fn is_streaming(&self) -> bool;

    /// Set streaming options (e.g., include_usage)
    fn set_streaming_options(&mut self);

    /// Extract text content from messages for token counting
    fn extract_messages_text(&self) -> String;

    /// Extract the user message for tracing/logging purposes
    fn extract_user_message(&self) -> Option<String>;

    /// Convert to provider-specific format
    fn to_provider_bytes(&self, mode: ConversionMode) -> Result<Vec<u8>, ProviderRequestError>;
}

/// Trait for provider-specific response types
pub trait ProviderResponse: Send + Sync {
    /// Get usage information if available - returns dynamic trait object
    fn usage(&self) -> Option<&dyn TokenUsage>;

    /// Extract token counts for metrics
    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        self.usage().map(|u| (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()))
    }
}

/// Trait for provider-specific streaming response types
pub trait ProviderStreamResponse: Send + Sync {
    /// Get the content delta for this chunk
    fn content_delta(&self) -> Option<&str>;

    /// Check if this is the final chunk in the stream
    fn is_final(&self) -> bool;

    /// Get role information if available
    fn role(&self) -> Option<&str>;
}

/// Trait for streaming response iterators
pub trait ProviderStreamResponseIter: Iterator<Item = Result<Box<dyn ProviderStreamResponse>, Box<dyn std::error::Error + Send + Sync>>> + Send + Sync {
    // No additional methods needed - just the Iterator constraint with proper bounds
}

/// Conversion mode for provider requests/responses
#[derive(Debug, Clone, Copy)]
pub enum ConversionMode {
    /// Compatible: Convert between different provider formats to ensure compatibility
    Compatible,
    /// Passthrough: Pass requests/responses through with minimal modification
    Passthrough,
}

/// Trait for token usage information
pub trait TokenUsage {
    fn completion_tokens(&self) -> usize;
    fn prompt_tokens(&self) -> usize;
    fn total_tokens(&self) -> usize;
}

// ============================================================================
// PROVIDER FUNCTIONS - NO TRAITS, JUST PARAMETERIZED CONVERSION
// ============================================================================
//
// ARCHITECTURAL DECISION: Function-based Provider API
//
// We chose this function-based approach over the original ProviderInterface trait
// for several critical reasons:
//
// 1. TRAIT OBJECT LIMITATION:
//    - The original ProviderInterface had associated types (Request, Response, etc.)
//    - Traits with associated types cannot be used as trait objects (Box<dyn ProviderInterface>)
//    - This prevented dynamic provider selection at runtime based on request headers
//    - Error: "the trait `ProviderInterface` cannot be made into an object"
//
// 2. DYNAMIC PROVIDER SELECTION REQUIREMENT:
//    - The gateway needs to select providers dynamically based on incoming headers
//    - Cannot know provider type at compile time - must dispatch at runtime
//    - Need ability to return generic trait objects that work polymorphically
//
// 3. WRAPPER TYPE ELIMINATION:
//    - Original design required wrapper types like OpenAIRequestWrapper, OpenAIResponseWrapper
//    - User wanted to implement traits directly on concrete types (ChatCompletionsRequest, etc.)
//    - Function-based approach allows direct trait implementations without wrappers
//
// 4. PARAMETERIZED CONVERSION PATTERN:
//    - Follows existing codebase pattern: TryFrom<(&[u8], &ProviderId)>
//    - Enables runtime provider selection while maintaining type safety
//    - Single implementation can handle multiple OpenAI-compatible providers
//
// 5. TYPE ERASURE FOR GENERIC INTERFACE:
//    - Functions return Box<dyn ProviderRequest/Response> - works as trait objects
//    - stream_context.rs can work with generic interfaces without knowing concrete types
//    - Maintains polymorphism while enabling dynamic dispatch
// ============================================================================

use crate::ProviderId;

// ============================================================================
// PROVIDER ADAPTER REGISTRY (Organizational Enhancement)
// ============================================================================

/// Provider adapter configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub supported_apis: &'static [&'static str],
    pub adapter_type: AdapterType,
}

#[derive(Debug, Clone)]
pub enum AdapterType {
    OpenAICompatible,
    // Future: Claude, Gemini, etc.
}

/// Get provider configuration
pub fn get_provider_config(provider_id: &ProviderId) -> ProviderConfig {
    match provider_id {
        ProviderId::OpenAI | ProviderId::Groq | ProviderId::Mistral | ProviderId::Deepseek
        | ProviderId::Arch | ProviderId::Gemini | ProviderId::Claude | ProviderId::GitHub => {
            ProviderConfig {
                supported_apis: &["/v1/chat/completions"],
                adapter_type: AdapterType::OpenAICompatible,
            }
        }
    }
}

/// Parse request from bytes using provider ID - returns generic ProviderRequest trait object
pub fn try_request_from_bytes(bytes: &[u8], provider_id: &ProviderId) -> Result<Box<dyn ProviderRequest>, ProviderRequestError> {
    let config = get_provider_config(provider_id);

    match config.adapter_type {
        AdapterType::OpenAICompatible => {
            let request = crate::apis::openai::ChatCompletionsRequest::try_from((bytes, provider_id))
                .map_err(|e| ProviderRequestError {
                    message: format!("Failed to parse request: {}", e),
                    source: Some(Box::new(e)),
                })?;

            // Return as trait object - this enables polymorphic usage
            // ChatCompletionsRequest implements ProviderRequest directly (no wrapper needed)
            Ok(Box::new(request) as Box<dyn ProviderRequest>)
        }
    }
}

/// Parse response from bytes using provider ID - returns generic ProviderResponse trait object
pub fn try_response_from_bytes(bytes: &[u8], provider_id: &ProviderId, _mode: ConversionMode) -> Result<Box<dyn ProviderResponse>, ProviderResponseError> {
    let config = get_provider_config(provider_id);

    match config.adapter_type {
        AdapterType::OpenAICompatible => {
            // Parameterized conversion allows provider-specific response parsing
            let response = crate::apis::openai::ChatCompletionsResponse::try_from((bytes, provider_id))
                .map_err(|e| ProviderResponseError {
                    message: format!("Failed to parse response: {}", e),
                    source: Some(Box::new(e)),
                })?;

            // ChatCompletionsResponse implements ProviderResponse directly - no wrapper needed!
            Ok(Box::new(response) as Box<dyn ProviderResponse>)
        }
    }
}

/// Create streaming response using provider ID - returns clean ProviderStreamResponseIter trait object
pub fn try_streaming_from_bytes(bytes: &[u8], provider_id: &ProviderId, _mode: ConversionMode) -> Result<Box<dyn ProviderStreamResponseIter>, Box<dyn std::error::Error + Send + Sync>> {
    let config = get_provider_config(provider_id);

    match config.adapter_type {
        AdapterType::OpenAICompatible => {
            // Parse SSE (Server-Sent Events) streaming data
            let s = std::str::from_utf8(bytes)?;
            let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
            let iter = crate::apis::openai::SseChatCompletionIter::new(lines.into_iter());

            // Return the iterator directly - it implements ProviderStreamResponseIter
            Ok(Box::new(iter))
        }
    }
}

/// Check if provider has compatible API
pub fn has_compatible_api(provider_id: &ProviderId, api_path: &str) -> bool {
    let config = get_provider_config(provider_id);
    config.supported_apis.iter().any(|&supported| supported == api_path)
}

/// Get supported APIs for provider
pub fn supported_apis(provider_id: &ProviderId) -> Vec<&'static str> {
    let config = get_provider_config(provider_id);
    config.supported_apis.to_vec()
}

/// Error types for provider operations
#[derive(Debug)]
pub struct ProviderRequestError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}

#[derive(Debug)]
pub struct ProviderResponseError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}

impl fmt::Display for ProviderRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider request error: {}", self.message)
    }
}

impl fmt::Display for ProviderResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider response error: {}", self.message)
    }
}

impl Error for ProviderRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

impl Error for ProviderResponseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}
