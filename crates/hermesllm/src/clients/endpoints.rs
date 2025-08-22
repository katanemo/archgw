//! Supported endpoint registry for LLM APIs
//!
//! This module provides a simple registry to check which API endpoint paths
//! we support across different providers.
//!
//! # Examples
//!
//! ```rust
//! use hermesllm::clients::endpoints::supported_endpoints;
//!
//! // Check if we support an endpoint
//! use hermesllm::clients::endpoints::SupportedApi;
//! assert!(SupportedApi::from_endpoint("/v1/chat/completions").is_some());
//! assert!(SupportedApi::from_endpoint("/v1/messages").is_some());
//! assert!(!SupportedApi::from_endpoint("/v1/unknown").is_some());
//!
//! // Get all supported endpoints
//! let endpoints = supported_endpoints();
//! assert_eq!(endpoints.len(), 2);
//! assert!(endpoints.contains(&"/v1/chat/completions"));
//! assert!(endpoints.contains(&"/v1/messages"));
//! ```

use crate::apis::{AnthropicApi, OpenAIApi, ApiDefinition};

/// Unified enum representing all supported API endpoints across providers
#[derive(Debug, Clone, PartialEq)]
pub enum SupportedApi {
    OpenAI(OpenAIApi),
    Anthropic(AnthropicApi),
}

impl SupportedApi {
    /// Create a SupportedApi from an endpoint path
    pub fn from_endpoint(endpoint: &str) -> Option<Self> {
        if let Some(openai_api) = OpenAIApi::from_endpoint(endpoint) {
            return Some(SupportedApi::OpenAI(openai_api));
        }

        if let Some(anthropic_api) = AnthropicApi::from_endpoint(endpoint) {
            return Some(SupportedApi::Anthropic(anthropic_api));
        }

        None
    }

    /// Get the endpoint path for this API
    pub fn endpoint(&self) -> &'static str {
        match self {
            SupportedApi::OpenAI(api) => api.endpoint(),
            SupportedApi::Anthropic(api) => api.endpoint(),
        }
    }

    /// Determine the target endpoint for a given provider
    /// For /v1/messages: if provider is Anthropic, use /v1/messages; otherwise use /v1/chat/completions
    pub fn target_endpoint_for_provider(&self, provider: &str) -> &'static str {
        match self {
            SupportedApi::Anthropic(AnthropicApi::Messages) => {
                if provider.to_lowercase().contains("anthropic") ||
                   provider.to_lowercase().contains("claude") {
                    "/v1/messages"
                } else {
                    "/v1/chat/completions"
                }
            }
            _ => self.endpoint()
        }
    }
}



/// Get all supported endpoint paths
pub fn supported_endpoints() -> Vec<&'static str> {
    let mut endpoints = Vec::new();

    // Add all OpenAI endpoints
    for api in OpenAIApi::all_variants() {
        endpoints.push(api.endpoint());
    }

    // Add all Anthropic endpoints
    for api in AnthropicApi::all_variants() {
        endpoints.push(api.endpoint());
    }

    endpoints
}

/// Identify which provider supports a given endpoint
pub fn identify_provider(endpoint: &str) -> Option<&'static str> {
    if OpenAIApi::from_endpoint(endpoint).is_some() {
        return Some("openai");
    }

    if AnthropicApi::from_endpoint(endpoint).is_some() {
        return Some("anthropic");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_endpoint() {
        // OpenAI endpoints
        assert!(SupportedApi::from_endpoint("/v1/chat/completions").is_some());

        // Anthropic endpoints
        assert!(SupportedApi::from_endpoint("/v1/messages").is_some());

        // Unsupported endpoints
        assert!(!SupportedApi::from_endpoint("/v1/unknown").is_some());
        assert!(!SupportedApi::from_endpoint("/v2/chat").is_some());
        assert!(!SupportedApi::from_endpoint("").is_some());
    }

    #[test]
    fn test_supported_endpoints() {
        let endpoints = supported_endpoints();
        assert_eq!(endpoints.len(), 2);
        assert!(endpoints.contains(&"/v1/chat/completions"));
        assert!(endpoints.contains(&"/v1/messages"));
    }

    #[test]
    fn test_identify_provider() {
        assert_eq!(identify_provider("/v1/chat/completions"), Some("openai"));
        assert_eq!(identify_provider("/v1/messages"), Some("anthropic"));
        assert_eq!(identify_provider("/v1/unknown"), None);
    }

    #[test]
    fn test_endpoints_generated_from_api_definitions() {
        let endpoints = supported_endpoints();

        // Verify that we get endpoints from all API variants
        let openai_endpoints: Vec<_> = OpenAIApi::all_variants()
            .iter()
            .map(|api| api.endpoint())
            .collect();
        let anthropic_endpoints: Vec<_> = AnthropicApi::all_variants()
            .iter()
            .map(|api| api.endpoint())
            .collect();

        // All OpenAI endpoints should be in the result
        for endpoint in openai_endpoints {
            assert!(endpoints.contains(&endpoint), "Missing OpenAI endpoint: {}", endpoint);
        }

        // All Anthropic endpoints should be in the result
        for endpoint in anthropic_endpoints {
            assert!(endpoints.contains(&endpoint), "Missing Anthropic endpoint: {}", endpoint);
        }

        // Total should match
        assert_eq!(endpoints.len(), OpenAIApi::all_variants().len() + AnthropicApi::all_variants().len());
    }
}
