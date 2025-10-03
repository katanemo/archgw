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
//! use hermesllm::clients::endpoints::SupportedAPIs;
//! assert!(SupportedAPIs::from_endpoint("/v1/chat/completions").is_some());
//! assert!(SupportedAPIs::from_endpoint("/v1/messages").is_some());
//! assert!(!SupportedAPIs::from_endpoint("/v1/unknown").is_some());
//!
//! // Get all supported endpoints
//! let endpoints = supported_endpoints();
//! assert_eq!(endpoints.len(), 2);
//! assert!(endpoints.contains(&"/v1/chat/completions"));
//! assert!(endpoints.contains(&"/v1/messages"));
//! ```

use crate::{apis::{AnthropicApi, ApiDefinition, OpenAIApi}, ProviderId};
use std::fmt;

/// Unified enum representing all supported API endpoints across providers
#[derive(Debug, Clone, PartialEq)]
pub enum SupportedAPIs {
    OpenAIChatCompletions(OpenAIApi),
    AnthropicMessagesAPI(AnthropicApi),
}

impl fmt::Display for SupportedAPIs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SupportedAPIs::OpenAIChatCompletions(api) => write!(f, "OpenAI API ({})", api.endpoint()),
            SupportedAPIs::AnthropicMessagesAPI(api) => write!(f, "Anthropic API ({})", api.endpoint()),
        }
    }
}

impl SupportedAPIs {
    /// Create a SupportedApi from an endpoint path
    pub fn from_endpoint(endpoint: &str) -> Option<Self> {
        if let Some(openai_api) = OpenAIApi::from_endpoint(endpoint) {
            return Some(SupportedAPIs::OpenAIChatCompletions(openai_api));
        }

        if let Some(anthropic_api) = AnthropicApi::from_endpoint(endpoint) {
            return Some(SupportedAPIs::AnthropicMessagesAPI(anthropic_api));
        }

        None
    }

    /// Get the endpoint path for this API
    pub fn endpoint(&self) -> &'static str {
        match self {
            SupportedAPIs::OpenAIChatCompletions(api) => api.endpoint(),
            SupportedAPIs::AnthropicMessagesAPI(api) => api.endpoint(),
        }
    }

    pub fn target_endpoint_for_provider(&self, provider_id: &ProviderId, request_path: &str, model_id: &str) -> String {
        let default_endpoint = "/v1/chat/completions".to_string();
        match self {
            SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages) => {
                match provider_id {
                    ProviderId::Anthropic => "/v1/messages".to_string(),
                    _ => default_endpoint,
                }
            }
            _ => {
                match provider_id {
                    ProviderId::Groq => {
                        if request_path.starts_with("/v1/") {
                            format!("/openai{}", request_path)
                        } else {
                            default_endpoint
                        }
                    }
                    ProviderId::Zhipu => {
                        if request_path.starts_with("/v1/") {
                            "/api/paas/v4/chat/completions".to_string()
                        } else {
                            default_endpoint
                        }
                    }
                    ProviderId::Qwen => {
                        if request_path.starts_with("/v1/") {
                            "/compatible-mode/v1/chat/completions".to_string()
                        } else {
                            default_endpoint
                        }
                    }
                    ProviderId::AzureOpenAI => {
                        if request_path.starts_with("/v1/") {
                            format!("/openai/deployments/{}/chat/completions?api-version=2025-01-01-preview", model_id)
                        } else {
                            default_endpoint
                        }
                    }
                    ProviderId::Gemini => {
                        if request_path.starts_with("/v1/") {
                            "/v1beta/openai/chat/completions".to_string()
                        } else {
                            default_endpoint
                        }
                    }
                    _ => default_endpoint,
                }
            }
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
        assert!(SupportedAPIs::from_endpoint("/v1/chat/completions").is_some());

        // Anthropic endpoints
        assert!(SupportedAPIs::from_endpoint("/v1/messages").is_some());

        // Unsupported endpoints
        assert!(!SupportedAPIs::from_endpoint("/v1/unknown").is_some());
        assert!(!SupportedAPIs::from_endpoint("/v2/chat").is_some());
        assert!(!SupportedAPIs::from_endpoint("").is_some());
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
