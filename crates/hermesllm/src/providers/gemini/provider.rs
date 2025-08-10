//! Gemini provider implementation
//!
//! TODO: Implement Gemini-specific API format when needed
//! For now, uses OpenAI-compatible format as fallback

use crate::providers::{ProviderInterface, ConversionMode};
use crate::apis::openai::{ChatCompletionsRequest, ChatCompletionsResponse};
use crate::providers::traits::{ProviderRequest, ProviderResponse};

/// Gemini provider implementation
#[derive(Debug, Clone)]
pub struct GeminiProvider;

impl ProviderInterface for GeminiProvider {
    fn has_compatible_api(&self, api_path: &str) -> bool {
        // TODO: Update when Gemini API is fully implemented
        matches!(api_path, "/v1/chat/completions" | "/v1/models")
    }

    fn supported_apis(&self) -> Vec<&'static str> {
        // TODO: Update when Gemini API is fully implemented
        vec!["/v1/models"]
    }

    fn parse_request(&self, bytes: &[u8]) -> Result<ChatCompletionsRequest, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement Gemini-specific request parsing
        match ChatCompletionsRequest::try_from_bytes(bytes) {
            Ok(req) => Ok(req),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn parse_response(&self, bytes: &[u8], provider_id: super::super::ProviderId, mode: ConversionMode) -> Result<ChatCompletionsResponse, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement Gemini-specific response parsing
        match ChatCompletionsResponse::try_from_bytes(bytes, &provider_id, mode) {
            Ok(resp) => Ok(resp),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn request_to_bytes(&self, request: &ChatCompletionsRequest, provider_id: super::super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement Gemini-specific request serialization
        match request.to_provider_bytes(provider_id, mode) {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(Box::new(e)),
        }
    }
}
