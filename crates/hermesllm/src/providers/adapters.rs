//! Provider adapter configuration and API compatibility utilities.
//
// Note: For all request/response conversions between Anthropic and OpenAI APIs,
// use the peer-reviewed and well-tested implementations in `clients/transformer.rs`.
// This file should not contain conversion logic.

/// Utility to check if a model is from the Claude/Anthropic family
pub fn is_claude_family(model: &str) -> bool {
    let model = model.to_lowercase();
    model.contains("claude") || model.contains("anthropic")
}
use crate::providers::id::ProviderId;

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
                supported_apis: &["/v1/chat/completions"],
                adapter_type: AdapterType::OpenAICompatible,
            }
        }
        ProviderId::Claude => {
            ProviderConfig {
                supported_apis: &["/v1/messages", "/v1/chat/completions"],
                adapter_type: AdapterType::AnthropicCompatible,
            }
        }
    }
}
