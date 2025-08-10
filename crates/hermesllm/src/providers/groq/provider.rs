//! Groq provider implementation
//!
//! This module contains the Groq provider that handles Groq API format requests.
//! Groq uses OpenAI-compatible format but may have provider-specific nuances.

use crate::providers::{ProviderInterface, ConversionMode};
use crate::apis::openai::{ChatCompletionsRequest, ChatCompletionsResponse};
use crate::providers::traits::{ProviderRequest, ProviderResponse};

/// Groq provider implementation
#[derive(Debug, Clone)]
pub struct GroqProvider;

impl ProviderInterface for GroqProvider {
    fn has_compatible_api(&self, api_path: &str) -> bool {
        matches!(api_path, "/v1/chat/completions" | "/openai/v1/chat/completions")
    }

    fn supported_apis(&self) -> Vec<&'static str> {
        vec!["/openai/v1/chat/completions"]
    }

    fn parse_request(&self, bytes: &[u8]) -> Result<ChatCompletionsRequest, Box<dyn std::error::Error + Send + Sync>> {
        match ChatCompletionsRequest::try_from_bytes(bytes) {
            Ok(req) => Ok(req),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn parse_response(&self, bytes: &[u8], provider_id: super::super::ProviderId, mode: ConversionMode) -> Result<ChatCompletionsResponse, Box<dyn std::error::Error + Send + Sync>> {
        match ChatCompletionsResponse::try_from_bytes(bytes, &provider_id, mode) {
            Ok(resp) => Ok(resp),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn request_to_bytes(&self, request: &ChatCompletionsRequest, provider_id: super::super::ProviderId, mode: ConversionMode) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        match request.to_provider_bytes(provider_id, mode) {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(Box::new(e)),
        }
    }
}
