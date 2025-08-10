//! Provider implementations for different LLM APIs
//!
//! This module contains provider-specific implementations that handle
//! request/response conversion for different LLM service APIs.

pub mod traits;
pub mod openai;
pub mod groq;
pub mod mistral;
pub mod deepseek;
pub mod arch;
pub mod gemini;
pub mod claude;
pub mod github;

// Re-export the main interfaces
pub use traits::*;
pub use openai::OpenAIProvider;
pub use groq::GroqProvider;
pub use mistral::MistralProvider;
pub use deepseek::DeepseekProvider;
pub use arch::ArchProvider;
pub use gemini::GeminiProvider;
pub use claude::ClaudeProvider;
pub use github::GitHubProvider;

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
pub enum Provider {
    OpenAI(OpenAIProvider, ProviderId),
    Groq(GroqProvider, ProviderId),
    Mistral(MistralProvider, ProviderId),
    Deepseek(DeepseekProvider, ProviderId),
    Arch(ArchProvider, ProviderId),
    Gemini(GeminiProvider, ProviderId),
    Claude(ClaudeProvider, ProviderId),
    GitHub(GitHubProvider, ProviderId),
}

impl Provider {
    /// Create a provider instance from a provider ID
    pub fn new(id: ProviderId) -> Self {
        match id {
            ProviderId::OpenAI => Provider::OpenAI(OpenAIProvider, id),
            ProviderId::Groq => Provider::Groq(GroqProvider, id),
            ProviderId::Mistral => Provider::Mistral(MistralProvider, id),
            ProviderId::Deepseek => Provider::Deepseek(DeepseekProvider, id),
            ProviderId::Arch => Provider::Arch(ArchProvider, id),
            ProviderId::Gemini => Provider::Gemini(GeminiProvider, id),
            ProviderId::Claude => Provider::Claude(ClaudeProvider, id),
            ProviderId::GitHub => Provider::GitHub(GitHubProvider, id),
        }
    }

    /// Get the provider ID
    pub fn id(&self) -> ProviderId {
        match self {
            Provider::OpenAI(_, id) => *id,
            Provider::Groq(_, id) => *id,
            Provider::Mistral(_, id) => *id,
            Provider::Deepseek(_, id) => *id,
            Provider::Arch(_, id) => *id,
            Provider::Gemini(_, id) => *id,
            Provider::Claude(_, id) => *id,
            Provider::GitHub(_, id) => *id,
        }
    }
}

// Implement traits directly on the Provider enum
impl ProviderRequest for Provider {
    type Error = openai::provider::OpenAIApiError;

    fn try_from_bytes(&self, bytes: &[u8]) -> Result<crate::apis::openai::ChatCompletionsRequest, Self::Error> {
        match self {
            Provider::OpenAI(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
            Provider::Groq(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
            Provider::Mistral(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
            Provider::Deepseek(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
            Provider::Arch(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
            Provider::Gemini(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
            Provider::Claude(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
            Provider::GitHub(provider, _) => ProviderRequest::try_from_bytes(provider, bytes),
        }
    }

    fn to_provider_bytes(&self, request: &crate::apis::openai::ChatCompletionsRequest, provider_id: super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Self::Error> {
        match self {
            Provider::OpenAI(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
            Provider::Groq(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
            Provider::Mistral(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
            Provider::Deepseek(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
            Provider::Arch(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
            Provider::Gemini(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
            Provider::Claude(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
            Provider::GitHub(provider, _) => ProviderRequest::to_provider_bytes(provider, request, provider_id, mode),
        }
    }

    fn extract_model<'a>(&self, request: &'a crate::apis::openai::ChatCompletionsRequest) -> &'a str {
        // Since all providers use the same implementation, just use the first one
        &request.model
    }

    fn is_streaming(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> bool {
        // Since all providers use the same implementation, just use the first one
        request.stream.unwrap_or_default()
    }

    fn set_streaming_options(&self, request: &mut crate::apis::openai::ChatCompletionsRequest) {
        match self {
            Provider::OpenAI(provider, _) => ProviderRequest::set_streaming_options(provider, request),
            Provider::Groq(provider, _) => ProviderRequest::set_streaming_options(provider, request),
            Provider::Mistral(provider, _) => ProviderRequest::set_streaming_options(provider, request),
            Provider::Deepseek(provider, _) => ProviderRequest::set_streaming_options(provider, request),
            Provider::Arch(provider, _) => ProviderRequest::set_streaming_options(provider, request),
            Provider::Gemini(provider, _) => ProviderRequest::set_streaming_options(provider, request),
            Provider::Claude(provider, _) => ProviderRequest::set_streaming_options(provider, request),
            Provider::GitHub(provider, _) => ProviderRequest::set_streaming_options(provider, request),
        }
    }

    fn extract_messages_text(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> String {
        match self {
            Provider::OpenAI(provider, _) => ProviderRequest::extract_messages_text(provider, request),
            Provider::Groq(provider, _) => ProviderRequest::extract_messages_text(provider, request),
            Provider::Mistral(provider, _) => ProviderRequest::extract_messages_text(provider, request),
            Provider::Deepseek(provider, _) => ProviderRequest::extract_messages_text(provider, request),
            Provider::Arch(provider, _) => ProviderRequest::extract_messages_text(provider, request),
            Provider::Gemini(provider, _) => ProviderRequest::extract_messages_text(provider, request),
            Provider::Claude(provider, _) => ProviderRequest::extract_messages_text(provider, request),
            Provider::GitHub(provider, _) => ProviderRequest::extract_messages_text(provider, request),
        }
    }

    fn extract_user_message(&self, request: &crate::apis::openai::ChatCompletionsRequest) -> Option<String> {
        match self {
            Provider::OpenAI(provider, _) => ProviderRequest::extract_user_message(provider, request),
            Provider::Groq(provider, _) => ProviderRequest::extract_user_message(provider, request),
            Provider::Mistral(provider, _) => ProviderRequest::extract_user_message(provider, request),
            Provider::Deepseek(provider, _) => ProviderRequest::extract_user_message(provider, request),
            Provider::Arch(provider, _) => ProviderRequest::extract_user_message(provider, request),
            Provider::Gemini(provider, _) => ProviderRequest::extract_user_message(provider, request),
            Provider::Claude(provider, _) => ProviderRequest::extract_user_message(provider, request),
            Provider::GitHub(provider, _) => ProviderRequest::extract_user_message(provider, request),
        }
    }
}

impl ProviderResponse for Provider {
    type Error = openai::provider::OpenAIApiError;
    type Usage = crate::apis::openai::Usage;

    fn try_from_bytes(&self, bytes: &[u8], provider_id: &super::ProviderId, mode: ConversionMode) -> Result<crate::apis::openai::ChatCompletionsResponse, Self::Error> {
        match self {
            Provider::OpenAI(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Groq(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Mistral(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Deepseek(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Arch(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Gemini(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Claude(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::GitHub(provider, _) => ProviderResponse::try_from_bytes(provider, bytes, provider_id, mode),
        }
    }

    fn usage<'a>(&self, response: &'a crate::apis::openai::ChatCompletionsResponse) -> Option<&'a Self::Usage> {
        // Since all providers use the same implementation, just use the direct implementation
        Some(&response.usage)
    }
}

impl StreamingResponse for Provider {
    type Error = openai::provider::OpenAIApiError;
    type StreamChunk = crate::apis::openai::ChatCompletionsStreamResponse;
    type StreamingIter = openai::provider::OpenAIStreamingResponse;

    fn try_from_bytes(&self, bytes: &[u8], provider_id: &super::ProviderId, mode: ConversionMode) -> Result<Self::StreamingIter, Self::Error> {
        match self {
            Provider::OpenAI(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Groq(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Mistral(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Deepseek(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Arch(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Gemini(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::Claude(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
            Provider::GitHub(provider, _) => StreamingResponse::try_from_bytes(provider, bytes, provider_id, mode),
        }
    }
}

impl ProviderInterface for Provider {
    fn has_compatible_api(&self, api_path: &str) -> bool {
        match self {
            Provider::OpenAI(provider, _) => provider.has_compatible_api(api_path),
            Provider::Groq(provider, _) => provider.has_compatible_api(api_path),
            Provider::Mistral(provider, _) => provider.has_compatible_api(api_path),
            Provider::Deepseek(provider, _) => provider.has_compatible_api(api_path),
            Provider::Arch(provider, _) => provider.has_compatible_api(api_path),
            Provider::Gemini(provider, _) => provider.has_compatible_api(api_path),
            Provider::Claude(provider, _) => provider.has_compatible_api(api_path),
            Provider::GitHub(provider, _) => provider.has_compatible_api(api_path),
        }
    }

    fn supported_apis(&self) -> Vec<&'static str> {
        match self {
            Provider::OpenAI(provider, _) => provider.supported_apis(),
            Provider::Groq(provider, _) => provider.supported_apis(),
            Provider::Mistral(provider, _) => provider.supported_apis(),
            Provider::Deepseek(provider, _) => provider.supported_apis(),
            Provider::Arch(provider, _) => provider.supported_apis(),
            Provider::Gemini(provider, _) => provider.supported_apis(),
            Provider::Claude(provider, _) => provider.supported_apis(),
            Provider::GitHub(provider, _) => provider.supported_apis(),
        }
    }
}
