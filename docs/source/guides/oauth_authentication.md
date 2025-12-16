# OAuth Authentication Guide for ArchGW

This guide explains how to use the OAuth Gateway service in ArchGW for authenticating with multiple LLM providers including Claude (Anthropic), Gemini CLI (Google), and ChatGPT Plus/Pro.

## Overview

The OAuth Gateway (`oauth_gateway`) is a dedicated microservice that handles OAuth authentication flows for different LLM providers. It provides REST API endpoints for:

- Starting OAuth authorization flows
- Exchanging authorization codes for tokens
- Managing stored OAuth tokens
- Refreshing expired tokens

## Architecture

The OAuth Gateway runs as a separate service listening on `127.0.0.1:54545` within the ArchGW container. Requests to `/auth/*` paths are automatically routed through Envoy to the OAuth Gateway.

### Components

1. **oauth_gateway binary** - Standalone service handling OAuth flows
2. **Token Store** - Persistent storage at `~/.archgw/oauth_tokens.json` with secure file permissions (0600)
3. **Envoy Integration** - Routes `/auth/*` requests through the proxy
4. **Supervisor Integration** - Manages OAuth Gateway process lifecycle

## Supported OAuth Providers

### 1. Claude Pro/Max (Anthropic)

For Claude Pro and Claude Max subscriptions.

- **Flow Type**: `max`
- **Authentication**: PKCE (Proof Key for Code Exchange)
- **Scopes**: User account access
- **Configuration**: Automated via OAuth Config

### 2. Anthropic Console

For creating API keys through the Anthropic API console.

- **Flow Type**: `console`
- **Authentication**: PKCE
- **Result**: API key creation capability
- **Configuration**: Automated via OAuth Config

### 3. Gemini CLI (Google Cloud Code Assist)

For Google's Gemini AI Pro/Ultra with workspace or individual accounts.

- **Flow Type**: `gemini`
- **Authentication**: OAuth2 with client secret
- **Project ID**: Automatically retrieved for workspace accounts
- **Configuration**: Automated via OAuth Config

### 4. ChatGPT Plus/Pro (OpenAI)

For ChatGPT Plus and ChatGPT Pro subscriptions.

- **Flow Type**: `openai-codex`
- **Authentication**: OAuth2
- **Scopes**: User account access
- **Configuration**: Automated via OAuth Config

## REST API Endpoints

All endpoints accept/return JSON and are available at `http://[host]:54545/auth/` or `http://[host]/auth/` (when routed through Envoy).

### POST /auth/authorize

Start an OAuth authorization flow.

**Request:**
```json
{
  "oauth_type": "max"
}
```

**Parameters:**
- `oauth_type`: One of `"max"`, `"console"`, `"openai-codex"`, or `"gemini"`

**Response:**
```json
{
  "url": "https://api.anthropic.com/oauth/authorize?...",
  "verifier": "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
  "instructions": "Visit the URL above to authorize with your Claude Pro/Max account..."
}
```

Store the `verifier` value - you'll need it for the exchange step.

### POST /auth/exchange

Exchange the authorization code for tokens.

**Request:**
```json
{
  "code": "code_from_oauth_callback",
  "verifier": "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
  "provider_id": "claude-max-personal",
  "oauth_type": "max"
}
```

**Parameters:**
- `code`: Authorization code from OAuth callback
- `verifier`: PKCE verifier from authorize step
- `provider_id`: Identifier for storing this token
- `oauth_type`: (Optional) OAuth type if not inferrable from provider_id

**Response:**
```json
{
  "success": true,
  "message": "OAuth authentication successful! Token saved.",
  "provider_id": "claude-max-personal",
  "expires_at": "2025-01-16T10:00:00Z"
}
```

### GET /auth/tokens

List all stored OAuth tokens.

**Response:**
```json
[
  {
    "provider_id": "claude-max-personal",
    "expires_at": "2025-01-16T10:00:00Z",
    "is_expired": false,
    "needs_refresh": false
  },
  {
    "provider_id": "gemini-workspace",
    "expires_at": "2025-01-16T09:30:00Z",
    "is_expired": false,
    "needs_refresh": false
  }
]
```

### POST /auth/tokens/delete

Delete a stored OAuth token.

**Request:**
```json
{
  "provider_id": "claude-max-personal"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Token for 'claude-max-personal' deleted"
}
```

### POST /auth/tokens/refresh

Manually refresh an OAuth token.

**Request:**
```json
{
  "provider_id": "claude-max-personal"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Token refreshed successfully",
  "provider_id": "claude-max-personal",
  "expires_at": "2025-01-16T10:00:00Z"
}
```

### GET /auth/callback

OAuth callback handler. This endpoint displays the authorization code to the user after successful OAuth completion. Typically used by browser-based OAuth flows.

## OAuth Flow Examples

### Example 1: Claude Pro Authentication

**Step 1: Initiate Authorization**
```bash
curl -X POST http://localhost/auth/authorize \
  -H "Content-Type: application/json" \
  -d '{"oauth_type": "max"}'
```

Response:
```json
{
  "url": "https://api.anthropic.com/oauth/authorize?...",
  "verifier": "ABC123XYZ...",
  "instructions": "Visit the URL above..."
}
```

**Step 2: User visits URL and completes OAuth**

The OAuth provider redirects to `http://localhost:54545/auth/callback?code=AUTH_CODE`

The callback page displays the authorization code.

**Step 3: Exchange Code for Token**
```bash
curl -X POST http://localhost/auth/exchange \
  -H "Content-Type: application/json" \
  -d '{
    "code": "AUTH_CODE_FROM_CALLBACK",
    "verifier": "ABC123XYZ...",
    "provider_id": "claude-max-personal",
    "oauth_type": "max"
  }'
```

Response:
```json
{
  "success": true,
  "message": "OAuth authentication successful! Token saved.",
  "provider_id": "claude-max-personal",
  "expires_at": "2025-01-16T10:00:00Z"
}
```

### Example 2: Gemini Workspace Authentication

**Step 1: Initiate Authorization**
```bash
curl -X POST http://localhost/auth/authorize \
  -H "Content-Type: application/json" \
  -d '{"oauth_type": "gemini"}'
```

**Step 2-3: Same as Claude example, but with `oauth_type: "gemini"`**

The OAuth Gateway will automatically:
- Exchange the code for tokens
- Call Gemini's `loadCodeAssist()` API to get project ID
- Store both token and project ID

## Token Storage

OAuth tokens are stored in `~/.archgw/oauth_tokens.json` with secure permissions (0600 - owner read/write only).

**Token File Format:**
```json
{
  "claude-max-personal": {
    "provider_id": "claude-max-personal",
    "access_token": "sk_live_...",
    "refresh_token": "refresh_...",
    "expires_at": "2025-01-16T10:00:00Z"
  },
  "gemini-workspace": {
    "provider_id": "gemini-workspace",
    "access_token": "ya29.a0A...",
    "refresh_token": "1//0...",
    "expires_at": "2025-01-16T09:30:00Z",
    "project_id": "my-workspace-project"
  }
}
```

## Token Refresh

Tokens are automatically refreshed when:

1. An expired token is requested and a refresh token exists
2. A token is within 5 minutes of expiration
3. Manual refresh is triggered via the `/auth/tokens/refresh` endpoint

## Security Considerations

### PKCE (Proof Key for Code Exchange)

All OAuth flows use PKCE for enhanced security:
- 128-character random verifiers are generated
- SHA256 challenge encoding is used
- Verifiers are unique per authorization request

### Token Storage

- Tokens stored with file permissions 0600 (read/write for owner only)
- Tokens persisted to JSON file at `~/.archgw/oauth_tokens.json`
- File is created only when first token is stored

### Environment Variables

OAuth client IDs and secrets are configured at compile time from environment variables:

```bash
# Anthropic OAuth
ANTHROPIC_CLIENT_ID=...

# Google OAuth
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...

# OpenAI OAuth
OPENAI_CLIENT_ID=...
```

## Troubleshooting

### Token Expired

**Error**: "Token for 'provider-id' is expired"

**Solution**: Call the `/auth/tokens/refresh` endpoint to refresh the token, or delete and re-authenticate.

### Project ID Not Available (Gemini)

**Warning**: "No project ID available" when authenticating with Gemini

**Cause**: Individual Google accounts don't require a project ID. Workspace accounts will have project ID automatically retrieved.

**Solution**: If you're a workspace user and need a specific project ID, set the `GOOGLE_CLOUD_PROJECT` environment variable.

### OAuth Callback Not Received

**Problem**: OAuth provider redirects but page doesn't display code

**Cause**: Envoy routing or port access issues

**Solution**:
1. Verify port 54545 is accessible
2. Check Envoy logs for `/auth/callback` routing
3. Ensure OAuth app redirect URI matches `http://localhost:54545/auth/callback`

### Token Store Not Found

**Error**: "Failed to initialize token store"

**Cause**: Permission issues or corrupted token file

**Solution**:
1. Check `~/.archgw/oauth_tokens.json` exists and is readable
2. Verify directory permissions allow write access
3. Delete token file to force recreation

## Integration with ArchGW

The OAuth Gateway integrates with ArchGW's Envoy proxy:

1. Requests to `http://archgw:80/auth/*` are routed to the OAuth Gateway via Envoy
2. Supervisor manages the oauth_gateway process
3. Token store is shared across service restarts
4. Logs are written to `/var/log/oauth_gateway.log`

## Advanced: Running OAuth Gateway Standalone

For development or testing, you can run the OAuth Gateway as a standalone service:

```bash
./oauth_gateway [host] [port]

# Examples:
./oauth_gateway 127.0.0.1 54545      # Default
./oauth_gateway 0.0.0.0 54545        # Accept external connections
./oauth_gateway localhost 8080       # Custom port
```

The service will display available endpoints on startup:

```
OAuth Gateway Server
[OK] Token store ready at: ~/.archgw/oauth_tokens.json
[OK] Claude Pro/Max OAuth configured: anthropic_...
[OK] Gemini CLI OAuth configured: google_...
[OK] ChatGPT Plus/Pro OAuth configured: openai_...
[OK] Anthropic Console OAuth configured: anthropic_...

[INFO] Starting OAuth Gateway server on 127.0.0.1:54545

[INFO] Available endpoints:
  POST   http://127.0.0.1:54545/auth/authorize          - Start OAuth authorization
  POST   http://127.0.0.1:54545/auth/exchange           - Exchange code for tokens
  GET    http://127.0.0.1:54545/auth/callback           - OAuth callback handler
  GET    http://127.0.0.1:54545/auth/tokens             - List all tokens
  POST   http://127.0.0.1:54545/auth/tokens/delete      - Delete a token
  POST   http://127.0.0.1:54545/auth/tokens/refresh     - Refresh a token
```

## Related Documentation

- [ArchGW Architecture](../architecture.md)
- [Envoy Configuration](../configuration/envoy.md)
- [Supervisor Configuration](../configuration/supervisor.md)
