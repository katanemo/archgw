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
    Anthropic,
    GitHub,
    Arch,
    AzureOpenAI,
    XAI,
    TogetherAI,
    Ollama,
}

impl From<&str> for ProviderId {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "openai" => ProviderId::OpenAI,
            "mistral" => ProviderId::Mistral,
            "deepseek" => ProviderId::Deepseek,
            "groq" => ProviderId::Groq,
            "gemini" => ProviderId::Gemini,
            "anthropic" => ProviderId::Anthropic,
            "github" => ProviderId::GitHub,
            "arch" => ProviderId::Arch,
            "azure_openai" => ProviderId::AzureOpenAI,
            "xai" => ProviderId::XAI,
            "together_ai" => ProviderId::TogetherAI,
            "ollama" => ProviderId::Ollama,
            _ => panic!("Unknown provider: {}", value),
        }
    }
}

impl ProviderId {
    /// Given a client API, return the compatible upstream API for this provider
    pub fn compatible_api_for_client(&self, client_api: &SupportedAPIs) -> SupportedAPIs {
        match (self, client_api) {
            // Claude/Anthropic providers natively support Anthropic APIs
            (ProviderId::Anthropic, SupportedAPIs::AnthropicMessagesAPI(_)) => SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages),
            (ProviderId::Anthropic, SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions)) => SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions),

            // OpenAI-compatible providers only support OpenAI chat completions
            (ProviderId::OpenAI
            | ProviderId::Groq
            | ProviderId::Mistral
            | ProviderId::Deepseek
            | ProviderId::Arch
            | ProviderId::Gemini
            | ProviderId::GitHub
            | ProviderId::AzureOpenAI
            | ProviderId::XAI
            | ProviderId::TogetherAI
            | ProviderId::Ollama,
            SupportedAPIs::AnthropicMessagesAPI(_)) => SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions),

            (ProviderId::OpenAI
            | ProviderId::Groq
            | ProviderId::Mistral
            | ProviderId::Deepseek
            | ProviderId::Arch
            | ProviderId::Gemini
            | ProviderId::GitHub
            | ProviderId::AzureOpenAI
            | ProviderId::XAI
            | ProviderId::TogetherAI
            | ProviderId::Ollama,
            SupportedAPIs::OpenAIChatCompletions(_)) => SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions),
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
            ProviderId::Anthropic => write!(f, "Anthropic"),
            ProviderId::GitHub => write!(f, "GitHub"),
            ProviderId::Arch => write!(f, "Arch"),
            ProviderId::AzureOpenAI => write!(f, "azure_openai"),
            ProviderId::XAI => write!(f, "xai"),
            ProviderId::TogetherAI => write!(f, "together_ai"),
            ProviderId::Ollama => write!(f, "ollama"),
        }
    }
}
