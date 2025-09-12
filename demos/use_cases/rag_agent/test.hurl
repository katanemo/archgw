# Step 1: Initialize session
POST http://localhost:10101/mcp
Content-Type: application/json
Accept: application/json, text/event-stream

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "ExampleClient",
      "version": "1.0.0"
    }
  }
}

HTTP 200
[Captures]
session_id: header "mcp-session-id"

# # Step 2: List tools (use session ID from previous response)
POST http://localhost:10101/mcp
Content-Type: application/json
Accept: application/json, text/event-stream
mcp-session-id: 07603206a9b44a3d91d76f6b16f24faa

{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
