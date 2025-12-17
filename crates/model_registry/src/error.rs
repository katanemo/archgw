use thiserror::Error;

pub type Result<T> = std::result::Result<T, ModelRegistryError>;

#[derive(Error, Debug)]
pub enum ModelRegistryError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("No available providers for model: {0}")]
    NoAvailableProviders(String),

    #[error("Client not found: {0}")]
    ClientNotFound(String),

    #[error("Provider API error: {0}")]
    ProviderApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid model metadata: {0}")]
    InvalidMetadata(String),

    #[error("Model discovery timeout for {0}")]
    DiscoveryTimeout(String),

    #[error("Quota exceeded for client: {0}")]
    QuotaExceeded(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
