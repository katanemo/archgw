use crate::model_info::ModelInfo;
use crate::{ModelRegistryError, Result};
use std::collections::HashMap;

/// Strategy for selecting fallback models
pub trait FallbackStrategy: Send + Sync {
    /// Select a fallback model given the requested model and available options
    fn select_fallback(
        &self,
        requested_model: &str,
        available_models: &[ModelInfo],
    ) -> Option<String>;
}

/// Same provider preference - prefer fallback from same provider
pub struct SameProviderFallback;

impl FallbackStrategy for SameProviderFallback {
    fn select_fallback(
        &self,
        requested_model: &str,
        available_models: &[ModelInfo],
    ) -> Option<String> {
        // Note: In real usage, we'd look up the requested model's provider
        // For now, we use simple heuristics

        // Try exact provider family (e.g., claude-opus -> claude-sonnet)
        let provider_prefix = requested_model.split('-').next()?;

        available_models
            .iter()
            .find(|m| m.id.starts_with(provider_prefix) && m.is_available())
            .map(|m| m.id.clone())
    }
}

/// Capability matching fallback - find model with similar capabilities
pub struct CapabilityMatchFallback;

impl FallbackStrategy for CapabilityMatchFallback {
    fn select_fallback(
        &self,
        _requested_model: &str,
        available_models: &[ModelInfo],
    ) -> Option<String> {
        // Find highest capability model available
        available_models
            .iter()
            .filter(|m| m.is_available())
            .max_by_key(|m| {
                // Score based on capabilities
                let mut score = 0;
                if m.supports_vision {
                    score += 10;
                }
                if m.supports_function_calling {
                    score += 10;
                }
                if m.context_window.unwrap_or(0) > 100_000 {
                    score += 5;
                }
                score
            })
            .map(|m| m.id.clone())
    }
}

/// Cost optimized fallback - choose cheapest equivalent
pub struct CostOptimizedFallback;

impl FallbackStrategy for CostOptimizedFallback {
    fn select_fallback(
        &self,
        _requested_model: &str,
        available_models: &[ModelInfo],
    ) -> Option<String> {
        available_models
            .iter()
            .filter(|m| m.is_available() && m.pricing.is_some())
            .min_by(|a, b| {
                let a_cost = a
                    .pricing
                    .as_ref()
                    .map(|p| p.input_cost_per_1m_tokens + p.output_cost_per_1m_tokens)
                    .unwrap_or(f64::MAX);

                let b_cost = b
                    .pricing
                    .as_ref()
                    .map(|p| p.input_cost_per_1m_tokens + p.output_cost_per_1m_tokens)
                    .unwrap_or(f64::MAX);

                a_cost.partial_cmp(&b_cost).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|m| m.id.clone())
    }
}

/// Model mapping/aliasing configuration
#[derive(Debug, Clone)]
pub struct ModelMapping {
    /// from -> to mapping
    pub mappings: HashMap<String, String>,

    /// Whether mappings are strict (only use if exact provider match) or flexible
    pub strict_mode: bool,
}

impl ModelMapping {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            strict_mode: false,
        }
    }

    /// Add a model mapping
    pub fn add_mapping(&mut self, from: String, to: String) {
        self.mappings.insert(from, to);
    }

    /// Map a model, returning the mapped model or the original
    pub fn map_model(&self, model: &str) -> String {
        self.mappings
            .get(model)
            .cloned()
            .unwrap_or_else(|| model.to_string())
    }

    /// Try to get a mapping
    pub fn get_mapping(&self, model: &str) -> Option<&String> {
        self.mappings.get(model)
    }

    /// Check if a model has a mapping
    pub fn has_mapping(&self, model: &str) -> bool {
        self.mappings.contains_key(model)
    }
}

impl Default for ModelMapping {
    fn default() -> Self {
        Self::new()
    }
}

/// Fallback routing logic following CLIProxyAPI patterns
pub struct FallbackRouter {
    /// Fallback strategy
    strategy: Box<dyn FallbackStrategy>,

    /// Model mappings/aliases
    mapping: ModelMapping,
}

impl FallbackRouter {
    pub fn new(strategy: Box<dyn FallbackStrategy>, mapping: ModelMapping) -> Self {
        Self { strategy, mapping }
    }

    /// Execute layered fallback strategy:
    /// 1. Check if model exists and has available providers
    /// 2. Try model mapping as fallback
    /// 3. Use strategy to select alternative
    pub fn resolve_model(
        &self,
        requested_model: &str,
        available_models: &[ModelInfo],
    ) -> Result<String> {
        // Check if requested model exists and is available
        if let Some(model) = available_models
            .iter()
            .find(|m| m.id == requested_model && m.is_available())
        {
            return Ok(model.id.clone());
        }

        // Try model mapping as fallback
        if let Some(mapped_model) = self.mapping.get_mapping(requested_model) {
            if available_models
                .iter()
                .any(|m| &m.id == mapped_model && m.is_available())
            {
                return Ok(mapped_model.clone());
            }
        }

        // Use strategy to find alternative
        if let Some(fallback) = self.strategy.select_fallback(requested_model, available_models) {
            return Ok(fallback);
        }

        Err(ModelRegistryError::NoAvailableProviders(
            requested_model.to_string(),
        ))
    }
}

impl Default for FallbackRouter {
    fn default() -> Self {
        Self {
            strategy: Box::new(SameProviderFallback),
            mapping: ModelMapping::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::configuration::LlmProviderType;

    fn create_test_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo::new(
                "claude-opus-4-5".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            ),
            ModelInfo::new(
                "claude-sonnet-4".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            ),
            ModelInfo::new(
                "gpt-4".to_string(),
                LlmProviderType::OpenAI,
                "openai".to_string(),
            ),
        ]
    }

    #[test]
    fn test_same_provider_fallback() {
        let strategy = SameProviderFallback;
        let models = create_test_models();

        let fallback = strategy.select_fallback("claude-opus-4-5", &models);
        assert!(fallback.is_some());
        assert!(fallback.unwrap().starts_with("claude"));
    }

    #[test]
    fn test_model_mapping() {
        let mut mapping = ModelMapping::new();
        mapping.add_mapping("claude-old".to_string(), "claude-opus-4-5".to_string());

        assert_eq!(mapping.map_model("claude-old"), "claude-opus-4-5");
        assert_eq!(mapping.map_model("unknown"), "unknown");
    }

    #[test]
    fn test_fallback_router() {
        let mut mapping = ModelMapping::new();
        mapping.add_mapping("claude-old".to_string(), "claude-sonnet-4".to_string());

        let router = FallbackRouter::new(Box::new(SameProviderFallback), mapping);
        let models = create_test_models();

        // Test mapping fallback
        let result = router.resolve_model("claude-old", &models);
        assert_eq!(result.unwrap(), "claude-sonnet-4");

        // Test provider fallback
        let result = router.resolve_model("claude-opus-4-5", &models);
        assert!(result.is_ok());
    }
}
