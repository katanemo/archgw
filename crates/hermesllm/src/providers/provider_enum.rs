use crate::providers::traits::*;
use crate::providers::openai::provider::{OpenAIProvider, OpenAIStreamingResponse};
use crate::apis::openai::{ChatCompletionsRequest, ChatCompletionsResponse, Usage};

/// Enum that wraps all possible providers for dynamic dispatch
pub enum ProviderInstance {
    OpenAI(OpenAIProvider),
    // TODO: Add other providers as they are implemented
    // Anthropic(AnthropicProvider),
    // Mistral(MistralProvider),
    // etc.
}

impl ProviderInstance {
    /// Creates a provider from a provider name string
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "openai" | "groq" | "gemini" | "mistral" | "deepseek" | "arch" | "claude" => {
                ProviderInstance::OpenAI(OpenAIProvider)
            }
            // TODO: Add other providers when implemented
            // "claude" | "anthropic" => ProviderInstance::Anthropic(AnthropicProvider),
            // "mistral" => ProviderInstance::Mistral(MistralProvider),
            _ => {
                // Default to OpenAI for unknown providers
                ProviderInstance::OpenAI(OpenAIProvider)
            }
        }
    }

    /// Parse request from bytes using the appropriate provider
    pub fn parse_request(&self, bytes: &[u8]) -> Result<ChatCompletionsRequest, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            ProviderInstance::OpenAI(_) => {
                ChatCompletionsRequest::try_from_bytes(bytes).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            // TODO: Add other provider cases when implemented
        }
    }

    /// Parse response from bytes using the appropriate provider
    pub fn parse_response(&self, bytes: &[u8], provider: &crate::Provider) -> Result<ChatCompletionsResponse, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            ProviderInstance::OpenAI(_) => {
                ChatCompletionsResponse::try_from_bytes(bytes, provider).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            // TODO: Add other provider cases when implemented
        }
    }

    /// Parse streaming response from bytes using the appropriate provider
    pub fn parse_streaming_response(&self, bytes: &[u8], provider: &crate::Provider) -> Result<OpenAIStreamingResponse, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            ProviderInstance::OpenAI(_) => {
                OpenAIStreamingResponse::try_from_bytes(bytes, provider).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            // TODO: Add other provider cases when implemented
        }
    }
}

impl ProviderInterface for ProviderInstance {
    type Request = ChatCompletionsRequest;
    type Response = ChatCompletionsResponse;
    type StreamingResponse = OpenAIStreamingResponse;
    type Usage = Usage;
}
