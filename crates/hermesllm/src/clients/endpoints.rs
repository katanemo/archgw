//! Supported endpoint registry for LLM APIs
//!
//! This module provides a simple registry to check which API endpoint paths
//! we support across different providers.
//!
//! # Examples
//!
//! ```rust
//! use hermesllm::clients::endpoints::{is_supported_endpoint, supported_endpoints};
//!
//! // Check if we support an endpoint
//! assert!(is_supported_endpoint("/v1/chat/completions"));
//! assert!(is_supported_endpoint("/v1/messages"));
//! assert!(!is_supported_endpoint("/v1/unknown"));
//!
//! // Get all supported endpoints
//! let endpoints = supported_endpoints();
//! assert_eq!(endpoints.len(), 2);
//! assert!(endpoints.contains(&"/v1/chat/completions"));
//! assert!(endpoints.contains(&"/v1/messages"));
//! ```

use crate::apis::{AnthropicApi, OpenAIApi, ApiDefinition};

/// Check if the given endpoint path is supported
pub fn is_supported_endpoint(endpoint: &str) -> bool {
    // Try OpenAI APIs
    if OpenAIApi::from_endpoint(endpoint).is_some() {
        return true;
    }

    // Try Anthropic APIs
    if AnthropicApi::from_endpoint(endpoint).is_some() {
        return true;
    }

    false
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
        assert!(is_supported_endpoint("/v1/chat/completions"));

        // Anthropic endpoints
        assert!(is_supported_endpoint("/v1/messages"));

        // Unsupported endpoints
        assert!(!is_supported_endpoint("/v1/unknown"));
        assert!(!is_supported_endpoint("/v2/chat"));
        assert!(!is_supported_endpoint(""));
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
