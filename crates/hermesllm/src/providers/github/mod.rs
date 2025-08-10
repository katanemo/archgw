//! GitHub provider implementation
//!
//! GitHub will use a different API format in the future (/models)
//! For now, fallback to OpenAI-compatible format

pub mod provider;
pub use provider::GitHubProvider;
