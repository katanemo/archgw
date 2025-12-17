pub mod error;
pub mod model_info;
pub mod registry;
pub mod fallback;
pub mod definitions;
pub mod discovery;

pub use error::{ModelRegistryError, Result};
pub use model_info::ModelInfo;
pub use registry::ModelRegistry;
pub use fallback::FallbackStrategy;
pub use discovery::{ModelDiscovery, OpenAIDiscovery, AnthropicDiscovery, GeminiDiscovery, GroqDiscovery, MistralDiscovery, DiscoveryManager, CachedDiscovery};

use std::sync::{Arc, OnceLock};
use parking_lot::RwLock;

static GLOBAL_REGISTRY: OnceLock<Arc<RwLock<ModelRegistry>>> = OnceLock::new();

pub fn get_global_registry() -> Arc<RwLock<ModelRegistry>> {
    GLOBAL_REGISTRY
        .get_or_init(|| Arc::new(RwLock::new(ModelRegistry::new())))
        .clone()
}

pub fn init_global_registry(registry: ModelRegistry) -> Arc<RwLock<ModelRegistry>> {
    let arc = Arc::new(RwLock::new(registry));
    let _ = GLOBAL_REGISTRY.set(arc.clone());
    arc
}
