// OAuth Gateway Server
//
// Standalone OAuth authentication service for Claude Code and Gemini CLI
// Provides REST API endpoints for OAuth authorization flows

use oauth_gateway::{TokenStore, OAuthConfig, start_server};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("OAuth Gateway Server");
    println!();

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1");
    let port = args.get(2)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(54545);

    // Initialize token store
    let token_store = TokenStore::default()?;
    println!("[OK] Token store ready at: ~/.archgw/oauth_tokens.json");
    let existing_tokens = token_store.list_providers();
    if !existing_tokens.is_empty() {
        println!("[OK] Loaded {} existing OAuth tokens", existing_tokens.len());
    }

    // Display OAuth provider information
    let claude_config = OAuthConfig::anthropic();
    println!("[OK] Claude Pro/Max OAuth configured: {}", claude_config.client_id);

    let gemini_config = OAuthConfig::gemini();
    println!("[OK] Gemini CLI OAuth configured: {}", gemini_config.client_id);

    let openai_config = OAuthConfig::openai_codex();
    println!("[OK] ChatGPT Plus/Pro OAuth configured: {}", openai_config.client_id);

    let console_config = OAuthConfig::anthropic_console();
    println!("[OK] Anthropic Console OAuth configured: {}", console_config.client_id);

    println!();
    println!("[INFO] Starting OAuth Gateway server on {}:{}", host, port);
    println!();
    println!("[INFO] Available endpoints:");
    println!("  POST   http://{}:{}/auth/authorize          - Start OAuth authorization", host, port);
    println!("  POST   http://{}:{}/auth/exchange           - Exchange code for tokens", host, port);
    println!("  GET    http://{}:{}/auth/callback           - OAuth callback handler", host, port);
    println!("  GET    http://{}:{}/auth/tokens             - List all tokens", host, port);
    println!("  POST   http://{}:{}/auth/tokens/delete      - Delete a token", host, port);
    println!("  POST   http://{}:{}/auth/tokens/refresh     - Refresh a token", host, port);
    println!();

    // Start the server
    start_server(host, port).await?;

    Ok(())
}
