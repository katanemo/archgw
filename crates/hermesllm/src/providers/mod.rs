//! Provider implementations for different LLM APIs
//!
//! This module contains provider-specific implementations that handle
//! request/response conversion for different LLM service APIs.

pub mod openai;
pub mod traits;

// Re-export the main interfaces
pub use traits::*;
// Note: OpenAIProvider has been deprecated in favor of function-based approach
// OpenAI functionality is accessed through openai::builder and openai::types modules

use std::fmt::Display;

use crate::apis::anthropic::MessagesRequest as ClaudeRequest;
use crate::apis::{ChatCompletionsRequest as OpenAIRequest, MessagesRole, Role};

/// Provider identifier enum - simple enum for identifying providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    OpenAI,
    Mistral,
    Deepseek,
    Groq,
    Gemini,
    Claude,
    GitHub,
    Arch,
}

impl From<&str> for ProviderId {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "openai" => ProviderId::OpenAI,
            "mistral" => ProviderId::Mistral,
            "deepseek" => ProviderId::Deepseek,
            "groq" => ProviderId::Groq,
            "gemini" => ProviderId::Gemini,
            "claude" => ProviderId::Claude,
            "github" => ProviderId::GitHub,
            "arch" => ProviderId::Arch,
            _ => panic!("Unknown provider: {}", value),
        }
    }
}

impl Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderId::OpenAI => write!(f, "OpenAI"),
            ProviderId::Mistral => write!(f, "Mistral"),
            ProviderId::Deepseek => write!(f, "Deepseek"),
            ProviderId::Groq => write!(f, "Groq"),
            ProviderId::Gemini => write!(f, "Gemini"),
            ProviderId::Claude => write!(f, "Claude"),
            ProviderId::GitHub => write!(f, "GitHub"),
            ProviderId::Arch => write!(f, "Arch"),
        }
    }
}

pub enum LlmRequest {
    OpenAI(OpenAIRequest),
    Claude(ClaudeRequest),
}

impl LlmRequest {
    pub fn get_model(&self) -> &str {
        match self {
            LlmRequest::OpenAI(req) => req.model.as_str(),
            LlmRequest::Claude(req) => req.model.as_str(),
        }
    }
    pub fn set_model(&mut self, model: String) {
        match self {
            LlmRequest::OpenAI(req) => req.model = model.clone(),
            LlmRequest::Claude(req) => req.model = model.clone(),
        }
    }

    pub fn extract_recent_user_message(&self) -> Option<String> {
        match self {
            LlmRequest::OpenAI(req) => req
                .messages
                .iter()
                .filter(|msg| msg.role == Role::User)
                .last()
                .map(|msg| msg.content.to_string().clone()),
            LlmRequest::Claude(req) => req
                .messages
                .iter()
                .filter(|msg| msg.role == MessagesRole::User)
                .last()
                .map(|msg| msg.content.to_string().clone()),
        }
    }

    pub fn is_streaming(&self) -> bool {
        match self {
            LlmRequest::OpenAI(req) => req.stream.unwrap_or(false),
            LlmRequest::Claude(req) => req.stream.unwrap_or(false),
        }
    }

    pub fn extract_messages_text(&self) -> Vec<String> {
        match self {
            LlmRequest::OpenAI(req) => req
                .messages
                .iter()
                .map(|msg| msg.content.to_string().clone())
                .collect(),
            LlmRequest::Claude(req) => req
                .messages
                .iter()
                .map(|msg| msg.content.to_string().clone())
                .collect(),
        }
    }

    /// Serialize this request into bytes for the target upstream provider.
    /// Simple passthrough for matching providers; basic Claude -> OpenAI conversion when requested.
    pub fn to_bytes(&self, target: ProviderId) -> Result<Vec<u8>, std::io::Error> {
        match (self, target) {
            // Direct passthrough: OpenAI request to OpenAI bytes
            (LlmRequest::OpenAI(req), ProviderId::OpenAI) => serde_json::to_vec(req)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            // Direct passthrough: Claude request to Claude bytes
            (LlmRequest::Claude(req), ProviderId::Claude) => serde_json::to_vec(req)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            // Basic Claude -> OpenAI conversion: map roles and flatten content text
            (LlmRequest::Claude(req), ProviderId::OpenAI) => {
                let chat_completion_request: OpenAIRequest = req
                    .clone()
                    .try_into()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                serde_json::to_vec(&chat_completion_request)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
            // Fallback: throw error with provider name
            (_, _) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported provider: {}", target),
            )),
        }
    }
}

impl TryFrom<&[u8]> for LlmRequest {
    type Error = std::io::Error;

    // if passing bytes without provider id we assume the request is in OpenAI format
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let chat_completion_request: OpenAIRequest = serde_json::from_slice(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(LlmRequest::OpenAI(chat_completion_request))
    }
}

pub struct LlmRequestDetails {
    pub provider_id: ProviderId,
    pub request_bytes: Vec<u8>,
}

impl TryFrom<&LlmRequestDetails> for LlmRequest {
    type Error = std::io::Error;

    fn try_from(raw: &LlmRequestDetails) -> Result<Self, Self::Error> {
        match raw.provider_id {
            ProviderId::OpenAI => {
                let chat_completion_request: OpenAIRequest =
                    serde_json::from_slice(&raw.request_bytes)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(LlmRequest::OpenAI(chat_completion_request))
            }
            ProviderId::Claude => {
                let messages_request: ClaudeRequest = serde_json::from_slice(&raw.request_bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(LlmRequest::Claude(messages_request))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported provider",
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        apis::{ChatCompletionsRequest as OpenAIRequest, MessagesRequest as ClaudeRequest},
        providers::{LlmRequest, LlmRequestDetails},
        ProviderId,
    };

    #[test]
    fn test_llm_request_bytes() {
        let json_request = r#"{
        "model": "gpt-4",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant"
            },
            {
                "role": "user",
                "content": "Hello!"
            }
        ]
    }"#;

        let llm_request: LlmRequest = LlmRequest::try_from(json_request.as_bytes()).unwrap();
        let chat_completion_request: OpenAIRequest = match llm_request {
            LlmRequest::OpenAI(req) => {
                assert_eq!(req.model, "gpt-4");
                req
            }
            _ => panic!("Expected OpenAI request"),
        };

        assert_eq!(chat_completion_request.messages.len(), 2);
    }

    #[test]
    fn test_llm_request_details() {
        let json_request = r#"{
        "model": "gpt-4",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant"
            },
            {
                "role": "user",
                "content": "Hello!"
            }
        ]
    }"#;

        let request_bytes = json_request.as_bytes().to_vec();
        let details = LlmRequestDetails {
            provider_id: ProviderId::OpenAI,
            request_bytes,
        };

        let llm_request: LlmRequest = LlmRequest::try_from(&details).unwrap();
        let chat_completion_request: OpenAIRequest = match llm_request {
            LlmRequest::OpenAI(req) => {
                assert_eq!(req.model, "gpt-4");
                req
            }
            _ => panic!("Expected OpenAI request"),
        };

        assert_eq!(chat_completion_request.messages.len(), 2);
    }

    #[test]
    fn test_llm_request_details_claude() {
        let json_request = r#"{
        "model": "claude-2",
        "messages": [
            {
                "role": "user",
                "content": "Hello!"
            }
        ],
        "max_tokens": 1024
    }"#;

        let request_bytes = json_request.as_bytes().to_vec();
        let details = LlmRequestDetails {
            provider_id: ProviderId::Claude,
            request_bytes,
        };

        let llm_request: LlmRequest = LlmRequest::try_from(&details).unwrap();
        let messages_request: ClaudeRequest = match llm_request {
            LlmRequest::Claude(req) => {
                assert_eq!(req.model, "claude-2");
                req
            }
            _ => panic!("Expected Claude request"),
        };

        assert_eq!(messages_request.messages.len(), 1);
    }
}
