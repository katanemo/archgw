//! Claude provider implementation
//!
//! Claude will use a different API format in the future (/v1/messages)
//! For now, fallback to OpenAI-compatible format

pub mod provider;
pub use provider::ClaudeProvider;
