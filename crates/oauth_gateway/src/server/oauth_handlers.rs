//! OAuth Authentication Handlers
//!
//! HTTP handlers for OAuth authorization flows supporting:
//! - Claude (Anthropic) OAuth with PKCE
//! - Gemini CLI (Google Cloud Code Assist) OAuth
//! - OpenAI Codex (ChatGPT Plus/Pro) OAuth
//! - Anthropic Console (API key creation) OAuth

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{OAuthClient, OAuthConfig};

use super::AppState;

/// Request to start OAuth authorization flow
#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    /// Type of OAuth flow: "max" (Claude Pro/Max) or "console" (API key creation)
    #[serde(default = "default_oauth_type")]
    pub oauth_type: String,
}

fn default_oauth_type() -> String {
    "max".to_string()
}

/// Response with authorization URL
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizeResponse {
    /// Authorization URL for user to visit
    pub url: String,
    /// PKCE verifier (store this for exchange step)
    pub verifier: String,
    /// Instructions for the user
    pub instructions: String,
}

/// Request to exchange authorization code for tokens
#[derive(Debug, Deserialize)]
pub struct OAuthExchangeRequest {
    /// Authorization code from OAuth callback
    pub code: String,
    /// PKCE verifier from authorize step
    pub verifier: String,
    /// Provider ID to store token under
    pub provider_id: String,
    /// OAuth type (optional, for determining config)
    #[serde(default)]
    pub oauth_type: Option<String>,
}

/// Response after successful token exchange
#[derive(Debug, Serialize)]
pub struct OAuthExchangeResponse {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
    /// Provider ID
    pub provider_id: String,
    /// Token expiration timestamp (ISO 8601)
    pub expires_at: String,
}

/// Token information for listing
#[derive(Debug, Serialize)]
pub struct TokenInfo {
    pub provider_id: String,
    pub expires_at: String,
    pub is_expired: bool,
    pub needs_refresh: bool,
}

/// Get authorization URL
pub async fn oauth_authorize(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OAuthAuthorizeRequest>,
) -> Result<Json<OAuthAuthorizeResponse>, (StatusCode, String)> {
    // Create OAuth config based on type
    let config = match req.oauth_type.as_str() {
        "max" => OAuthConfig::anthropic(),
        "console" => OAuthConfig::anthropic_console(),
        "openai-codex" => OAuthConfig::openai_codex(),
        "gemini" => OAuthConfig::gemini(),
        _ => return Err((
            StatusCode::BAD_REQUEST,
            "Invalid oauth_type. Must be 'max', 'console', 'openai-codex', or 'gemini'".to_string()
        )),
    };

    let oauth_client = OAuthClient::new(config, state.token_store.clone());
    let auth_url = oauth_client.get_authorization_url();

    let instructions = match req.oauth_type.as_str() {
        "max" => "Visit the URL above to authorize with your Claude Pro/Max account. After authorization, you'll receive a code. Paste it in the next step.".to_string(),
        "console" => "Visit the URL above to authorize and create an API key. After authorization, you'll receive a code. Paste it in the next step.".to_string(),
        "openai-codex" => "Visit the URL above to authorize with your ChatGPT Plus/Pro account. After authorization, you'll receive a code. Paste it in the next step.".to_string(),
        "gemini" => "Visit the URL above to authorize with your Google account (AI Pro/Ultra). After authorization, you'll receive a code. Paste it in the next step.".to_string(),
        _ => String::new(),
    };

    Ok(Json(OAuthAuthorizeResponse {
        url: auth_url.url,
        verifier: auth_url.verifier.verifier,
        instructions,
    }))
}

/// Exchange authorization code for tokens
pub async fn oauth_exchange(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OAuthExchangeRequest>,
) -> Result<Json<OAuthExchangeResponse>, (StatusCode, String)> {
    tracing::info!("[INFO] OAuth exchange request: provider_id={}, oauth_type={:?}",
        req.provider_id, req.oauth_type);

    // Determine OAuth config based on oauth_type if provided, otherwise fall back to provider_id
    let config = if let Some(ref oauth_type) = req.oauth_type {
        match oauth_type.as_str() {
            "openai-codex" => OAuthConfig::openai_codex(),
            "gemini" => OAuthConfig::gemini(),
            "console" => OAuthConfig::anthropic_console(),
            "max" => OAuthConfig::anthropic(),
            _ => return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid oauth_type: {}", oauth_type)
            )),
        }
    } else if req.provider_id.to_lowercase().contains("openai") ||
              req.provider_id.to_lowercase().contains("codex") ||
              req.provider_id.to_lowercase().contains("chatgpt") {
        OAuthConfig::openai_codex()
    } else {
        OAuthConfig::anthropic()
    };

    let oauth_client = OAuthClient::new(config.clone(), state.token_store.clone());

    // Exchange code for tokens
    let mut token = oauth_client
        .exchange_code(&req.code, &req.verifier, &req.provider_id)
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to exchange code: {}", e)
        ))?;

    // For Gemini providers, call loadCodeAssist to get project ID
    let is_gemini = req.oauth_type.as_deref() == Some("gemini") ||
                    req.provider_id.to_lowercase().contains("gemini") ||
                    req.provider_id.to_lowercase().contains("google");

    tracing::info!("[INFO] Checking if Gemini provider: is_gemini={}, oauth_type={:?}, provider_id={}",
        is_gemini, req.oauth_type, req.provider_id);

    if is_gemini {
        tracing::info!("[INFO] Gemini provider detected, calling loadCodeAssist to get project ID");

        match oauth_client.load_code_assist(&token.access_token).await {
            Ok(project_id) => {
                tracing::info!("[OK] Got project ID from loadCodeAssist: {}", project_id);
                token.project_id = Some(project_id);
                // Save updated token with project_id
                state.token_store.save(token.clone())
                    .map_err(|e| (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to save token with project_id: {}", e)
                    ))?;
            }
            Err(e) => {
                tracing::warn!("[WARN] No project ID available: {}", e);
                tracing::info!("[INFO] Individual Google accounts don't need project ID.");
                tracing::info!("[INFO] Workspace/licensed users: set GOOGLE_CLOUD_PROJECT env var.");
                // Continue without project_id - it's optional for individual accounts
            }
        }
    }

    Ok(Json(OAuthExchangeResponse {
        success: true,
        message: "OAuth authentication successful! Token saved.".to_string(),
        provider_id: req.provider_id,
        expires_at: token.expires_at.to_rfc3339(),
    }))
}

/// List all OAuth tokens
pub async fn oauth_list_tokens(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TokenInfo>>, (StatusCode, String)> {
    let all_tokens = state.token_store.all();

    let token_infos: Vec<TokenInfo> = all_tokens
        .into_iter()
        .map(|(_, token)| TokenInfo {
            provider_id: token.provider_id.clone(),
            expires_at: token.expires_at.to_rfc3339(),
            is_expired: token.is_expired(),
            needs_refresh: token.needs_refresh(),
        })
        .collect();

    Ok(Json(token_infos))
}

/// Delete OAuth token
#[derive(Debug, Deserialize)]
pub struct DeleteTokenRequest {
    pub provider_id: String,
}

pub async fn oauth_delete_token(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteTokenRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.token_store
        .remove(&req.provider_id)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete token: {}", e)
        ))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Token for '{}' deleted", req.provider_id),
    })))
}

/// Refresh a token manually (for testing/debugging)
pub async fn oauth_refresh_token(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteTokenRequest>,
) -> Result<Json<OAuthExchangeResponse>, (StatusCode, String)> {
    // Determine OAuth config based on provider_id
    let config = if req.provider_id.to_lowercase().contains("openai") ||
                     req.provider_id.to_lowercase().contains("codex") ||
                     req.provider_id.to_lowercase().contains("chatgpt") {
        OAuthConfig::openai_codex()
    } else if req.provider_id.to_lowercase().contains("gemini") ||
              req.provider_id.to_lowercase().contains("google") {
        OAuthConfig::gemini()
    } else {
        OAuthConfig::anthropic()
    };

    let oauth_client = OAuthClient::new(config, state.token_store.clone());

    let token = oauth_client
        .refresh_token(&req.provider_id)
        .await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to refresh token: {}", e)
        ))?;

    Ok(Json(OAuthExchangeResponse {
        success: true,
        message: "Token refreshed successfully".to_string(),
        provider_id: req.provider_id,
        expires_at: token.expires_at.to_rfc3339(),
    }))
}

/// OAuth callback query parameters
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// OAuth callback handler - displays the authorization code to the user
pub async fn oauth_callback(
    Query(params): Query<OAuthCallbackQuery>,
) -> Html<String> {
    // Check for errors
    if let Some(error) = params.error {
        let error_desc = params.error_description.unwrap_or_else(|| "Unknown error".to_string());
        return Html(format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OAuth Error</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }}
        .container {{
            background: white;
            padding: 3rem;
            border-radius: 1rem;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            max-width: 500px;
            text-align: center;
        }}
        .error-icon {{
            font-size: 4rem;
            margin-bottom: 1rem;
        }}
        h1 {{
            color: #e53e3e;
            margin-bottom: 1rem;
        }}
        .error-message {{
            background: #fff5f5;
            border: 1px solid #feb2b2;
            color: #c53030;
            padding: 1rem;
            border-radius: 0.5rem;
            margin-top: 1rem;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="error-icon">[ERROR]</div>
        <h1>Authorization Failed</h1>
        <p><strong>Error:</strong> {error}</p>
        <div class="error-message">{error_desc}</div>
        <p style="margin-top: 2rem; color: #666;">You can close this window and try again.</p>
    </div>
</body>
</html>
"#));
    }

    // Extract code (state is not used for token exchange, verifier is stored in frontend)
    let code = params.code.unwrap_or_else(|| "No code received".to_string());

    Html(format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Authorization Successful</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }}
        .container {{
            background: white;
            padding: 3rem;
            border-radius: 1rem;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            max-width: 500px;
            text-align: center;
        }}
        .success-icon {{
            font-size: 4rem;
            margin-bottom: 1rem;
        }}
        h1 {{
            color: #2d3748;
            margin-bottom: 1rem;
        }}
        .code-box {{
            background: #f7fafc;
            border: 2px solid #e2e8f0;
            padding: 1.5rem;
            border-radius: 0.5rem;
            margin: 1.5rem 0;
            position: relative;
        }}
        .code {{
            font-family: 'Courier New', monospace;
            font-size: 0.9rem;
            word-break: break-all;
            color: #2d3748;
            user-select: all;
        }}
        .copy-button {{
            margin-top: 1rem;
            background: #667eea;
            color: white;
            border: none;
            padding: 0.75rem 2rem;
            border-radius: 0.5rem;
            font-size: 1rem;
            cursor: pointer;
            transition: background 0.3s;
        }}
        .copy-button:hover {{
            background: #5a67d8;
        }}
        .copy-button:active {{
            background: #4c51bf;
        }}
        .copied {{
            color: #48bb78;
            font-weight: bold;
            margin-top: 0.5rem;
            opacity: 0;
            transition: opacity 0.3s;
        }}
        .copied.show {{
            opacity: 1;
        }}
        .instructions {{
            text-align: left;
            margin-top: 2rem;
            padding: 1rem;
            background: #edf2f7;
            border-radius: 0.5rem;
        }}
        .instructions ol {{
            margin: 0.5rem 0;
            padding-left: 1.5rem;
        }}
        .instructions li {{
            margin: 0.5rem 0;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="success-icon">[OK]</div>
        <h1>Authorization Successful!</h1>
        <p>Copy the code below and paste it in the admin panel:</p>

        <div class="code-box">
            <div class="code" id="authCode">{code}</div>
        </div>

        <button class="copy-button" onclick="copyCode()">Copy Code</button>
        <div class="copied" id="copiedMsg">[OK] Copied to clipboard!</div>

        <div class="instructions">
            <strong>Next steps:</strong>
            <ol>
                <li>Click "Copy Code" button above</li>
                <li>Return to the admin panel</li>
                <li>Paste the code in the authorization field</li>
                <li>Click "Complete OAuth" to finish</li>
            </ol>
        </div>

        <p style="margin-top: 2rem; color: #666;">You can close this window after copying the code.</p>
    </div>

    <script>
        function copyCode() {{
            const codeText = document.getElementById('authCode').textContent;
            navigator.clipboard.writeText(codeText).then(() => {{
                const copiedMsg = document.getElementById('copiedMsg');
                copiedMsg.classList.add('show');
                setTimeout(() => {{
                    copiedMsg.classList.remove('show');
                }}, 2000);
            }});
        }}

        // Auto-select code on click
        document.getElementById('authCode').addEventListener('click', function() {{
            const range = document.createRange();
            range.selectNodeContents(this);
            const selection = window.getSelection();
            selection.removeAllRanges();
            selection.addRange(range);
        }});
    </script>
</body>
</html>
"#))
}
