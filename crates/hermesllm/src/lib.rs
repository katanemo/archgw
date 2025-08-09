//! hermesllm: A library for translating LLM API requests and responses
//! between Mistral, Grok, Gemini, and OpenAI-compliant formats.

pub mod providers;
pub mod apis;
pub mod clients;

// Re-export important traits
pub use providers::traits::*;
pub use providers::openai::provider::OpenAIProvider;
pub use providers::provider_enum::ProviderInstance;


use std::fmt::Display;
pub enum Provider {
    Arch,
    Mistral,
    Deepseek,
    Groq,
    Gemini,
    OpenAI,
    Claude,
    Github,
}

impl From<&str> for Provider {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "arch" => Provider::Arch,
            "mistral" => Provider::Mistral,
            "deepseek" => Provider::Deepseek,
            "groq" => Provider::Groq,
            "gemini" => Provider::Gemini,
            "openai" => Provider::OpenAI,
            "claude" => Provider::Claude,
            "github" => Provider::Github,
            _ => panic!("Unknown provider: {}", value),
        }
    }
}

impl Provider {
    /// Get the API endpoint path for this provider
    pub fn api_path(&self) -> &'static str {
        match self {
            Provider::OpenAI => "/v1/chat/completions",
            Provider::Groq => "/openai/v1/chat/completions", // Groq maps to OpenAI-compatible endpoint
            Provider::Gemini => "/v1/models", // TODO: Update with correct Gemini path
            Provider::Claude => "/v1/messages", // TODO: Update with correct Claude path
            Provider::Mistral => "/v1/chat/completions", // Mistral uses OpenAI-compatible API
            Provider::Deepseek => "/v1/chat/completions", // DeepSeek uses OpenAI-compatible API
            Provider::Arch => "/v1/chat/completions", // Arch gateway endpoint
            Provider::Github => "/models", // TODO: Update with correct GitHub models path
        }
    }

    /// Check if this provider uses OpenAI-compatible API format
    pub fn uses_openai_format(&self) -> bool {
        match self {
            Provider::OpenAI | Provider::Groq | Provider::Mistral | Provider::Deepseek | Provider::Arch => true,
            Provider::Gemini | Provider::Claude | Provider::Github => false, // These have their own formats
        }
    }

    /// Create a provider implementation instance for this provider
    pub fn create_provider_instance(&self) -> ProviderInstance {
        match self {
            Provider::OpenAI => ProviderInstance::OpenAI(OpenAIProvider),
            Provider::Groq => ProviderInstance::OpenAI(OpenAIProvider), // Groq uses OpenAI-compatible API
            Provider::Mistral => ProviderInstance::OpenAI(OpenAIProvider), // Mistral uses OpenAI-compatible API
            Provider::Deepseek => ProviderInstance::OpenAI(OpenAIProvider), // Deepseek uses OpenAI-compatible API
            Provider::Arch => ProviderInstance::OpenAI(OpenAIProvider), // Arch gateway uses OpenAI-compatible API
            // TODO: Implement specific providers for these when they have different APIs
            Provider::Gemini => ProviderInstance::OpenAI(OpenAIProvider), // For now, use OpenAI-compatible
            Provider::Claude => ProviderInstance::OpenAI(OpenAIProvider), // For now, use OpenAI-compatible
            Provider::Github => ProviderInstance::OpenAI(OpenAIProvider), // For now, use OpenAI-compatible
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Arch => write!(f, "Arch"),
            Provider::Mistral => write!(f, "Mistral"),
            Provider::Deepseek => write!(f, "Deepseek"),
            Provider::Groq => write!(f, "Groq"),
            Provider::Gemini => write!(f, "Gemini"),
            Provider::OpenAI => write!(f, "OpenAI"),
            Provider::Claude => write!(f, "Claude"),
            Provider::Github => write!(f, "Github"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::providers::openai::types::{ChatCompletionsRequest, Message};

    #[test]
    fn openai_builder() {
        let request =
            ChatCompletionsRequest::builder("gpt-3.5-turbo", vec![Message::new("Hi".to_string())])
                .temperature(0.7)
                .top_p(0.9)
                .n(1)
                .max_tokens(100)
                .stream(false)
                .stop(vec!["\n".to_string()])
                .presence_penalty(0.0)
                .frequency_penalty(0.0)
                .build()
                .expect("Failed to build OpenAIRequest");

        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.n, Some(1));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.stream, Some(false));
        assert_eq!(request.stop, Some(vec!["\n".to_string()]));
        assert_eq!(request.presence_penalty, Some(0.0));
        assert_eq!(request.frequency_penalty, Some(0.0));
    }
}
