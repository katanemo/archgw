use crate::{ModelInfo, Result, ModelRegistryError};
use common::configuration::LlmProviderType;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;

/// Trait for discovering models from LLM providers
#[async_trait::async_trait]
pub trait ModelDiscovery: Send + Sync {
    /// List available models from the provider
    async fn list_available_models(&self) -> Result<Vec<ModelInfo>>;

    /// Get a specific model by ID
    async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>>;

    /// Get the provider type this discovery service represents
    fn provider_type(&self) -> LlmProviderType;

    /// Get human-readable provider name
    fn provider_name(&self) -> &str;
}

/// OpenAI models discovery via `/v1/models` API
pub struct OpenAIDiscovery {
    api_key: String,
    base_url: String,
}

impl OpenAIDiscovery {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com".to_string(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

#[async_trait::async_trait]
impl ModelDiscovery for OpenAIDiscovery {
    async fn list_available_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/v1/models", self.base_url);
        let client = reqwest::Client::new();

        let response = tokio::time::timeout(
            Duration::from_secs(10),
            client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send(),
        )
        .await
        .map_err(|_| ModelRegistryError::DiscoveryTimeout("OpenAI".to_string()))?
        .map_err(|e| ModelRegistryError::ConfigError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ModelRegistryError::ProviderNotFound(
                "OpenAI API returned error".to_string(),
            ));
        }

        let body = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| ModelRegistryError::ConfigError(e.to_string()))?;

        let models = body
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ModelRegistryError::InvalidMetadata("Invalid OpenAI response".to_string()))?;

        let mut result = Vec::new();
        for model_data in models {
            if let Some(id) = model_data.get("id").and_then(|v| v.as_str()) {
                // Filter to relevant models (GPT models)
                if id.contains("gpt") {
                    let model = ModelInfo::new(
                        id.to_string(),
                        LlmProviderType::OpenAI,
                        "openai".to_string(),
                    );
                    result.push(model);
                }
            }
        }

        Ok(result)
    }

    async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
        let models = self.list_available_models().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::OpenAI
    }

    fn provider_name(&self) -> &str {
        "OpenAI"
    }
}

/// Anthropic models discovery (via static list, as Anthropic doesn't expose a models API)
pub struct AnthropicDiscovery;

#[async_trait::async_trait]
impl ModelDiscovery for AnthropicDiscovery {
    async fn list_available_models(&self) -> Result<Vec<ModelInfo>> {
        // Anthropic doesn't provide a public models API, so we use known models
        Ok(vec![
            ModelInfo::new(
                "claude-opus-4-5-20251101".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            ),
            ModelInfo::new(
                "claude-sonnet-4-20250514".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            ),
            ModelInfo::new(
                "claude-haiku-3-5-20241022".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            ),
        ])
    }

    async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
        let models = self.list_available_models().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::Anthropic
    }

    fn provider_name(&self) -> &str {
        "Anthropic"
    }
}

/// Google Gemini models discovery via API
pub struct GeminiDiscovery {
    api_key: String,
    base_url: String,
}

impl GeminiDiscovery {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

#[async_trait::async_trait]
impl ModelDiscovery for GeminiDiscovery {
    async fn list_available_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}?key={}", self.base_url, self.api_key);
        let client = reqwest::Client::new();

        let response = tokio::time::timeout(
            Duration::from_secs(10),
            client.get(&url).send(),
        )
        .await
        .map_err(|_| ModelRegistryError::DiscoveryTimeout("Gemini".to_string()))?
        .map_err(|e| ModelRegistryError::ConfigError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ModelRegistryError::ProviderNotFound(
                "Gemini API returned error".to_string(),
            ));
        }

        let body = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| ModelRegistryError::ConfigError(e.to_string()))?;

        let models = body
            .get("models")
            .and_then(|m| m.as_array())
            .ok_or_else(|| ModelRegistryError::InvalidMetadata("Invalid Gemini response".to_string()))?;

        let mut result = Vec::new();
        for model_data in models {
            if let Some(name) = model_data.get("name").and_then(|v| v.as_str()) {
                // Extract model ID from "models/gemini-2.5-pro"
                if let Some(id) = name.strip_prefix("models/") {
                    let mut model = ModelInfo::new(
                        id.to_string(),
                        LlmProviderType::Gemini,
                        "google".to_string(),
                    );
                    model.name = Some(name.to_string());
                    result.push(model);
                }
            }
        }

        Ok(result)
    }

    async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
        let models = self.list_available_models().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::Gemini
    }

    fn provider_name(&self) -> &str {
        "Gemini"
    }
}

/// Groq models discovery (via static list, as Groq API is limited)
pub struct GroqDiscovery;

#[async_trait::async_trait]
impl ModelDiscovery for GroqDiscovery {
    async fn list_available_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo::new(
                "mixtral-8x7b-32768".to_string(),
                LlmProviderType::Groq,
                "groq".to_string(),
            ),
            ModelInfo::new(
                "llama2-70b-4096".to_string(),
                LlmProviderType::Groq,
                "groq".to_string(),
            ),
        ])
    }

    async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
        let models = self.list_available_models().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::Groq
    }

    fn provider_name(&self) -> &str {
        "Groq"
    }
}

/// Mistral models discovery (via static list)
pub struct MistralDiscovery;

#[async_trait::async_trait]
impl ModelDiscovery for MistralDiscovery {
    async fn list_available_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo::new(
                "mistral-large-latest".to_string(),
                LlmProviderType::Mistral,
                "mistral".to_string(),
            ),
            ModelInfo::new(
                "mistral-medium-latest".to_string(),
                LlmProviderType::Mistral,
                "mistral".to_string(),
            ),
        ])
    }

    async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
        let models = self.list_available_models().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    fn provider_type(&self) -> LlmProviderType {
        LlmProviderType::Mistral
    }

    fn provider_name(&self) -> &str {
        "Mistral"
    }
}

/// Cached model discovery with TTL
pub struct CachedDiscovery {
    inner: Box<dyn ModelDiscovery>,
    cache: parking_lot::RwLock<Option<(SystemTime, Vec<ModelInfo>)>>,
    ttl: Duration,
}

impl CachedDiscovery {
    pub fn new(discovery: Box<dyn ModelDiscovery>, ttl_secs: u64) -> Self {
        Self {
            inner: discovery,
            cache: parking_lot::RwLock::new(None),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    fn is_cache_valid(&self, cached_at: SystemTime) -> bool {
        cached_at.elapsed().unwrap_or(Duration::MAX) < self.ttl
    }
}

#[async_trait::async_trait]
impl ModelDiscovery for CachedDiscovery {
    async fn list_available_models(&self) -> Result<Vec<ModelInfo>> {
        {
            let cache = self.cache.read();
            if let Some((cached_at, models)) = &*cache {
                if self.is_cache_valid(*cached_at) {
                    return Ok(models.clone());
                }
            }
        }

        let models = self.inner.list_available_models().await?;
        let mut cache = self.cache.write();
        *cache = Some((SystemTime::now(), models.clone()));
        Ok(models)
    }

    async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
        let models = self.list_available_models().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    fn provider_type(&self) -> LlmProviderType {
        self.inner.provider_type()
    }

    fn provider_name(&self) -> &str {
        self.inner.provider_name()
    }
}

/// Discovery manager for all providers
pub struct DiscoveryManager {
    discoveries: HashMap<String, Box<dyn ModelDiscovery>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            discoveries: HashMap::new(),
        }
    }

    pub fn add_discovery(&mut self, name: String, discovery: Box<dyn ModelDiscovery>) {
        self.discoveries.insert(name, discovery);
    }

    pub async fn discover_all(&self) -> Result<Vec<ModelInfo>> {
        let mut all_models = Vec::new();

        for (_, discovery) in &self.discoveries {
            match discovery.list_available_models().await {
                Ok(models) => {
                    tracing::info!(
                        "Discovered {} models from {}",
                        models.len(),
                        discovery.provider_name()
                    );
                    all_models.extend(models);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to discover models from {}: {:?}",
                        discovery.provider_name(),
                        e
                    );
                }
            }
        }

        Ok(all_models)
    }

    pub async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
        for discovery in self.discoveries.values() {
            if let Ok(Some(model)) = discovery.get_model(id).await {
                return Ok(Some(model));
            }
        }
        Ok(None)
    }
}

impl Default for DiscoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDiscovery;

    #[async_trait::async_trait]
    impl ModelDiscovery for MockDiscovery {
        async fn list_available_models(&self) -> Result<Vec<ModelInfo>> {
            Ok(vec![
                ModelInfo::new(
                    "test-model".to_string(),
                    LlmProviderType::OpenAI,
                    "test".to_string(),
                ),
            ])
        }

        async fn get_model(&self, id: &str) -> Result<Option<ModelInfo>> {
            let models = self.list_available_models().await?;
            Ok(models.into_iter().find(|m| m.id == id))
        }

        fn provider_type(&self) -> LlmProviderType {
            LlmProviderType::OpenAI
        }

        fn provider_name(&self) -> &str {
            "Test"
        }
    }

    #[tokio::test]
    async fn test_cached_discovery() {
        let mock = Box::new(MockDiscovery);
        let cached = CachedDiscovery::new(mock, 60);

        let models1 = cached.list_available_models().await.unwrap();
        let models2 = cached.list_available_models().await.unwrap();

        assert_eq!(models1.len(), 1);
        assert_eq!(models2.len(), 1);
        assert_eq!(models1[0].id, "test-model");
    }

    #[tokio::test]
    async fn test_anthropic_discovery() {
        let discovery = AnthropicDiscovery;
        let models = discovery.list_available_models().await.unwrap();

        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id.contains("claude")));
    }

    #[tokio::test]
    async fn test_groq_discovery() {
        let discovery = GroqDiscovery;
        let models = discovery.list_available_models().await.unwrap();

        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id.contains("mixtral")));
    }

    #[tokio::test]
    async fn test_discovery_manager() {
        let mut manager = DiscoveryManager::new();
        manager.add_discovery("test".to_string(), Box::new(MockDiscovery));

        let models = manager.discover_all().await.unwrap();
        assert!(!models.is_empty());
        assert_eq!(models[0].id, "test-model");

        let model = manager.get_model("test-model").await.unwrap();
        assert!(model.is_some());
    }
}
