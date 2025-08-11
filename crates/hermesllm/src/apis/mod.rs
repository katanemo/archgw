pub mod anthropic;
pub mod openai;

// Re-export all types for convenience
pub use anthropic::*;
pub use openai::*;

/// Common trait that all API definitions must implement
///
/// This trait ensures consistency across different AI provider API definitions
/// and makes it easy to add new providers like Gemini, Claude, etc.
///
/// Note: This is different from the `ApiProvider` enum in `clients::endpoints`
/// which represents provider identification, while this trait defines API capabilities.
///
/// # Benefits
///
/// - **Consistency**: All API providers implement the same interface
/// - **Extensibility**: Easy to add new providers without breaking existing code
/// - **Type Safety**: Compile-time guarantees that all providers implement required methods
/// - **Discoverability**: Clear documentation of what capabilities each API supports
///
/// # Example implementation for a new provider:
///
/// ```rust,ignore
/// use serde::{Deserialize, Serialize};
/// use super::ApiDefinition;
///
/// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// pub enum GeminiApi {
///     GenerateContent,
///     ChatCompletions,
/// }
///
/// impl GeminiApi {
///     pub fn endpoint(&self) -> &'static str {
///         match self {
///             GeminiApi::GenerateContent => "/v1/models/gemini-pro:generateContent",
///             GeminiApi::ChatCompletions => "/v1/models/gemini-pro:chat",
///         }
///     }
///
///     pub fn from_endpoint(endpoint: &str) -> Option<Self> {
///         match endpoint {
///             "/v1/models/gemini-pro:generateContent" => Some(GeminiApi::GenerateContent),
///             "/v1/models/gemini-pro:chat" => Some(GeminiApi::ChatCompletions),
///             _ => None,
///         }
///     }
///
///     pub fn supports_streaming(&self) -> bool {
///         match self {
///             GeminiApi::GenerateContent => true,
///             GeminiApi::ChatCompletions => true,
///         }
///     }
///
///     pub fn supports_tools(&self) -> bool {
///         match self {
///             GeminiApi::GenerateContent => true,
///             GeminiApi::ChatCompletions => false,
///         }
///     }
///
///     pub fn supports_vision(&self) -> bool {
///         match self {
///             GeminiApi::GenerateContent => true,
///             GeminiApi::ChatCompletions => false,
///         }
///     }
/// }
///
/// impl ApiDefinition for GeminiApi {
///     fn endpoint(&self) -> &'static str {
///         self.endpoint()
///     }
///
///     fn from_endpoint(endpoint: &str) -> Option<Self> {
///         Self::from_endpoint(endpoint)
///     }
///
///     fn supports_streaming(&self) -> bool {
///         self.supports_streaming()
///     }
///
///     fn supports_tools(&self) -> bool {
///         self.supports_tools()
///     }
///
///     fn supports_vision(&self) -> bool {
///         self.supports_vision()
///     }
/// }
///
/// // Now you can use generic code that works with any API:
/// fn print_api_info<T: ApiDefinition>(api: &T) {
///     println!("Endpoint: {}", api.endpoint());
///     println!("Supports streaming: {}", api.supports_streaming());
///     println!("Supports tools: {}", api.supports_tools());
///     println!("Supports vision: {}", api.supports_vision());
/// }
///
/// // Works with both OpenAI and Anthropic (and future Gemini)
/// print_api_info(&OpenAIApi::ChatCompletions);
/// print_api_info(&AnthropicApi::Messages);
/// print_api_info(&GeminiApi::GenerateContent);
/// ```
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
            "/v1/chat/completions",
            "/v1/messages",
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
