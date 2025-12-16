use std::collections::HashMap;

use common::configuration::{Agent, AgentFilterChain};
use common::consts::{ARCH_UPSTREAM_HOST_HEADER, ENVOY_RETRY_HEADER};
use common::traces::{SpanBuilder, SpanKind};
use hermesllm::apis::openai::{ChatCompletionsRequest, Message};
use hyper::header::HeaderMap;
use opentelemetry::trace::TraceContextExt;
use tracing::{debug, info, warn};
use std::time::{Instant, SystemTime};

use crate::handlers::jsonrpc::{JsonRpcId, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use uuid::Uuid;

/// Errors that can occur during pipeline processing
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Agent '{0}' not found in agent map")]
    AgentNotFound(String),
    #[error("No choices in response from agent '{0}'")]
    NoChoicesInResponse(String),
    #[error("No content in response from agent '{0}'")]
    NoContentInResponse(String),
    #[error("No result in response from agent '{0}'")]
    NoResultInResponse(String),
    #[error("No structured content in response from agent '{0}'")]
    NoStructuredContentInResponse(String),
    #[error("No messages in response from agent '{0}'")]
    NoMessagesInResponse(String),
    #[error("Client error from agent '{agent}' (HTTP {status}): {body}")]
    ClientError {
        agent: String,
        status: u16,
        body: String,
    },
    #[error("Server error from agent '{agent}' (HTTP {status}): {body}")]
    ServerError {
        agent: String,
        status: u16,
        body: String,
    },
}

/// Service for processing agent pipelines
pub struct PipelineProcessor {
    client: reqwest::Client,
    url: String,
    agent_id_session_map: HashMap<String, String>,
}

const ENVOY_API_ROUTER_ADDRESS: &str = "http://localhost:11000";

impl Default for PipelineProcessor {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
            url: ENVOY_API_ROUTER_ADDRESS.to_string(),
            agent_id_session_map: HashMap::new(),
        }
    }
}

impl PipelineProcessor {
    pub fn new(url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            url,
            agent_id_session_map: HashMap::new(),
        }
    }

    /// Process the filter chain of agents (all except the terminal agent)
    pub async fn process_filter_chain(
        &mut self,
        chat_history: &[Message],
        agent_filter_chain: &AgentFilterChain,
        agent_map: &HashMap<String, Agent>,
        request_headers: &HeaderMap,
        trace_collector: Option<&std::sync::Arc<common::traces::TraceCollector>>,
    ) -> Result<Vec<Message>, PipelineError> {
        let mut chat_history_updated = chat_history.to_vec();

        for agent_name in &agent_filter_chain.filter_chain {
            debug!("Processing filter agent: {}", agent_name);

            let agent = agent_map
                .get(agent_name)
                .ok_or_else(|| PipelineError::AgentNotFound(agent_name.clone()))?;

            let tool_name = agent.tool.as_deref().unwrap_or(&agent.id);

            info!(
                "executing filter: {}/{}, url: {}, conversation length: {}",
                agent_name,
                tool_name,
                agent.url,
                chat_history.len()
            );

            let start_time = SystemTime::now();
            let start_instant = Instant::now();
            
            // Extract trace context from OpenTelemetry
            let current_cx = opentelemetry::Context::current();
            let span_ref = current_cx.span();
            let span_context = span_ref.span_context();
            let trace_id = if span_context.is_valid() {
                format!("{:032x}", span_context.trace_id())
            } else {
                String::new() // SpanBuilder will generate one
            };
            let parent_span_id = if span_context.is_valid() {
                Some(format!("{:016x}", span_context.span_id()))
            } else {
                None
            };
            
            chat_history_updated = self
                .execute_filter(&chat_history_updated, agent, request_headers)
                .await?;
            
            let end_time = SystemTime::now();
            let elapsed = start_instant.elapsed();

            info!(
                "Filter '{}' completed in {:.2}ms, updated conversation length: {}",
                agent_name,
                elapsed.as_secs_f64() * 1000.0,
                chat_history_updated.len()
            );
            
            // Build span with trace context
            if let Some(collector) = trace_collector {
                let mut span_builder = SpanBuilder::new(format!("filter_execution: {}", agent_name))
                    .with_kind(SpanKind::Internal)
                    .with_start_time(start_time)
                    .with_end_time(end_time)
                    .with_attribute("filter_name", agent_name.to_string())
                    .with_attribute("tool_name", tool_name.to_string())
                    .with_attribute("duration_ms", format!("{:.2}", elapsed.as_secs_f64() * 1000.0));
                
                if !trace_id.is_empty() {
                    span_builder = span_builder.with_trace_id(trace_id);
                }
                if let Some(parent_id) = parent_span_id {
                    span_builder = span_builder.with_parent_span_id(parent_id);
                }
                
                let span = span_builder.build();
                collector.record_span("brightstaff", span);
            }
        }

        Ok(chat_history_updated)
    }

    /// Send request to a specific agent and return the response content
    async fn execute_filter(
        &mut self,
        messages: &[Message],
        agent: &Agent,
        request_headers: &HeaderMap,
    ) -> Result<Vec<Message>, PipelineError> {
        let mcp_session_id = if let Some(session_id) = self.agent_id_session_map.get(&agent.id) {
            session_id.clone()
        } else {
            let session_id = self.get_new_session_id(&agent.id).await;
            self.agent_id_session_map
                .insert(agent.id.clone(), session_id.clone());
            session_id
        };

        // let mut request = original_request.clone();
        // request.messages = messages.to_vec();

        let tool_name = agent.tool.as_deref().unwrap_or(&agent.id);

        let arguments = serde_json::json!({
            "messages": messages
        });

        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        let json_rpc_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::String(Uuid::new_v4().to_string()),
            method: "tools/call".to_string(),
            params: Some(serde_json::from_value(params)?),
        };

        let request_body = serde_json::to_string(&json_rpc_request)?;
        info!("Sending request to agent {}", agent.id);
        info!("Request body: {}", request_body);

        // Pretty print for debugging
        let pretty_body = serde_json::to_string_pretty(&json_rpc_request)?;
        info!("Request body (pretty):\n{}", pretty_body);

        let mut agent_headers = request_headers.clone();
        info!(
            "Using MCP session ID {} for agent {}",
            mcp_session_id, agent.id
        );

        // Log all headers being sent
        info!("Headers being sent:");
        for (key, value) in agent_headers.iter() {
            info!("  {}: {:?}", key, value);
        }

        agent_headers.insert(
            "mcp-session-id",
            hyper::header::HeaderValue::from_str(&mcp_session_id).unwrap(),
        );
        agent_headers.remove(hyper::header::CONTENT_LENGTH);
        agent_headers.insert(
            ARCH_UPSTREAM_HOST_HEADER,
            hyper::header::HeaderValue::from_str(&agent.id)
                .map_err(|_| PipelineError::AgentNotFound(agent.id.clone()))?,
        );

        agent_headers.insert(
            ENVOY_RETRY_HEADER,
            hyper::header::HeaderValue::from_str("3").unwrap(),
        );

        agent_headers.insert(
            "Accept",
            hyper::header::HeaderValue::from_static("application/json, text/event-stream"),
        );

        agent_headers.insert(
            "Content-Type",
            hyper::header::HeaderValue::from_static("application/json"),
        );

        info!("Final headers being sent:");
        for (key, value) in agent_headers.iter() {
            info!("  {}: {:?}", key, value);
        }

        let response = self
            .client
            .post(format!("{}/mcp", self.url))
            .headers(agent_headers)
            .body(request_body)
            .send()
            .await?;

        let http_status = response.status();
        let response_bytes = response.bytes().await?;

        if !http_status.is_success() {
            let error_body = String::from_utf8_lossy(&response_bytes).to_string();

            if http_status.is_client_error() {
                // 4xx errors - cascade back to developer
                return Err(PipelineError::ClientError {
                    agent: agent.id.clone(),
                    status: http_status.as_u16(),
                    body: error_body,
                });
            } else if http_status.is_server_error() {
                // 5xx errors - server/agent error
                return Err(PipelineError::ServerError {
                    agent: agent.id.clone(),
                    status: http_status.as_u16(),
                    body: error_body,
                });
            }
        }

        info!(
            "response bytes in str: {}",
            String::from_utf8_lossy(&response_bytes)
        );

        let response_str = String::from_utf8_lossy(&response_bytes);
        let lines: Vec<&str> = response_str.lines().collect();

        // Validate SSE format: first line should be "event: message"
        if lines.is_empty() || lines[0] != "event: message" {
            warn!("Invalid SSE response format from agent {}: expected 'event: message' as first line, got: {:?}", agent.id, lines.first());
            return Err(PipelineError::NoContentInResponse(format!(
                "Invalid SSE response format from agent {}: expected 'event: message' as first line",
                agent.id
            )));
        }

        // Find the data line
        let data_lines: Vec<&str> = lines
            .iter()
            .filter(|line| line.starts_with("data: "))
            .copied()
            .collect();

        if data_lines.len() != 1 {
            warn!(
                "Expected exactly one 'data:' line from agent {}, found {}",
                agent.id,
                data_lines.len()
            );
            return Err(PipelineError::NoContentInResponse(format!(
                "Expected exactly one 'data:' line from agent {}, found {}",
                agent.id,
                data_lines.len()
            )));
        }

        let data_chunk = &data_lines[0][6..]; // Skip "data: " prefix

        let response: JsonRpcResponse = serde_json::from_str(data_chunk)?;
        let response_result = response
            .result
            .ok_or_else(|| PipelineError::NoResultInResponse(agent.id.clone()))?;

        // check if error field is set in response result
        let mcp_error = response_result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if mcp_error {
            let error_message = response_result
                .get("content")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.get(0))
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or("unknown_error".to_string());

            return Err(PipelineError::ClientError {
                agent: agent.id.clone(),
                status: http_status.as_u16(),
                body: error_message,
            });
        }

        let response_json = response_result
            .get("structuredContent")
            .ok_or_else(|| PipelineError::NoStructuredContentInResponse(agent.id.clone()))?;
        // Parse the response as JSON to extract the content
        // let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)?;

        let messages: Vec<Message> = response_json
            .get("result")
            .and_then(|v| v.as_array())
            .ok_or_else(|| PipelineError::NoMessagesInResponse(agent.id.clone()))?
            .iter()
            .map(|msg_value| serde_json::from_value(msg_value.clone()))
            .collect::<Result<Vec<Message>, _>>()
            .map_err(PipelineError::ParseError)?;

        Ok(messages)
    }

    async fn get_new_session_id(&self, agent_id: &str) -> String {
        let initialize_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(1),
            method: "initialize".to_string(),
            params: Some({
                let mut params = HashMap::new();
                params.insert(
                    "protocolVersion".to_string(),
                    serde_json::Value::String("2024-11-05".to_string()),
                );
                params.insert("capabilities".to_string(), serde_json::json!({}));
                params.insert(
                    "clientInfo".to_string(),
                    serde_json::json!({
                        "name": "brightstaff",
                        "version": "1.0.0"
                    }),
                );
                params
            }),
        };

        let request_body = serde_json::to_string(&initialize_request).unwrap();

        info!("Initializing MCP session for agent {}", agent_id);
        info!("Initialize request body: {}", request_body);

        let response = self
            .client
            .post(format!("{}/mcp", self.url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header(ARCH_UPSTREAM_HOST_HEADER, agent_id)
            .body(request_body)
            .send()
            .await
            .expect("Failed to initialize MCP session");

        info!("Initialize response status: {}", response.status());
        info!("Initialize response headers: {:?}", response.headers());

        let session_id = response
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .expect("No mcp-session-id in response")
            .to_string();

        info!(
            "Created new MCP session for agent {}: {}",
            agent_id, session_id
        );

        // Send initialized notification (without id field per JSON-RPC 2.0 spec)
        let initialized_notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
            params: None,
        };

        let notification_body = serde_json::to_string(&initialized_notification).unwrap();

        info!("Sending initialized notification: {}", notification_body);

        let notif_response = self
            .client
            .post(format!("{}/mcp", self.url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-session-id", &session_id)
            .header(ARCH_UPSTREAM_HOST_HEADER, agent_id)
            .body(notification_body)
            .send()
            .await
            .expect("Failed to send initialized notification");

        info!(
            "Initialized notification response status: {}",
            notif_response.status()
        );

        session_id
    }

    /// Send request to terminal agent and return the raw response for streaming
    pub async fn invoke_terminal_agent(
        &self,
        messages: &[Message],
        original_request: &ChatCompletionsRequest,
        terminal_agent: &Agent,
        request_headers: &HeaderMap,
    ) -> Result<reqwest::Response, PipelineError> {
        let mut request = original_request.clone();
        request.messages = messages.to_vec();

        let request_body = serde_json::to_string(&request)?;
        debug!("Sending request to terminal agent {}", terminal_agent.id);

        let mut agent_headers = request_headers.clone();
        agent_headers.remove(hyper::header::CONTENT_LENGTH);
        agent_headers.insert(
            ARCH_UPSTREAM_HOST_HEADER,
            hyper::header::HeaderValue::from_str(&terminal_agent.id)
                .map_err(|_| PipelineError::AgentNotFound(terminal_agent.id.clone()))?,
        );

        agent_headers.insert(
            ENVOY_RETRY_HEADER,
            hyper::header::HeaderValue::from_str("3").unwrap(),
        );

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.url))
            .headers(agent_headers)
            .body(request_body)
            .send()
            .await?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermesllm::apis::openai::{Message, MessageContent, Role};
    use std::collections::HashMap;

    fn create_test_message(role: Role, content: &str) -> Message {
        Message {
            role,
            content: MessageContent::Text(content.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn create_test_pipeline(agents: Vec<&str>) -> AgentFilterChain {
        AgentFilterChain {
            id: "test-agent".to_string(),
            filter_chain: agents.iter().map(|s| s.to_string()).collect(),
            description: None,
            default: None,
        }
    }

    #[tokio::test]
    async fn test_agent_not_found_error() {
        let mut processor = PipelineProcessor::default();
        let agent_map = HashMap::new();
        let request_headers = HeaderMap::new();

        let initial_request = ChatCompletionsRequest {
            messages: vec![create_test_message(Role::User, "Hello")],
            model: "test-model".to_string(),
            ..Default::default()
        };

        let pipeline = create_test_pipeline(vec!["nonexistent-agent", "terminal-agent"]);

        let result = processor
            .process_filter_chain(
                &initial_request.messages,
                &pipeline,
                &agent_map,
                &request_headers,
                None,
            )
            .await;

        assert!(result.is_err());
        matches!(result.unwrap_err(), PipelineError::AgentNotFound(_));
    }
}
