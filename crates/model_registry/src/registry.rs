use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use chrono::{DateTime, Duration, Utc};
use crate::model_info::ModelInfo;
use crate::{ModelRegistryError, Result};
use common::configuration::LlmProviderType;

/// Registration information for a single model
#[derive(Debug, Clone)]
pub struct ModelRegistration {
    /// Model information
    pub info: ModelInfo,

    /// Number of active clients providing this model
    pub count: usize,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,

    /// Clients with exceeded quota (with timestamp for 5-minute cooldown)
    pub quota_exceeded_clients: HashMap<String, DateTime<Utc>>,

    /// Suspended clients with reasons (error, maintenance, etc.)
    pub suspended_clients: HashMap<String, String>,

    /// Provider distribution (provider name -> count)
    pub providers: HashMap<String, usize>,
}

impl ModelRegistration {
    fn new(info: ModelInfo) -> Self {
        Self {
            info,
            count: 0,
            last_updated: Utc::now(),
            quota_exceeded_clients: HashMap::new(),
            suspended_clients: HashMap::new(),
            providers: HashMap::new(),
        }
    }

    /// Get effective available clients (accounting for quota and suspension)
    pub fn effective_clients(&self) -> usize {
        let now = Utc::now();
        let mut available = self.count;

        // Subtract active quota exceeded clients (within 5-minute window)
        for quota_time in self.quota_exceeded_clients.values() {
            if now.signed_duration_since(*quota_time) < Duration::minutes(5) {
                available = available.saturating_sub(1);
            }
        }

        // Subtract suspended clients
        available = available.saturating_sub(self.suspended_clients.len());

        available
    }

    /// Check if model is still available
    pub fn is_available(&self) -> bool {
        let effective = self.effective_clients();
        if effective > 0 {
            return true;
        }

        // With quota cooldown, might recover soon
        let now = Utc::now();
        for quota_time in self.quota_exceeded_clients.values() {
            if now.signed_duration_since(*quota_time) < Duration::minutes(5) {
                return true; // Still in cooldown period, might recover
            }
        }

        false
    }
}

/// Thread-safe model registry following CLIProxyAPI patterns
pub struct ModelRegistry {
    /// Model ID -> Registration
    models: RwLock<HashMap<String, ModelRegistration>>,

    /// Client ID -> Model IDs
    client_models: RwLock<HashMap<String, HashSet<String>>>,

    /// Client ID -> Provider type
    client_providers: RwLock<HashMap<String, LlmProviderType>>,

    /// Model ID -> Provider -> Client IDs
    model_providers: RwLock<HashMap<String, HashMap<String, HashSet<String>>>>,
}

impl ModelRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            client_models: RwLock::new(HashMap::new()),
            client_providers: RwLock::new(HashMap::new()),
            model_providers: RwLock::new(HashMap::new()),
        }
    }

    /// Register or update a model
    pub fn register_model(&self, model: ModelInfo) -> Result<()> {
        let mut models = self.models.write();
        models.insert(
            model.id.clone(),
            ModelRegistration::new(model.clone()),
        );
        Ok(())
    }

    /// Register multiple models
    pub fn register_models(&self, models: Vec<ModelInfo>) -> Result<()> {
        for model in models {
            self.register_model(model)?;
        }
        Ok(())
    }

    /// Associate a client with models they provide
    pub fn register_client(
        &self,
        client_id: &str,
        provider: LlmProviderType,
        model_ids: Vec<String>,
    ) -> Result<()> {
        let mut models = self.models.write();
        let mut client_models = self.client_models.write();
        let mut client_providers = self.client_providers.write();
        let mut model_providers = self.model_providers.write();

        // Register client provider association
        client_providers.insert(client_id.to_string(), provider.clone());

        // Register each model
        for model_id in model_ids {
            // Ensure model exists
            if !models.contains_key(&model_id) {
                return Err(ModelRegistryError::ModelNotFound(model_id));
            }

            // Update model registration
            if let Some(registration) = models.get_mut(&model_id) {
                registration.count += 1;
                registration.last_updated = Utc::now();

                let provider_name = provider.to_string();
                registration
                    .providers
                    .entry(provider_name.clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);

                // Track provider distribution
                model_providers
                    .entry(model_id.clone())
                    .or_insert_with(HashMap::new)
                    .entry(provider_name)
                    .or_insert_with(HashSet::new)
                    .insert(client_id.to_string());
            }

            // Add to client's model list
            client_models
                .entry(client_id.to_string())
                .or_insert_with(HashSet::new)
                .insert(model_id);
        }

        Ok(())
    }

    /// Get a model by ID
    pub fn get_model(&self, model_id: &str) -> Result<ModelInfo> {
        let models = self.models.read();
        models
            .get(model_id)
            .map(|r| r.info.clone())
            .ok_or_else(|| ModelRegistryError::ModelNotFound(model_id.to_string()))
    }

    /// Get all models
    pub fn get_all_models(&self) -> Vec<ModelInfo> {
        let models = self.models.read();
        models.values().map(|r| r.info.clone()).collect()
    }

    /// Get available models (active, with providers)
    pub fn get_available_models(&self) -> Vec<ModelInfo> {
        let models = self.models.read();
        models
            .values()
            .filter(|r| r.is_available() && r.info.is_available())
            .map(|r| r.info.clone())
            .collect()
    }

    /// Get models by provider
    pub fn get_models_by_provider(&self, provider: LlmProviderType) -> Vec<ModelInfo> {
        let models = self.models.read();
        models
            .values()
            .filter(|r| r.info.provider == provider && r.info.is_available())
            .map(|r| r.info.clone())
            .collect()
    }

    /// Get providers for a model (ordered by availability)
    pub fn get_model_providers(&self, model_id: &str) -> Result<Vec<String>> {
        let models = self.models.read();
        let registration = models
            .get(model_id)
            .ok_or_else(|| ModelRegistryError::ModelNotFound(model_id.to_string()))?;

        // Sort providers by availability (descending)
        let mut providers: Vec<_> = registration.providers.iter().collect();
        providers.sort_by(|a, b| b.1.cmp(a.1));

        Ok(providers.into_iter().map(|(p, _)| p.clone()).collect())
    }

    /// Mark model quota exceeded for client (5-minute cooldown)
    pub fn set_model_quota_exceeded(&self, client_id: &str, model_id: &str) -> Result<()> {
        let mut models = self.models.write();
        let registration = models
            .get_mut(model_id)
            .ok_or_else(|| ModelRegistryError::ModelNotFound(model_id.to_string()))?;

        registration
            .quota_exceeded_clients
            .insert(client_id.to_string(), Utc::now());

        Ok(())
    }

    /// Suspend client model with reason
    pub fn suspend_client_model(
        &self,
        client_id: &str,
        model_id: &str,
        reason: &str,
    ) -> Result<()> {
        let mut models = self.models.write();
        let registration = models
            .get_mut(model_id)
            .ok_or_else(|| ModelRegistryError::ModelNotFound(model_id.to_string()))?;

        registration
            .suspended_clients
            .insert(client_id.to_string(), reason.to_string());

        Ok(())
    }

    /// Resume client model (clear suspension)
    pub fn resume_client_model(&self, client_id: &str, model_id: &str) -> Result<()> {
        let mut models = self.models.write();
        let registration = models
            .get_mut(model_id)
            .ok_or_else(|| ModelRegistryError::ModelNotFound(model_id.to_string()))?;

        registration.suspended_clients.remove(client_id);

        Ok(())
    }

    /// Unregister client (remove from all models)
    pub fn unregister_client(&self, client_id: &str) -> Result<()> {
        let mut models = self.models.write();
        let mut client_models = self.client_models.write();
        let mut client_providers = self.client_providers.write();

        if let Some(model_ids) = client_models.remove(client_id) {
            for model_id in model_ids {
                if let Some(registration) = models.get_mut(&model_id) {
                    registration.count = registration.count.saturating_sub(1);
                    registration.quota_exceeded_clients.remove(client_id);
                    registration.suspended_clients.remove(client_id);
                }
            }
        }

        client_providers.remove(client_id);

        Ok(())
    }

    /// Get stats about the registry
    pub fn get_stats(&self) -> RegistryStats {
        let models = self.models.read();
        let client_models = self.client_models.read();

        let total_models = models.len();
        let available_models = models.values().filter(|r| r.is_available()).count();
        let total_clients = client_models.len();

        let providers_count: HashSet<_> = models
            .values()
            .flat_map(|r| r.providers.keys().cloned())
            .collect();

        RegistryStats {
            total_models,
            available_models,
            total_clients,
            unique_providers: providers_count.len(),
        }
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_models: usize,
    pub available_models: usize,
    pub total_clients: usize,
    pub unique_providers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_model() {
        let registry = ModelRegistry::new();
        let model = ModelInfo::new(
            "gpt-4".to_string(),
            LlmProviderType::OpenAI,
            "openai".to_string(),
        );
        assert!(registry.register_model(model).is_ok());
        assert!(registry.get_model("gpt-4").is_ok());
    }

    #[test]
    fn test_register_client() {
        let registry = ModelRegistry::new();
        let model = ModelInfo::new(
            "gpt-4".to_string(),
            LlmProviderType::OpenAI,
            "openai".to_string(),
        );
        registry.register_model(model).unwrap();

        assert!(registry
            .register_client("client1", LlmProviderType::OpenAI, vec!["gpt-4".to_string()])
            .is_ok());

        let providers = registry.get_model_providers("gpt-4").unwrap();
        assert!(providers.contains(&"openai".to_string()));
    }

    #[test]
    fn test_quota_cooldown() {
        let registry = ModelRegistry::new();
        let model = ModelInfo::new(
            "gpt-4".to_string(),
            LlmProviderType::OpenAI,
            "openai".to_string(),
        );
        registry.register_model(model).unwrap();
        registry
            .register_client("client1", LlmProviderType::OpenAI, vec!["gpt-4".to_string()])
            .unwrap();

        registry
            .set_model_quota_exceeded("client1", "gpt-4")
            .unwrap();

        let models = registry.models.read();
        let registration = models.get("gpt-4").unwrap();
        assert!(registration.quota_exceeded_clients.contains_key("client1"));
    }
}
