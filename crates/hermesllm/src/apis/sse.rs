use crate::providers::response::ProviderStreamResponse;
use crate::providers::response::ProviderStreamResponseType;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

// ============================================================================
// SSE EVENT CONTAINER
// ============================================================================

/// Represents a single Server-Sent Event with the complete wire format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(rename = "data")]
    pub data: Option<String>, // The JSON payload after "data: "

    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>, // Optional event type (e.g., "message_start", "content_block_delta")

    #[serde(skip_serializing, skip_deserializing)]
    pub raw_line: String, // The complete line as received including "data: " prefix and "\n\n"

    #[serde(skip_serializing, skip_deserializing)]
    pub sse_transform_buffer: String, // The complete line as received including "data: " prefix and "\n\n"

    #[serde(skip_serializing, skip_deserializing)]
    pub provider_stream_response: Option<ProviderStreamResponseType>, // Parsed provider stream response object
}

impl SseEvent {
    /// Check if this event represents the end of the stream
    pub fn is_done(&self) -> bool {
        self.data == Some("[DONE]".into())
    }

    /// Check if this event should be skipped during processing
    /// This includes ping messages and other provider-specific events that don't contain content
    pub fn should_skip(&self) -> bool {
        // Skip ping messages (commonly used by providers for connection keep-alive)
        self.data == Some(r#"{"type": "ping"}"#.into())
    }

    /// Check if this is an event-only SSE event (no data payload)
    pub fn is_event_only(&self) -> bool {
        self.event.is_some() && self.data.is_none()
    }

    /// Get the parsed provider response if available
    pub fn provider_response(&self) -> Result<&dyn ProviderStreamResponse, std::io::Error> {
        self.provider_stream_response
            .as_ref()
            .map(|resp| resp as &dyn ProviderStreamResponse)
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Provider response not found")
            })
    }
}

impl FromStr for SseEvent {
    type Err = SseParseError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        if line.starts_with("data: ") {
            let data: String = line[6..].to_string(); // Remove "data: " prefix
            if data.is_empty() {
                return Err(SseParseError {
                    message: "Empty data field is not a valid SSE event".to_string(),
                });
            }
            Ok(SseEvent {
                data: Some(data),
                event: None,
                raw_line: line.to_string(),
                sse_transform_buffer: line.to_string(),
                provider_stream_response: None,
            })
        } else if line.starts_with("event: ") {
            //used by Anthropic
            let event_type = line[7..].to_string();
            if event_type.is_empty() {
                return Err(SseParseError {
                    message: "Empty event field is not a valid SSE event".to_string(),
                });
            }
            Ok(SseEvent {
                data: None,
                event: Some(event_type),
                raw_line: line.to_string(),
                sse_transform_buffer: line.to_string(),
                provider_stream_response: None,
            })
        } else {
            Err(SseParseError {
                message: format!("Line does not start with 'data: ' or 'event: ': {}", line),
            })
        }
    }
}

impl fmt::Display for SseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sse_transform_buffer)
    }
}

// Into implementation to convert SseEvent to bytes for response buffer
impl Into<Vec<u8>> for SseEvent {
    fn into(self) -> Vec<u8> {
        format!("{}\n\n", self.sse_transform_buffer).into_bytes()
    }
}

#[derive(Debug)]
pub struct SseParseError {
    pub message: String,
}

impl fmt::Display for SseParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SSE parse error: {}", self.message)
    }
}

impl Error for SseParseError {}

/// Generic SSE (Server-Sent Events) streaming iterator container
/// Parses raw SSE lines into SseEvent objects
pub struct SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub lines: I,
    pub done_seen: bool,
}

impl<I> SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub fn new(lines: I) -> Self {
        Self {
            lines,
            done_seen: false,
        }
    }
}

// TryFrom implementation to parse bytes into SseStreamIter
// Handles both text-based SSE and binary AWS Event Stream formats
impl TryFrom<&[u8]> for SseStreamIter<std::vec::IntoIter<String>> {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        // Parse as text-based SSE format
        let s = std::str::from_utf8(bytes)?;
        let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
        Ok(SseStreamIter::new(lines.into_iter()))
    }
}

impl<I> Iterator for SseStreamIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    type Item = SseEvent;

    fn next(&mut self) -> Option<Self::Item> {
        // If we already returned [DONE], terminate the stream
        if self.done_seen {
            return None;
        }

        for line in &mut self.lines {
            let line_str = line.as_ref();

            // Try to parse as either data: or event: line
            if let Ok(event) = line_str.parse::<SseEvent>() {
                // For data: lines, check if this is the [DONE] marker
                if event.data.is_some() && event.is_done() {
                    self.done_seen = true;
                    return Some(event); // Return [DONE] event for transformation
                }
                // For data: lines, skip events that should be filtered at the transport layer
                if event.data.is_some() && event.should_skip() {
                    continue;
                }
                return Some(event);
            }
        }
        None
    }
}
