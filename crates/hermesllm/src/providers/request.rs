
use crate::apis::openai::ChatCompletionsRequest;
use super::{ProviderId, get_provider_config, AdapterType};
use std::error::Error;
use std::fmt;
pub enum ProviderRequestType {
    ChatCompletionsRequest(ChatCompletionsRequest),
    //MessagesRequest(MessagesRequest),
    //add more request types here
}

impl TryFrom<&[u8]> for ProviderRequestType {
    type Error = std::io::Error;

    // if passing bytes without provider id we assume the request is in OpenAI format
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let chat_completion_request: ChatCompletionsRequest = ChatCompletionsRequest::try_from(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(ProviderRequestType::ChatCompletionsRequest(chat_completion_request))
    }
}

impl TryFrom<(&[u8], &ProviderId)> for ProviderRequestType {
    type Error = std::io::Error;

    fn try_from((bytes, provider_id): (&[u8], &ProviderId)) -> Result<Self, Self::Error> {
        let config = get_provider_config(provider_id);
        match config.adapter_type {
            AdapterType::OpenAICompatible => {
                let chat_completion_request: ChatCompletionsRequest = ChatCompletionsRequest::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderRequestType::ChatCompletionsRequest(chat_completion_request))
            }
            // Future: handle other adapter types like Claude
        }
    }
}

pub trait ProviderRequest: Send + Sync {
    /// Extract the model name from the request
    fn model(&self) -> &str;

    /// Set the model name for the request
    fn set_model(&mut self, model: String);

    /// Check if this is a streaming request
    fn is_streaming(&self) -> bool;

    /// Extract text content from messages for token counting
    fn extract_messages_text(&self) -> String;

    /// Extract the user message for tracing/logging purposes
    fn get_recent_user_message(&self) -> Option<String>;

    /// Convert the request to bytes for transmission
    fn to_bytes(&self) -> Result<Vec<u8>, ProviderRequestError>;
}

impl ProviderRequest for ProviderRequestType {
    fn model(&self) -> &str {
        match self {
            Self::ChatCompletionsRequest(r) => r.model(),
        }
    }

    fn set_model(&mut self, model: String) {
        match self {
            Self::ChatCompletionsRequest(r) => r.set_model(model),
        }
    }

    fn is_streaming(&self) -> bool {
        match self {
            Self::ChatCompletionsRequest(r) => r.is_streaming(),
        }
    }

    fn extract_messages_text(&self) -> String {
        match self {
            Self::ChatCompletionsRequest(r) => r.extract_messages_text(),
        }
    }

    fn get_recent_user_message(&self) -> Option<String> {
        match self {
            Self::ChatCompletionsRequest(r) => r.get_recent_user_message(),
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>, ProviderRequestError> {
        match self {
            Self::ChatCompletionsRequest(r) => r.to_bytes(),
        }
    }
}


/// Error types for provider operations
#[derive(Debug)]
pub struct ProviderRequestError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}

impl fmt::Display for ProviderRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider request error: {}", self.message)
    }
}

impl Error for ProviderRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}
