//! OAuth Gateway Service
//!
//! Provides OAuth authentication for Claude Code and Gemini CLI integration with ArchGW.
//!
//! # Features
//! - Claude (Anthropic) OAuth with PKCE
//! - Gemini CLI (Google Cloud Code Assist) OAuth
//! - Secure token storage with automatic refresh
//! - Cloud Code Assist project ID management
//! - HTTP server with REST API endpoints

pub mod auth;
pub mod server;

pub use auth::{OAuthClient, OAuthConfig, OAuthToken, TokenStore, AuthorizationUrl, PKCEVerifier};
pub use server::{AppState, start_server};

pub mod error {
    use anyhow::Error;
    pub type Result<T> = std::result::Result<T, Error>;
}
