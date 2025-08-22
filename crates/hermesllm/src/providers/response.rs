use crate::providers::id::ProviderId;

use serde::Serialize;
use std::error::Error;
use std::fmt;

use crate::apis::openai::ChatCompletionsResponse;
use crate::apis::OpenAISseIter;
use crate::clients::endpoints::SupportedApi;
use std::convert::TryFrom;

#[derive(Serialize)]
pub enum ProviderResponseType {
    ChatCompletionsResponse(ChatCompletionsResponse),
    //MessagesResponse(MessagesResponse),
}

pub enum ProviderStreamResponseIter {
    ChatCompletionsStream(OpenAISseIter<std::vec::IntoIter<String>>),
    //MessagesStream(AnthropicSseIter<std::vec::IntoIter<String>>),
}


// --- Response transformation logic for client API compatibility ---
impl TryFrom<(&[u8], &SupportedApi, &ProviderId)> for ProviderResponseType {
    type Error = std::io::Error;

    fn try_from((bytes, client_api, provider_id): (&[u8], &SupportedApi, &ProviderId)) -> Result<Self, Self::Error> {
        let upstream_api = provider_id.compatible_api_for_client(client_api);
        match (&upstream_api, client_api) {
            (SupportedApi::OpenAI(_), SupportedApi::OpenAI(_)) => {
                let resp: ChatCompletionsResponse = ChatCompletionsResponse::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderResponseType::ChatCompletionsResponse(resp))
            }
            (SupportedApi::OpenAI(_), SupportedApi::Anthropic(_)) => {
                // If you add a MessagesResponse variant, return it here. For now, just error or serialize as needed.
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Anthropic response variant not implemented"))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Unsupported response transformation")),
        }
    }
}

impl TryFrom<(&[u8], &SupportedApi, &ProviderId)> for ProviderStreamResponseIter {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from((bytes, client_api, provider_id): (&[u8], &SupportedApi, &ProviderId)) -> Result<Self, Self::Error> {
        let upstream_api = provider_id.compatible_api_for_client(client_api);
        match (&upstream_api, client_api) {
            (SupportedApi::OpenAI(_), SupportedApi::OpenAI(_)) => {
                let s = std::str::from_utf8(bytes)?;
                let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
                let sse_container = crate::providers::response::SseStreamIter::new(lines.into_iter());
                let iter = crate::apis::openai::OpenAISseIter::new(sse_container);
                Ok(ProviderStreamResponseIter::ChatCompletionsStream(iter))
            }
            (SupportedApi::OpenAI(_), SupportedApi::Anthropic(_)) => {
                // TODO: Implement streaming transformation from OpenAI to Anthropic
                Err("Anthropic streaming response variant not implemented".into())
            }
            _ => Err("Unsupported streaming response transformation".into()),
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
