use std::error::Error;
use std::fmt;

use crate::apis::openai::ChatCompletionsResponse;
use crate::apis::openai::ChatCompletionsStreamResponse;
use crate::providers::id::ProviderId;
use crate::providers::adapters::{get_provider_config, AdapterType};

pub enum ProviderResponseType {
    ChatCompletionsResponse(ChatCompletionsResponse),
    //MessagesResponse(MessagesResponse),
}

pub enum ProviderStreamResponseType {
    ChatCompletionsStreamResponse(ChatCompletionsStreamResponse),
    //MessagesStreamResponse(MessagesStreamMessage),
}

impl TryFrom<(&[u8], ProviderId)> for ProviderResponseType {
    type Error = std::io::Error;

    fn try_from((bytes, provider_id): (&[u8], ProviderId)) -> Result<Self, Self::Error> {
        let config = get_provider_config(&provider_id);
        match config.adapter_type {
            AdapterType::OpenAICompatible => {
                let chat_completions_response: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(chat_completions_response))
            }
            // Future: handle other adapter types like Claude
        }
    }
}
pub trait ProviderResponse: Send + Sync {
    /// Get usage information if available - returns dynamic trait object
    fn usage(&self) -> Option<&dyn TokenUsage>;

    /// Extract token counts for metrics
    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        self.usage().map(|u| (u.prompt_tokens(), u.completion_tokens(), u.total_tokens()))
    }
}

pub trait ProviderStreamResponse: Send + Sync {
    /// Get the content delta for this chunk
    fn content_delta(&self) -> Option<&str>;

    /// Check if this is the final chunk in the stream
    fn is_final(&self) -> bool;

    /// Get role information if available
    fn role(&self) -> Option<&str>;
}

/// Trait for streaming response iterators
pub trait ProviderStreamResponseIter: Iterator<Item = Result<Box<dyn ProviderStreamResponse>, Box<dyn std::error::Error + Send + Sync>>> + Send + Sync {

}


impl ProviderResponse for ProviderResponseType {
    fn usage(&self) -> Option<&dyn TokenUsage> {
        match self {
            ProviderResponseType::ChatCompletionsResponse(resp) => resp.usage(),
            // Future: ProviderResponseType::MessagesResponse(resp) => resp.usage(),
        }
    }

    fn extract_usage_counts(&self) -> Option<(usize, usize, usize)> {
        match self {
            ProviderResponseType::ChatCompletionsResponse(resp) => resp.extract_usage_counts(),
            // Future: ProviderResponseType::MessagesResponse(resp) => resp.extract_usage_counts(),
        }
    }
}

impl ProviderStreamResponse for ProviderStreamResponseType {
    fn content_delta(&self) -> Option<&str> {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.content_delta(),
            // Future: ProviderStreamResponseType::MessagesStreamResponse(resp) => resp.content_delta(),
        }
    }

    fn is_final(&self) -> bool {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.is_final(),
            // Future: ProviderStreamResponseType::MessagesStreamResponse(resp) => resp.is_final(),
        }
    }

    fn role(&self) -> Option<&str> {
        match self {
            ProviderStreamResponseType::ChatCompletionsStreamResponse(resp) => resp.role(),
            // Future: ProviderStreamResponseType::MessagesStreamResponse(resp) => resp.role(),
        }
    }
}

/// Trait for token usage information
pub trait TokenUsage {
    fn completion_tokens(&self) -> usize;
    fn prompt_tokens(&self) -> usize;
    fn total_tokens(&self) -> usize;
}


#[derive(Debug)]
pub struct ProviderResponseError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}


impl fmt::Display for ProviderResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider response error: {}", self.message)
    }
}

impl Error for ProviderResponseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

/// Create streaming response using provider ID - returns clean ProviderStreamResponseIter trait object
pub fn try_streaming_from_bytes(bytes: &[u8], provider_id: &ProviderId) -> Result<Box<dyn ProviderStreamResponseIter>, Box<dyn std::error::Error + Send + Sync>> {
    let config = get_provider_config(provider_id);

    match config.adapter_type {
        AdapterType::OpenAICompatible => {
            // Parse SSE (Server-Sent Events) streaming data
            let s = std::str::from_utf8(bytes)?;
            let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
            let iter = crate::apis::openai::SseChatCompletionIter::new(lines.into_iter());

            // Return the iterator directly - it implements ProviderStreamResponseIter
            Ok(Box::new(iter))
        }
    }
}
