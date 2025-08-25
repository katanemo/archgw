use std::fmt::Display;
use crate::clients::endpoints::SupportedAPIs;
use crate::apis::{OpenAIApi, AnthropicApi};

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

impl ProviderId {
    /// Given a client API, return the compatible upstream API for this provider
    pub fn compatible_api_for_client(&self, client_api: &SupportedAPIs) -> SupportedAPIs {
        match (self, client_api) {
            // Claude/Anthropic providers natively support Anthropic APIs
            (ProviderId::Claude, SupportedAPIs::AnthropicMessagesAPI(_)) => client_api.clone(),
            // Claude/Anthropic providers can also support OpenAI chat completions by mapping to Anthropic Messages
            (ProviderId::Claude, SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions)) => SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages),

            // OpenAI-compatible providers only support OpenAI chat completions
            (ProviderId::OpenAI | ProviderId::Groq | ProviderId::Mistral | ProviderId::Deepseek | ProviderId::Arch | ProviderId::Gemini | ProviderId::GitHub, SupportedAPIs::AnthropicMessagesAPI(_)) => SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions),
            (ProviderId::OpenAI | ProviderId::Groq | ProviderId::Mistral | ProviderId::Deepseek | ProviderId::Arch | ProviderId::Gemini | ProviderId::GitHub, SupportedAPIs::OpenAIChatCompletions(_)) => SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions),
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
