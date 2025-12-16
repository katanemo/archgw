use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use common::configuration::LlmProviderType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique model identifier (e.g., "gpt-4-turbo", "claude-opus-4-5-20251101")
    pub id: String,

    /// Display name for UI/logging
    pub display_name: String,

    /// Gemini-style model name (e.g., "models/gemini-2.5-pro")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Short description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Provider type (claude, openai, gemini, etc.)
    pub provider: LlmProviderType,

    /// Object type for OpenAI compatibility
    #[serde(default = "default_object")]
    pub object: String,

    /// Creation timestamp
    #[serde(default = "chrono::Utc::now")]
    pub created: DateTime<Utc>,

    /// Owner/organization
    pub owned_by: String,

    /// Maximum tokens for input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_limit: Option<u32>,

    /// Maximum tokens for output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_limit: Option<u32>,

    /// Context window size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,

    /// Maximum completion tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,

    /// Supports vision/image input
    #[serde(default)]
    pub supports_vision: bool,

    /// Supports function calling
    #[serde(default)]
    pub supports_function_calling: bool,

    /// Supports streaming responses
    #[serde(default = "default_true")]
    pub supports_streaming: bool,

    /// Model status
    #[serde(default)]
    pub status: ModelStatus,

    /// Pricing information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<ModelPricing>,

    /// Thinking/extended thinking support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingSupport>,

    /// Supported generation methods (Gemini)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_generation_methods: Option<Vec<String>>,

    /// Last updated timestamp
    #[serde(default = "chrono::Utc::now")]
    pub updated_at: DateTime<Utc>,
}

fn default_object() -> String {
    "model".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    #[default]
    Active,
    Beta,
    Deprecated,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// ISO 4217 currency code
    pub currency: String,

    /// Input cost per million tokens
    pub input_cost_per_1m_tokens: f64,

    /// Output cost per million tokens
    pub output_cost_per_1m_tokens: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingSupport {
    /// Minimum thinking budget in tokens
    pub min: u32,

    /// Maximum thinking budget in tokens
    pub max: u32,

    /// Whether thinking can be disabled (set to 0)
    pub zero_allowed: bool,

    /// Whether -1 (auto) is allowed
    pub dynamic_allowed: bool,

    /// Discrete thinking levels ("low", "medium", "high")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<Vec<String>>,
}

impl ModelInfo {
    pub fn new(id: String, provider: LlmProviderType, owned_by: String) -> Self {
        Self {
            id,
            display_name: String::new(),
            name: None,
            description: None,
            provider,
            object: "model".to_string(),
            created: Utc::now(),
            owned_by,
            input_token_limit: None,
            output_token_limit: None,
            context_window: None,
            max_completion_tokens: None,
            supports_vision: false,
            supports_function_calling: false,
            supports_streaming: true,
            status: ModelStatus::Active,
            pricing: None,
            thinking: None,
            supported_generation_methods: None,
            updated_at: Utc::now(),
        }
    }

    /// Check if model is available
    pub fn is_available(&self) -> bool {
        matches!(self.status, ModelStatus::Active | ModelStatus::Beta)
    }

    /// Check if model is deprecated
    pub fn is_deprecated(&self) -> bool {
        matches!(self.status, ModelStatus::Deprecated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_creation() {
        let model = ModelInfo::new(
            "gpt-4-turbo".to_string(),
            LlmProviderType::OpenAI,
            "openai".to_string(),
        );
        assert_eq!(model.id, "gpt-4-turbo");
        assert!(model.is_available());
    }

    #[test]
    fn test_model_status() {
        let mut model = ModelInfo::new(
            "gpt-3.5-turbo".to_string(),
            LlmProviderType::OpenAI,
            "openai".to_string(),
        );
        model.status = ModelStatus::Deprecated;
        assert!(model.is_deprecated());
        assert!(!model.is_available());
    }
}
