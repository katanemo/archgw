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

    /// Get the provider interface implementation
    pub fn interface(&self) -> &dyn ProviderInterface {
        match self {
            Provider::OpenAI(provider, _) => provider,
            Provider::Groq(provider, _) => provider,
            Provider::Mistral(provider, _) => provider,
            Provider::Deepseek(provider, _) => provider,
            Provider::Arch(provider, _) => provider,
            Provider::Gemini(provider, _) => provider,
            Provider::Claude(provider, _) => provider,
            Provider::GitHub(provider, _) => provider,
        }
    }
}
