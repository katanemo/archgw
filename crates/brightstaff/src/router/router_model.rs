use common::configuration::ModelUsagePreference;
use hermesllm::apis::openai::{ChatCompletionsRequest, Message};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoutingModelError {
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RoutingModelError>;

pub trait RouterModel: Send + Sync {
    fn generate_request(
        &self,
        messages: &[Message],
        usage_preferences: &Option<Vec<ModelUsagePreference>>,
    ) -> ChatCompletionsRequest;
    fn parse_response(
        &self,
        content: &str,
        usage_preferences: &Option<Vec<ModelUsagePreference>>,
    ) -> Result<Option<(String, String)>>;
    fn get_model_name(&self) -> String;
}
