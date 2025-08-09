//! Provider implementations for different LLM APIs
//!
//! This module contains provider-specific implementations that handle
//! request/response conversion for different LLM service APIs.

pub mod traits;
pub mod openai;

// Re-export the main interfaces
pub use traits::*;
pub use openai::OpenAIProvider;

use std::fmt::Display;

/// Provider identifier enum - simple enum for identifying providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    OpenAI,
    Mistral,
    Deepseek,
    Groq,
    Gemini,
    Claude,
    GitHub,
    Arch,
}

impl From<&str> for ProviderId {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "openai" => ProviderId::OpenAI,
            "mistral" => ProviderId::Mistral,
            "deepseek" => ProviderId::Deepseek,
            "groq" => ProviderId::Groq,
            "gemini" => ProviderId::Gemini,
            "claude" => ProviderId::Claude,
            "github" => ProviderId::GitHub,
            "arch" => ProviderId::Arch,
            _ => panic!("Unknown provider: {}", value),
        }
    }
}

impl Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderId::OpenAI => write!(f, "OpenAI"),
            ProviderId::Mistral => write!(f, "Mistral"),
            ProviderId::Deepseek => write!(f, "Deepseek"),
            ProviderId::Groq => write!(f, "Groq"),
            ProviderId::Gemini => write!(f, "Gemini"),
            ProviderId::Claude => write!(f, "Claude"),
            ProviderId::GitHub => write!(f, "GitHub"),
            ProviderId::Arch => write!(f, "Arch"),
        }
    }
}

impl ProviderId {
    /// Get the API endpoint path for this provider
    pub fn api_path(&self) -> &'static str {
        match self {
            ProviderId::OpenAI => "/v1/chat/completions",
            ProviderId::Groq => "/openai/v1/chat/completions",
            ProviderId::Gemini => "/v1/models", // TODO: Update when Gemini API is implemented
            ProviderId::Claude => "/v1/messages", // TODO: Update when Claude API is implemented
            ProviderId::Mistral => "/v1/chat/completions",
            ProviderId::Deepseek => "/v1/chat/completions",
            ProviderId::GitHub => "/models", // TODO: Update when GitHub models API is implemented
            ProviderId::Arch => "/v1/chat/completions",
        }
    }

    /// Check if this provider supports OpenAI v1/chat/completions API format
    pub fn supports_openai_format(&self) -> bool {
        matches!(
            self,
            ProviderId::OpenAI | ProviderId::Groq | ProviderId::Mistral | ProviderId::Deepseek | ProviderId::Arch
        )
    }
}

/// Enum for dynamic dispatch of provider instances
/// For now, most providers use OpenAI-compatible format
pub enum Provider {
    OpenAI(OpenAIProvider, ProviderId),
    // TODO: Add specific implementations when providers have different APIs
    // Mistral(MistralProvider, ProviderId),
    // Groq(GroqProvider, ProviderId),
    // etc.
}

impl Provider {
    /// Create a provider instance from a provider ID
    pub fn new(id: ProviderId) -> Self {
        match id {
            // For now, all providers that support v1/chat/completions use OpenAI format
            ProviderId::OpenAI | ProviderId::Groq | ProviderId::Mistral | ProviderId::Deepseek | ProviderId::Arch => {
                Provider::OpenAI(OpenAIProvider, id)
            }
            // TODO: Implement specific providers when they have different APIs
            ProviderId::Gemini | ProviderId::Claude | ProviderId::GitHub => {
                Provider::OpenAI(OpenAIProvider, id) // Fallback to OpenAI for now
            }
        }
    }

    /// Get the provider ID
    pub fn id(&self) -> ProviderId {
        match self {
            Provider::OpenAI(_, id) => *id,
        }
    }

    /// Check if this provider has a compatible API with the client request
    pub fn has_compatible_api(&self, api_path: &str) -> bool {
        match self {
            Provider::OpenAI(provider, _) => provider.has_compatible_api(api_path),
        }
    }

    /// Get the interface implementation for this provider
    pub fn get_interface(&self, passthrough: bool) -> ConversionMode {
        match self {
            Provider::OpenAI(provider, _) => provider.get_interface(passthrough),
        }
    }

    /// Parse a request from raw bytes - returns the concrete OpenAI request type for now
    pub fn parse_request(&self, bytes: &[u8]) -> Result<crate::apis::openai::ChatCompletionsRequest, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Provider::OpenAI(_, _) => {
                use crate::apis::openai::ChatCompletionsRequest;
                use crate::providers::traits::ProviderRequest;

                match ChatCompletionsRequest::try_from_bytes(bytes) {
                    Ok(req) => Ok(req),
                    Err(e) => Err(Box::new(e)),
                }
            }
        }
    }

    /// Parse a response from raw bytes - returns the concrete OpenAI response type for now
    pub fn parse_response(&self, bytes: &[u8], mode: ConversionMode) -> Result<crate::apis::openai::ChatCompletionsResponse, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Provider::OpenAI(_, _) => {
                use crate::apis::openai::ChatCompletionsResponse;
                use crate::providers::traits::ProviderResponse;

                let provider_id = self.id();
                match ChatCompletionsResponse::try_from_bytes(bytes, &provider_id, mode) {
                    Ok(resp) => Ok(resp),
                    Err(e) => Err(Box::new(e)),
                }
            }
        }
    }

    /// Convert a request to bytes for sending to upstream API
    pub fn request_to_bytes(&self, request: &crate::apis::openai::ChatCompletionsRequest, mode: ConversionMode) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Provider::OpenAI(_, _) => {
                use crate::providers::traits::ProviderRequest;

                let provider_id = self.id();
                match request.to_provider_bytes(provider_id, mode) {
                    Ok(bytes) => Ok(bytes),
                    Err(e) => Err(Box::new(e)),
                }
            }
        }
    }
}
