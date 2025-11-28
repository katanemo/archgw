use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// MCP Tool definition from tools/list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<serde_json::Value>,
}

/// Response from MCP tools/list endpoint
#[derive(Debug, Serialize, Deserialize)]
struct McpToolsListResponse {
    tools: Vec<McpTool>,
}

/// Errors that can occur during MCP communication
#[derive(Debug, thiserror::Error)]
pub enum McpClientError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Invalid MCP URL: {0}")]
    InvalidUrl(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
}

/// Client for communicating with MCP (Model Context Protocol) servers
pub struct McpClient {
    client: Client,
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Parse MCP URL to extract host, port, and optional tool name
    /// Supports formats:
    /// - mcp://host:port
    /// - mcp://host:port/tool_name
    /// - mcp://host:port?tool=tool_name
    fn parse_mcp_url(&self, mcp_url: &str) -> Result<(String, Option<String>), McpClientError> {
        // Remove mcp:// prefix
        let url_without_scheme = mcp_url
            .strip_prefix("mcp://")
            .ok_or_else(|| McpClientError::InvalidUrl(format!("URL must start with mcp://: {}", mcp_url)))?;

        // Parse host:port and optional tool
        let base_url: String;
        let mut tool_name: Option<String> = None;

        if let Some(query_start) = url_without_scheme.find('?') {
            // Format: mcp://host:port?tool=tool_name
            base_url = url_without_scheme[..query_start].to_string();
            let query = &url_without_scheme[query_start + 1..];
            
            // Parse query parameters
            for param in query.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "tool" {
                        tool_name = Some(value.to_string());
                    }
                }
            }
        } else if let Some(path_start) = url_without_scheme.find('/') {
            // Format: mcp://host:port/tool_name
            base_url = url_without_scheme[..path_start].to_string();
            tool_name = Some(url_without_scheme[path_start + 1..].to_string());
        } else {
            // Format: mcp://host:port
            base_url = url_without_scheme.to_string();
        }

        Ok((format!("http://{}", base_url), tool_name))
    }

    /// Fetch list of tools from MCP server via SSE
    pub async fn fetch_tools(&self, mcp_url: &str) -> Result<Vec<McpTool>, McpClientError> {
        let (http_url, _) = self.parse_mcp_url(mcp_url)?;
        let tools_list_url = format!("{}/sse/tools/list", http_url);

        debug!("Fetching tools from MCP endpoint: {}", tools_list_url);

        let response = self.client
            .get(&tools_list_url)
            .header("Accept", "text/event-stream")
            .send()
            .await?;

        if !response.status().is_success() {
            warn!(
                "Failed to fetch tools from {}: status {}",
                tools_list_url,
                response.status()
            );
            return Ok(Vec::new());
        }

        let body = response.text().await?;
        debug!("Received tools list response: {}", body);

        // Parse SSE response - looking for data: lines
        let mut tools = Vec::new();
        for line in body.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    break;
                }
                
                match serde_json::from_str::<McpToolsListResponse>(data) {
                    Ok(response) => {
                        tools.extend(response.tools);
                    }
                    Err(e) => {
                        debug!("Failed to parse tools list data: {}, line: {}", e, data);
                    }
                }
            }
        }

        debug!("Fetched {} tools from MCP server", tools.len());
        Ok(tools)
    }

    /// Fetch specific tool description from MCP server
    /// If tool_name is None, uses the tool name from the URL or returns the first tool
    pub async fn fetch_tool_description(
        &self,
        mcp_url: &str,
        tool_name_override: Option<&str>,
    ) -> Result<String, McpClientError> {
        let (_, url_tool_name) = self.parse_mcp_url(mcp_url)?;
        
        // Determine which tool to look for
        let target_tool_name = tool_name_override
            .or(url_tool_name.as_deref())
            .ok_or_else(|| {
                McpClientError::InvalidUrl(
                    "No tool name specified in URL or parameter".to_string()
                )
            })?;

        debug!("Fetching description for tool: {}", target_tool_name);

        let tools = self.fetch_tools(mcp_url).await?;
        
        let tool = tools
            .iter()
            .find(|t| t.name == target_tool_name)
            .ok_or_else(|| McpClientError::ToolNotFound(target_tool_name.to_string()))?;

        Ok(tool.description.clone().unwrap_or_default())
    }

    /// Fetch all tools as a map of tool name to description
    pub async fn fetch_tools_map(
        &self,
        mcp_url: &str,
    ) -> Result<HashMap<String, String>, McpClientError> {
        let tools = self.fetch_tools(mcp_url).await?;
        
        Ok(tools
            .into_iter()
            .map(|tool| {
                (tool.name, tool.description.unwrap_or_default())
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mcp_url_basic() {
        let client = McpClient::new();
        
        let (http_url, tool) = client.parse_mcp_url("mcp://localhost:10500").unwrap();
        assert_eq!(http_url, "http://localhost:10500");
        assert_eq!(tool, None);
    }

    #[test]
    fn test_parse_mcp_url_with_path() {
        let client = McpClient::new();
        
        let (http_url, tool) = client.parse_mcp_url("mcp://localhost:10500/rewrite_query").unwrap();
        assert_eq!(http_url, "http://localhost:10500");
        assert_eq!(tool, Some("rewrite_query".to_string()));
    }

    #[test]
    fn test_parse_mcp_url_with_query_param() {
        let client = McpClient::new();
        
        let (http_url, tool) = client.parse_mcp_url("mcp://localhost:10500?tool=rewrite_query").unwrap();
        assert_eq!(http_url, "http://localhost:10500");
        assert_eq!(tool, Some("rewrite_query".to_string()));
    }

    #[test]
    fn test_parse_mcp_url_with_host_docker_internal() {
        let client = McpClient::new();
        
        let (http_url, tool) = client
            .parse_mcp_url("mcp://host.docker.internal:10500/context_builder")
            .unwrap();
        assert_eq!(http_url, "http://host.docker.internal:10500");
        assert_eq!(tool, Some("context_builder".to_string()));
    }

    #[test]
    fn test_parse_mcp_url_invalid() {
        let client = McpClient::new();
        
        let result = client.parse_mcp_url("http://localhost:10500");
        assert!(result.is_err());
    }
}
