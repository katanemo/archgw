pub mod anthropic;
pub mod openai;
pub use anthropic::*;
pub use openai::*;


pub trait ApiDefinition {
    /// Returns the endpoint path for this API
    fn endpoint(&self) -> &'static str;

    /// Creates an API instance from an endpoint path
    fn from_endpoint(endpoint: &str) -> Option<Self>
    where
        Self: Sized;

    /// Returns whether this API supports streaming responses
    fn supports_streaming(&self) -> bool;

    /// Returns whether this API supports tool/function calling
    fn supports_tools(&self) -> bool;

    /// Returns whether this API supports vision/image processing
    fn supports_vision(&self) -> bool;

    /// Returns all variants of this API enum
    fn all_variants() -> Vec<Self>
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CHAT_COMPLETIONS_PATH, MESSAGES_PATH};

    #[test]
    fn test_generic_api_functionality() {
        // Test that our generic API functionality works with both providers
        fn test_api<T: ApiDefinition>(api: &T) {
            let endpoint = api.endpoint();
            assert!(!endpoint.is_empty());
            assert!(endpoint.starts_with('/'));
        }

        test_api(&OpenAIApi::ChatCompletions);
        test_api(&AnthropicApi::Messages);
    }

    #[test]
    fn test_api_detection_from_endpoints() {
        // Test that we can detect APIs from endpoints using the trait
        let endpoints = vec![
            CHAT_COMPLETIONS_PATH,
            MESSAGES_PATH,
            "/v1/unknown"
        ];

        let mut detected_apis = Vec::new();

        for endpoint in endpoints {
            if let Some(api) = OpenAIApi::from_endpoint(endpoint) {
                detected_apis.push(format!("OpenAI: {:?}", api));
            } else if let Some(api) = AnthropicApi::from_endpoint(endpoint) {
                detected_apis.push(format!("Anthropic: {:?}", api));
            } else {
                detected_apis.push("Unknown API".to_string());
            }
        }

        assert_eq!(detected_apis, vec![
            "OpenAI: ChatCompletions",
            "Anthropic: Messages",
            "Unknown API"
        ]);
    }

    #[test]
    fn test_all_variants_method() {
        // Test that all_variants returns the expected variants
        let openai_variants = OpenAIApi::all_variants();
        assert_eq!(openai_variants.len(), 1);
        assert!(openai_variants.contains(&OpenAIApi::ChatCompletions));

        let anthropic_variants = AnthropicApi::all_variants();
        assert_eq!(anthropic_variants.len(), 1);
        assert!(anthropic_variants.contains(&AnthropicApi::Messages));

        // Verify each variant has a valid endpoint
        for variant in openai_variants {
            assert!(!variant.endpoint().is_empty());
        }

        for variant in anthropic_variants {
            assert!(!variant.endpoint().is_empty());
        }
    }
}
