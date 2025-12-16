//! OAuth Gateway HTTP Server
//!
//! Provides OAuth authentication endpoints for Claude Code and Gemini CLI integration.
//! Routes HTTP requests to appropriate OAuth handlers.

pub mod oauth_handlers;

use crate::TokenStore;
use axum::{
    routing::{get, post},
    Router as AxumRouter,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

/// Application state shared across OAuth handlers
/// Simplified state containing only what oauth_gateway needs
#[derive(Clone)]
pub struct AppState {
    /// OAuth token store for persisting authentication tokens
    pub token_store: TokenStore,
}

/// Start the OAuth Gateway HTTP server
///
/// Initializes token store and sets up routes for OAuth operations:
/// - POST /auth/authorize - Start OAuth authorization flow
/// - POST /auth/exchange - Exchange authorization code for tokens
/// - GET /auth/callback - OAuth callback handler
/// - GET /auth/tokens - List all stored OAuth tokens
/// - POST /auth/tokens/delete - Delete a specific token
/// - POST /auth/tokens/refresh - Manually refresh a token
///
/// # Arguments
/// * `host` - Server hostname (default: "0.0.0.0")
/// * `port` - Server port (default: 54545)
///
/// # Errors
/// Returns error if token store initialization fails or server binding fails
pub async fn start_server(host: &str, port: u16) -> anyhow::Result<()> {
    // Initialize OAuth token store
    let token_store = TokenStore::default()
        .map_err(|e| anyhow::anyhow!("Failed to initialize token store: {}", e))?;

    let existing_tokens = token_store.list_providers();
    if !existing_tokens.is_empty() {
        info!("[OK] Loaded {} OAuth tokens from storage", existing_tokens.len());
    }

    let state = Arc::new(AppState {
        token_store,
    });

    // Build router with OAuth endpoints
    let app = AxumRouter::new()
        // OAuth endpoints (on /auth prefix)
        .route("/auth/authorize", post(oauth_handlers::oauth_authorize))
        .route("/auth/exchange", post(oauth_handlers::oauth_exchange))
        .route("/auth/callback", get(oauth_handlers::oauth_callback))
        .route("/auth/tokens", get(oauth_handlers::oauth_list_tokens))
        .route("/auth/tokens/delete", post(oauth_handlers::oauth_delete_token))
        .route("/auth/tokens/refresh", post(oauth_handlers::oauth_refresh_token))
        // Legacy paths (for compatibility)
        .route("/api/oauth/authorize", post(oauth_handlers::oauth_authorize))
        .route("/api/oauth/exchange", post(oauth_handlers::oauth_exchange))
        .route("/api/oauth/callback", get(oauth_handlers::oauth_callback))
        .route("/api/oauth/tokens", get(oauth_handlers::oauth_list_tokens))
        .route("/api/oauth/tokens/delete", post(oauth_handlers::oauth_delete_token))
        .route("/api/oauth/tokens/refresh", post(oauth_handlers::oauth_refresh_token))
        .with_state(state);

    // Bind to address
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;

    info!("[INFO] OAuth Gateway listening on {}", addr);
    info!("[INFO] Available endpoints:");
    info!("  POST   /auth/authorize          - Start OAuth authorization flow");
    info!("  POST   /auth/exchange           - Exchange code for tokens");
    info!("  GET    /auth/callback           - OAuth callback handler");
    info!("  GET    /auth/tokens             - List all tokens");
    info!("  POST   /auth/tokens/delete      - Delete a token");
    info!("  POST   /auth/tokens/refresh     - Refresh a token");

    // Start server loop
    axum::serve(listener, app).await?;

    Ok(())
}
