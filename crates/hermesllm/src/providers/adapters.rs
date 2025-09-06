use crate::providers::id::ProviderId;
use crate::{CHAT_COMPLETIONS_PATH, MESSAGES_PATH};

#[derive(Debug, Clone)]
pub enum AdapterType {
    OpenAICompatible,
    AnthropicCompatible,
    // Future: Gemini, etc.
}

/// Provider adapter configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub supported_apis: &'static [&'static str],
    pub adapter_type: AdapterType,
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

/// Get provider configuration
pub fn get_provider_config(provider_id: &ProviderId) -> ProviderConfig {
    match provider_id {
        ProviderId::OpenAI | ProviderId::Groq | ProviderId::Mistral | ProviderId::Deepseek
        | ProviderId::Arch | ProviderId::Gemini | ProviderId::GitHub => {
            ProviderConfig {
                supported_apis: &[CHAT_COMPLETIONS_PATH],
                adapter_type: AdapterType::OpenAICompatible,
            }
        }
        ProviderId::Anthropic => {
            ProviderConfig {
                supported_apis: &[MESSAGES_PATH],
                adapter_type: AdapterType::AnthropicCompatible,
            }
        }
    }
}
