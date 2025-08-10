//! Gemini provider implementation
//!
//! Gemini will use a different API format in the future
//! For now, fallback to OpenAI-compatible format

pub mod provider;
pub use provider::GeminiProvider;
