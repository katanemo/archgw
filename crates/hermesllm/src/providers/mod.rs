//! Provider implementations for different LLM APIs
//!
//! This module contains provider-specific implementations that handle
//! request/response conversion for different LLM service APIs.

pub mod traits;
pub mod openai;

// Re-export the main interfaces
pub use traits::*;
// Note: OpenAIProvider has been deprecated in favor of function-based approach
// OpenAI functionality is accessed through openai::builder and openai::types modules

use std::fmt::Display;

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
