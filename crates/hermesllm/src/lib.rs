//! hermesllm: A library for translating LLM API requests and responses
//! between Mistral, Grok, Gemini, and OpenAI-compliant formats.

pub mod providers;
pub mod apis;
pub mod clients;

// Re-export important types and traits
pub use providers::{
    ProviderId, Provider, ConversionMode,
    ProviderInterface, ProviderRequest, ProviderResponse,
    TokenUsage, StreamChunk, StreamingResponse,
    OpenAIProvider
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_conversion() {
        assert_eq!(ProviderId::from("openai"), ProviderId::OpenAI);
        assert_eq!(ProviderId::from("mistral"), ProviderId::Mistral);
        assert_eq!(ProviderId::from("groq"), ProviderId::Groq);
        assert_eq!(ProviderId::from("arch"), ProviderId::Arch);
    }

    #[test]
    fn test_provider_api_paths() {
        assert_eq!(ProviderId::OpenAI.api_path(), "/v1/chat/completions");
        assert_eq!(ProviderId::Groq.api_path(), "/openai/v1/chat/completions");
        assert_eq!(ProviderId::Mistral.api_path(), "/v1/chat/completions");
        assert_eq!(ProviderId::Arch.api_path(), "/v1/chat/completions");
    }

    #[test]
    fn test_provider_openai_format_support() {
        assert!(ProviderId::OpenAI.supports_openai_format());
        assert!(ProviderId::Groq.supports_openai_format());
        assert!(ProviderId::Mistral.supports_openai_format());
        assert!(ProviderId::Arch.supports_openai_format());
        assert!(!ProviderId::Gemini.supports_openai_format());
        assert!(!ProviderId::Claude.supports_openai_format());
    }

    #[test]
    fn test_provider_instance_creation() {
        let provider = Provider::new(ProviderId::OpenAI);
        assert!(provider.has_compatible_api("/v1/chat/completions"));
        assert!(!provider.has_compatible_api("/v1/embeddings"));
    }

    #[test]
    fn test_provider_supported_apis() {
        let provider = Provider::new(ProviderId::OpenAI);

        let supported_apis = provider.supported_apis();
        assert!(supported_apis.contains(&"/v1/chat/completions"));

        // Test that provider supports the expected API endpoints
        assert!(provider.has_compatible_api("/v1/chat/completions"));
    }
}
