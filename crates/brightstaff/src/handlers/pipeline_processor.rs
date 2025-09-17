use std::collections::HashMap;

use common::configuration::{Agent, AgentPipeline};
use common::consts::ARCH_UPSTREAM_HOST_HEADER;
use hermesllm::apis::openai::{ChatCompletionsRequest, Message};
use hyper::header::HeaderMap;
use tracing::{debug, warn};

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
}

/// Service for processing agent pipelines
pub struct PipelineProcessor {
    client: reqwest::Client,
    llm_endpoint: String,
}

impl Default for PipelineProcessor {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
            llm_endpoint: "http://localhost:11000/v1/chat/completions".to_string(),
        }
    }
}

impl PipelineProcessor {
    pub fn new(llm_endpoint: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            llm_endpoint,
        }
    }

    /// Process the filter chain of agents (all except the terminal agent)
    pub async fn process_filter_chain(
        &self,
        initial_request: &ChatCompletionsRequest,
        agent_pipeline: &AgentPipeline,
        agent_map: &HashMap<String, Agent>,
        request_headers: &HeaderMap,
    ) -> Result<Vec<Message>, PipelineError> {
        let mut chat_completions_history = initial_request.messages.clone();

        let filter_chain_without_terminal =
            &agent_pipeline.filter_chain[..agent_pipeline.filter_chain.len().saturating_sub(1)];

        for agent_name in filter_chain_without_terminal {
            debug!("Processing filter agent: {}", agent_name);

            let agent = agent_map
                .get(agent_name)
                .ok_or_else(|| PipelineError::AgentNotFound(agent_name.clone()))?;

            debug!("Agent details: {:?}", agent);

            let response_content = self
                .send_agent_request(
                    &chat_completions_history,
                    initial_request,
                    agent,
                    request_headers,
                )
                .await?;

            debug!("Received response from filter agent {}", agent_name);

            // Parse the response content as new message history
            chat_completions_history =
                serde_json::from_str(&response_content).inspect_err(|err| {
                    warn!(
                        "Failed to parse response from agent {}, err: {}, response: {}",
                        agent_name, err, response_content
                    )
                })?;
        }

        Ok(chat_completions_history)
    }

    /// Send request to a specific agent and return the response content
    async fn send_agent_request(
        &self,
        messages: &[Message],
        original_request: &ChatCompletionsRequest,
        agent: &Agent,
        request_headers: &HeaderMap,
    ) -> Result<String, PipelineError> {
        let mut request = original_request.clone();
        request.messages = messages.to_vec();

        let request_body = serde_json::to_string(&request)?;
        debug!("Sending request to agent {}", agent.name);

        let mut agent_headers = request_headers.clone();
        agent_headers.remove(hyper::header::CONTENT_LENGTH);
        agent_headers.insert(
            ARCH_UPSTREAM_HOST_HEADER,
            hyper::header::HeaderValue::from_str(&agent.name)
                .map_err(|_| PipelineError::AgentNotFound(agent.name.clone()))?,
        );

        let response = self
            .client
            .post(&self.llm_endpoint)
            .headers(agent_headers)
            .body(request_body)
            .send()
            .await?;

        let response_bytes = response.bytes().await?;

        // Parse the response as JSON to extract the content
        let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)?;

        let content = response_json
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| PipelineError::NoContentInResponse(agent.name.clone()))?
            .to_string();

        Ok(content)
    }

    /// Send request to terminal agent and return the raw response for streaming
    pub async fn send_terminal_request(
        &self,
        messages: &[Message],
        original_request: &ChatCompletionsRequest,
        terminal_agent: &Agent,
        request_headers: &HeaderMap,
    ) -> Result<reqwest::Response, PipelineError> {
        let mut request = original_request.clone();
        request.messages = messages.to_vec();

        let request_body = serde_json::to_string(&request)?;
        debug!("Sending request to terminal agent {}", terminal_agent.name);

        let mut agent_headers = request_headers.clone();
        agent_headers.remove(hyper::header::CONTENT_LENGTH);
        agent_headers.insert(
            ARCH_UPSTREAM_HOST_HEADER,
            hyper::header::HeaderValue::from_str(&terminal_agent.name)
                .map_err(|_| PipelineError::AgentNotFound(terminal_agent.name.clone()))?,
        );

        let response = self
            .client
            .post(&self.llm_endpoint)
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

    fn create_test_pipeline(agents: Vec<&str>) -> AgentPipeline {
        AgentPipeline {
            name: "test-pipeline".to_string(),
            filter_chain: agents.iter().map(|s| s.to_string()).collect(),
            description: None,
            default: None,
        }
    }

    #[tokio::test]
    async fn test_process_empty_filter_chain() {
        let processor = PipelineProcessor::default();
        let agent_map = HashMap::new();
        let request_headers = HeaderMap::new();

        let initial_request = ChatCompletionsRequest {
            messages: vec![create_test_message(Role::User, "Hello")],
            model: "test-model".to_string(),
            ..Default::default()
        };

        // Pipeline with only terminal agent (no filter chain)
        let pipeline = create_test_pipeline(vec!["terminal-agent"]);

        let result = processor
            .process_filter_chain(&initial_request, &pipeline, &agent_map, &request_headers)
            .await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        if let MessageContent::Text(text) = &messages[0].content {
            assert_eq!(text, "Hello");
        }
    }

    #[tokio::test]
    async fn test_agent_not_found_error() {
        let processor = PipelineProcessor::default();
        let agent_map = HashMap::new();
        let request_headers = HeaderMap::new();

        let initial_request = ChatCompletionsRequest {
            messages: vec![create_test_message(Role::User, "Hello")],
            model: "test-model".to_string(),
            ..Default::default()
        };

        let pipeline = create_test_pipeline(vec!["nonexistent-agent", "terminal-agent"]);

        let result = processor
            .process_filter_chain(&initial_request, &pipeline, &agent_map, &request_headers)
            .await;

        assert!(result.is_err());
        matches!(result.unwrap_err(), PipelineError::AgentNotFound(_));
    }
}
