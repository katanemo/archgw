use std::error::Error;
use std::fmt;

use crate::apis::openai::ChatCompletionsResponse;
use crate::apis::OpenAISseIter;
use crate::providers::id::ProviderId;
use crate::providers::adapters::{get_provider_config, AdapterType};

pub enum ProviderResponseType {
    ChatCompletionsResponse(ChatCompletionsResponse),
    //MessagesResponse(MessagesResponse),
}

pub enum ProviderStreamResponseIter {
    ChatCompletionsStream(OpenAISseIter<std::vec::IntoIter<String>>),
    //MessagesStream(AnthropicSseIter<std::vec::IntoIter<String>>),
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

impl TryFrom<(&[u8], &ProviderId)> for ProviderStreamResponseIter {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((bytes, provider_id): (&[u8], &ProviderId)) -> Result<Self, Self::Error> {
        let config = get_provider_config(provider_id);

        // Parse SSE (Server-Sent Events) streaming data - protocol layer
        let s = std::str::from_utf8(bytes)?;
        let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();

        match config.adapter_type {
            AdapterType::OpenAICompatible => {
                // Delegate to OpenAI-specific iterator implementation
                let sse_container = SseStreamIter::new(lines.into_iter());
                let iter = crate::apis::openai::OpenAISseIter::new(sse_container);
                Ok(ProviderStreamResponseIter::ChatCompletionsStream(iter))
            }
            // Future: AdapterType::Claude => {
            //     let sse_container = SseStreamIter::new(lines.into_iter());
            //     let iter = crate::apis::anthropic::AnthropicSseIter::new(sse_container);
            //     Ok(ProviderStreamResponseIter::MessagesStream(iter))
            // }
        }
    }
}


impl Iterator for ProviderStreamResponseIter {
    type Item = Result<Box<dyn ProviderStreamResponse>, Box<dyn std::error::Error + Send + Sync>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ProviderStreamResponseIter::ChatCompletionsStream(iter) => iter.next(),
            // Future: ProviderStreamResponseIter::MessagesStream(iter) => iter.next(),
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



// ============================================================================
// GENERIC SSE STREAMING ITERATOR (Container Only)
// ============================================================================

/// Generic SSE (Server-Sent Events) streaming iterator container
/// This is just a simple wrapper - actual Iterator implementation is delegated to provider-specific modules
pub struct SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub lines: I,
}

impl<I> SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub fn new(lines: I) -> Self {
        Self { lines }
    }
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

// Implement Send + Sync for the enum to match the original trait requirements
unsafe impl Send for ProviderStreamResponseIter {}
unsafe impl Sync for ProviderStreamResponseIter {}

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
