use hermesllm::providers::openai::types::{ChatCompletionsRequest, Message};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoutingModelError {
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RoutingModelError>;

pub trait RouterModel: Send + Sync {
    fn generate_request(&self, messages: &[Message]) -> ChatCompletionsRequest;
    fn parse_response(&self, content: &str) -> Result<Option<String>>;
    fn get_model_name(&self) -> String;
}
