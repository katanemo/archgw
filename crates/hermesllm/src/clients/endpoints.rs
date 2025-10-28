use crate::apis::{AmazonBedrockApi, AnthropicApi, ApiDefinition, OpenAIApi};
use crate::ProviderId;
use std::fmt;

/// Unified enum representing all supported API endpoints across providers
#[derive(Debug, Clone, PartialEq)]
pub enum SupportedAPIs {
    OpenAIChatCompletions(OpenAIApi),
    AnthropicMessagesAPI(AnthropicApi),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SupportedUpstreamAPIs {
    OpenAIChatCompletions(OpenAIApi),
    AnthropicMessagesAPI(AnthropicApi),
    AmazonBedrockConverse(AmazonBedrockApi),
    AmazonBedrockConverseStream(AmazonBedrockApi),
}

impl fmt::Display for SupportedAPIs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SupportedAPIs::OpenAIChatCompletions(api) => {
                write!(f, "OpenAI ({})", api.endpoint())
            }
            SupportedAPIs::AnthropicMessagesAPI(api) => {
                write!(f, "Anthropic AI ({})", api.endpoint())
            }
        }
    }
}

impl fmt::Display for SupportedUpstreamAPIs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SupportedUpstreamAPIs::OpenAIChatCompletions(api) => {
                write!(f, "OpenAI ({})", api.endpoint())
            }
            SupportedUpstreamAPIs::AnthropicMessagesAPI(api) => {
                write!(f, "Anthropic ({})", api.endpoint())
            }
            SupportedUpstreamAPIs::AmazonBedrockConverse(api) => {
                write!(f, "Amazon Bedrock ({})", api.endpoint())
            }
            SupportedUpstreamAPIs::AmazonBedrockConverseStream(api) => {
                write!(f, "Amazon Bedrock ({})", api.endpoint())
            }
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

    pub fn target_endpoint_for_provider(
        &self,
        provider_id: &ProviderId,
        request_path: &str,
        model_id: &str,
        is_streaming: bool,
        base_url_path_prefix: &str,
    ) -> String {
        // Helper function to build endpoint with optional prefix override
        let build_endpoint = |provider_prefix: &str, suffix: &str| -> String {
            if !base_url_path_prefix.is_empty() {
                // Use base_url_path_prefix instead of provider's default prefix
                let prefix = base_url_path_prefix.trim_matches('/');
                if prefix.is_empty() {
                    // If prefix is just slashes, return suffix as-is
                    suffix.to_string()
                } else {
                    format!("/{}{}", prefix, suffix)
                }
            } else {
                // Use provider's default prefix
                if provider_prefix.is_empty() {
                    suffix.to_string()
                } else {
                    format!("{}{}", provider_prefix, suffix)
                }
            }
        };

        match self {
            SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages) => match provider_id {
                ProviderId::Anthropic => build_endpoint("/v1", "/messages"),
                ProviderId::AmazonBedrock => {
                    if request_path.starts_with("/v1/") && !is_streaming {
                        build_endpoint("", &format!("/model/{}/converse", model_id))
                    } else if request_path.starts_with("/v1/") && is_streaming {
                        build_endpoint("", &format!("/model/{}/converse-stream", model_id))
                    } else {
                        build_endpoint("/v1", "/chat/completions")
                    }
                }
                _ => build_endpoint("/v1", "/chat/completions"),
            },
            _ => match provider_id {
                ProviderId::Groq => {
                    if request_path.starts_with("/v1/") {
                        build_endpoint("/openai", request_path)
                    } else {
                        build_endpoint("/v1", "/chat/completions")
                    }
                }
                ProviderId::Zhipu => {
                    if request_path.starts_with("/v1/") {
                        build_endpoint("/api/paas/v4", "/chat/completions")
                    } else {
                        build_endpoint("/v1", "/chat/completions")
                    }
                }
                ProviderId::Qwen => {
                    if request_path.starts_with("/v1/") {
                        build_endpoint("/compatible-mode/v1", "/chat/completions")
                    } else {
                        build_endpoint("/v1", "/chat/completions")
                    }
                }
                ProviderId::AzureOpenAI => {
                    if request_path.starts_with("/v1/") {
                        build_endpoint("/openai/deployments", &format!("/{}/chat/completions?api-version=2025-01-01-preview", model_id))
                    } else {
                        build_endpoint("/v1", "/chat/completions")
                    }
                }
                ProviderId::Gemini => {
                    if request_path.starts_with("/v1/") {
                        build_endpoint("/v1beta/openai", "/chat/completions")
                    } else {
                        build_endpoint("/v1", "/chat/completions")
                    }
                }
                ProviderId::AmazonBedrock => {
                    if request_path.starts_with("/v1/") {
                        if !is_streaming {
                            build_endpoint("", &format!("/model/{}/converse", model_id))
                        } else {
                            build_endpoint("", &format!("/model/{}/converse-stream", model_id))
                        }
                    } else {
                        build_endpoint("/v1", "/chat/completions")
                    }
                }
                _ => build_endpoint("/v1", "/chat/completions"),
            },
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
        assert_eq!(endpoints.len(), 2); // We have 2 APIs defined
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
            assert!(
                endpoints.contains(&endpoint),
                "Missing OpenAI endpoint: {}",
                endpoint
            );
        }

        // All Anthropic endpoints should be in the result
        for endpoint in anthropic_endpoints {
            assert!(
                endpoints.contains(&endpoint),
                "Missing Anthropic endpoint: {}",
                endpoint
            );
        }
        // Total should match
        assert_eq!(
            endpoints.len(),
            OpenAIApi::all_variants().len() + AnthropicApi::all_variants().len()
        );
    }

    #[test]
    fn test_target_endpoint_without_base_url_prefix() {
        let api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Test default OpenAI provider
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::OpenAI,
                "/v1/chat/completions",
                "gpt-4",
                false,
                ""
            ),
            "/v1/chat/completions"
        );

        // Test Groq provider
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Groq,
                "/v1/chat/completions",
                "llama2",
                false,
                ""
            ),
            "/openai/v1/chat/completions"
        );

        // Test Zhipu provider
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/v1/chat/completions",
                "chatglm",
                false,
                ""
            ),
            "/api/paas/v4/chat/completions"
        );

        // Test Qwen provider
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Qwen,
                "/v1/chat/completions",
                "qwen-turbo",
                false,
                ""
            ),
            "/compatible-mode/v1/chat/completions"
        );

        // Test Azure OpenAI provider
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::AzureOpenAI,
                "/v1/chat/completions",
                "gpt-4",
                false,
                ""
            ),
            "/openai/deployments/gpt-4/chat/completions?api-version=2025-01-01-preview"
        );

        // Test Gemini provider
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Gemini,
                "/v1/chat/completions",
                "gemini-pro",
                false,
                ""
            ),
            "/v1beta/openai/chat/completions"
        );
    }

    #[test]
    fn test_target_endpoint_with_base_url_prefix() {
        let api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Test Zhipu with custom base_url_path_prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/v1/chat/completions",
                "chatglm",
                false,
                "/api/coding/paas/v4"
            ),
            "/api/coding/paas/v4/chat/completions"
        );

        // Test with prefix without leading slash
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/v1/chat/completions",
                "chatglm",
                false,
                "api/coding/paas/v4"
            ),
            "/api/coding/paas/v4/chat/completions"
        );

        // Test with prefix with trailing slash
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/v1/chat/completions",
                "chatglm",
                false,
                "/api/coding/paas/v4/"
            ),
            "/api/coding/paas/v4/chat/completions"
        );

        // Test OpenAI with custom prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::OpenAI,
                "/v1/chat/completions",
                "gpt-4",
                false,
                "/custom/api/v2"
            ),
            "/custom/api/v2/chat/completions"
        );

        // Test Groq with custom prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Groq,
                "/v1/chat/completions",
                "llama2",
                false,
                "/api/v2"
            ),
            "/api/v2/v1/chat/completions"
        );
    }

    #[test]
    fn test_target_endpoint_with_empty_base_url_prefix() {
        let api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Test with just slashes - should use default
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/v1/chat/completions",
                "chatglm",
                false,
                "/"
            ),
            "/chat/completions"
        );

        // Test with empty string
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/v1/chat/completions",
                "chatglm",
                false,
                ""
            ),
            "/api/paas/v4/chat/completions"
        );
    }

    #[test]
    fn test_amazon_bedrock_endpoints() {
        let api = SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);

        // Test Bedrock non-streaming without prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::AmazonBedrock,
                "/v1/messages",
                "us.amazon.nova-pro-v1:0",
                false,
                ""
            ),
            "/model/us.amazon.nova-pro-v1:0/converse"
        );

        // Test Bedrock streaming without prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::AmazonBedrock,
                "/v1/messages",
                "us.amazon.nova-pro-v1:0",
                true,
                ""
            ),
            "/model/us.amazon.nova-pro-v1:0/converse-stream"
        );

        // Test Bedrock non-streaming with prefix (prefix shouldn't affect bedrock)
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::AmazonBedrock,
                "/v1/messages",
                "us.amazon.nova-pro-v1:0",
                false,
                "/custom/path"
            ),
            "/custom/path/model/us.amazon.nova-pro-v1:0/converse"
        );

        // Test Bedrock streaming with prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::AmazonBedrock,
                "/v1/messages",
                "us.amazon.nova-pro-v1:0",
                true,
                "/custom/path"
            ),
            "/custom/path/model/us.amazon.nova-pro-v1:0/converse-stream"
        );
    }

    #[test]
    fn test_anthropic_messages_endpoint() {
        let api = SupportedAPIs::AnthropicMessagesAPI(AnthropicApi::Messages);

        // Test Anthropic without prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Anthropic,
                "/v1/messages",
                "claude-3-opus",
                false,
                ""
            ),
            "/v1/messages"
        );

        // Test Anthropic with prefix
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Anthropic,
                "/v1/messages",
                "claude-3-opus",
                false,
                "/api/v2"
            ),
            "/api/v2/messages"
        );
    }

    #[test]
    fn test_non_v1_request_paths() {
        let api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Test Groq with non-v1 path (should use default)
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Groq,
                "/custom/path",
                "llama2",
                false,
                ""
            ),
            "/v1/chat/completions"
        );

        // Test Zhipu with non-v1 path
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/custom/path",
                "chatglm",
                false,
                ""
            ),
            "/v1/chat/completions"
        );

        // Test with prefix on non-v1 path
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::Zhipu,
                "/custom/path",
                "chatglm",
                false,
                "/api/v2"
            ),
            "/api/v2/chat/completions"
        );
    }

    #[test]
    fn test_azure_openai_with_query_params() {
        let api = SupportedAPIs::OpenAIChatCompletions(OpenAIApi::ChatCompletions);

        // Test Azure without prefix - should include query params
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::AzureOpenAI,
                "/v1/chat/completions",
                "gpt-4-deployment",
                false,
                ""
            ),
            "/openai/deployments/gpt-4-deployment/chat/completions?api-version=2025-01-01-preview"
        );

        // Test Azure with prefix - prefix should replace /openai/deployments
        assert_eq!(
            api.target_endpoint_for_provider(
                &ProviderId::AzureOpenAI,
                "/v1/chat/completions",
                "gpt-4-deployment",
                false,
                "/custom/azure/path"
            ),
            "/custom/azure/path/gpt-4-deployment/chat/completions?api-version=2025-01-01-preview"
        );
    }
}
