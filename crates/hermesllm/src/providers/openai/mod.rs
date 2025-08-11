pub mod builder;
pub mod types;

// Re-export the main types and builder functionality
pub use crate::apis::openai::{ChatCompletionsRequest, ChatCompletionsResponse, ChatCompletionsStreamResponse};
pub use builder::*;
pub use types::*;

// Note: The OpenAIProvider struct has been deprecated in favor of the function-based approach in traits.rs
// All provider functionality is now accessed through try_request_from_bytes, try_response_from_bytes, etc.
