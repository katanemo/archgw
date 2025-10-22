use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::{Response, StatusCode};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::{info, warn};

// Re-export necessary types for the reasoning implementation
use super::pipeline_processor::PipelineProcessor;
use common::configuration::{Agent, AgentFilterChain};
use hermesllm::apis::openai::ChatCompletionsRequest;

/// Errors that can occur during response handling
#[derive(Debug, thiserror::Error)]
pub enum ResponseError {
    #[error("Failed to create response: {0}")]
    ResponseCreationFailed(#[from] hyper::http::Error),
    #[error("Stream error: {0}")]
    StreamError(String),
}

/// Service for handling HTTP responses and streaming
pub struct ResponseHandler;

impl ResponseHandler {
    pub fn new() -> Self {
        Self
    }

    /// Create a full response body from bytes
    pub fn create_full_body<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
        Full::new(chunk.into())
            .map_err(|never| match never {})
            .boxed()
    }

    /// Create an error response with a given status code and message
    pub fn create_error_response(
        status: StatusCode,
        message: &str,
    ) -> Response<BoxBody<Bytes, hyper::Error>> {
        let mut response = Response::new(Self::create_full_body(message.to_string()));
        *response.status_mut() = status;
        response
    }

    /// Create a bad request response
    pub fn create_bad_request(message: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
        Self::create_error_response(StatusCode::BAD_REQUEST, message)
    }

    /// Create an internal server error response
    pub fn create_internal_error(message: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
        Self::create_error_response(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// Create a JSON error response
    pub fn create_json_error_response(
        error_json: &serde_json::Value,
    ) -> Response<BoxBody<Bytes, hyper::Error>> {
        let json_string = error_json.to_string();
        let mut response = Response::new(Self::create_full_body(json_string));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        response
    }

    /// Create a streaming response from a reqwest response
    pub async fn create_streaming_response(
        &self,
        llm_response: reqwest::Response,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, ResponseError> {
        // Copy headers from the original response
        let response_headers = llm_response.headers();
        let mut response_builder = Response::builder();

        let headers = response_builder.headers_mut().ok_or_else(|| {
            ResponseError::StreamError("Failed to get mutable headers".to_string())
        })?;

        for (header_name, header_value) in response_headers.iter() {
            headers.insert(header_name, header_value.clone());
        }

        // Create channel for async streaming
        let (tx, rx) = mpsc::channel::<Bytes>(16);

        // Spawn task to stream data
        tokio::spawn(async move {
            let mut byte_stream = llm_response.bytes_stream();

            while let Some(item) = byte_stream.next().await {
                let chunk = match item {
                    Ok(chunk) => chunk,
                    Err(err) => {
                        warn!("Error receiving chunk: {:?}", err);
                        break;
                    }
                };

                if tx.send(chunk).await.is_err() {
                    warn!("Receiver dropped");
                    break;
                }
            }
        });

        let stream = ReceiverStream::new(rx).map(|chunk| Ok::<_, hyper::Error>(Frame::data(chunk)));
        let stream_body = BoxBody::new(StreamBody::new(stream));

        response_builder
            .body(stream_body)
            .map_err(ResponseError::from)
    }

    /// Create a streaming response with reasoning blocks for debug mode
    pub async fn create_streaming_response_with_reasoning(
        &self,
        chat_request: &ChatCompletionsRequest,
        selected_agent: &AgentFilterChain,
        processed_messages: &[hermesllm::apis::openai::Message],
        llm_response: reqwest::Response,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, ResponseError> {
        // Create reasoning content
        let mut reasoning_content = String::new();

        reasoning_content.push_str(&format!(
            "Starting agent processing pipeline for query: \"{}\"\n\n",
            chat_request
                .messages
                .last()
                .map(|m| match &m.content {
                    hermesllm::apis::openai::MessageContent::Text(text) => text.as_str(),
                    _ => "complex content",
                })
                .unwrap_or("unknown")
        ));

        reasoning_content.push_str(&format!(
            "Selected agent filter chain: {} {}\n",
            selected_agent.id,
            selected_agent.description.as_deref().unwrap_or("")
        ));

        if !selected_agent.filter_chain.is_empty() {
            reasoning_content.push_str(&format!(
                "Processed {} filter agents in sequence:\n",
                selected_agent.filter_chain.len()
            ));

            for (index, filter_name) in selected_agent.filter_chain.iter().enumerate() {
                reasoning_content.push_str(&format!(
                    "{}. ✓ Filter agent: {} completed\n",
                    index + 1,
                    filter_name
                ));
            }
        } else {
            reasoning_content
                .push_str("No filter agents configured, proceeded directly to terminal agent\n");
        }

        reasoning_content.push_str(&format!(
            "\nFilter chain processing completed successfully\n"
        ));
        reasoning_content.push_str(&format!(
            "Final message count: {}\n",
            processed_messages.len()
        ));
        reasoning_content.push_str("Now streaming response from terminal agent...\n\n");

        // Create channel for streaming
        let (tx, rx) = mpsc::channel::<Result<Bytes, hyper::Error>>(100);

        // Send reasoning block first
        let reasoning_chunk = Self::create_reasoning_chunk(&reasoning_content);
        let _ = tx.send(Ok(Bytes::from(reasoning_chunk))).await;

        // Clone response headers for streaming
        let response_headers = llm_response.headers().clone();

        tokio::spawn(async move {
            // Stream the LLM response chunks
            let mut byte_stream = llm_response.bytes_stream();
            while let Some(item) = byte_stream.next().await {
                match item {
                    Ok(chunk) => {
                        if tx.send(Ok(chunk)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Create streaming response with original headers
        let mut response_builder = Response::builder();

        // Copy relevant headers from the LLM response
        if let Some(headers) = response_builder.headers_mut() {
            for (header_name, header_value) in response_headers.iter() {
                if header_name == "content-type" || header_name == "cache-control" {
                    headers.insert(header_name, header_value.clone());
                }
            }
        }

        let stream =
            ReceiverStream::new(rx).map(|chunk| Ok::<_, hyper::Error>(Frame::data(chunk?)));
        let stream_body = BoxBody::new(StreamBody::new(stream));

        response_builder
            .status(200)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .body(stream_body)
            .map_err(ResponseError::from)
    }

    /// Create a streaming response with real-time reasoning blocks for debug mode
    pub async fn create_streaming_response_with_realtime_reasoning(
        &self,
        chat_request: &ChatCompletionsRequest,
        selected_agent: &AgentFilterChain,
        agent_map: &HashMap<String, Agent>,
        request_headers: &hyper::HeaderMap,
        _pipeline_processor: &PipelineProcessor,
        is_streaming: bool,
        debug_mode: bool,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, ResponseError> {
        // Create channel for streaming
        let (tx, rx) = mpsc::channel::<Result<Bytes, hyper::Error>>(100);

        // Clone necessary data for the async task
        let chat_request = chat_request.clone();
        let selected_agent = selected_agent.clone();
        let agent_map = agent_map.clone();
        let request_headers = request_headers.clone();
        let url = "http://localhost:11000/v1/chat/completions".to_string();

        tokio::spawn(async move {
            if debug_mode && is_streaming {
                // Send initial reasoning block
                let mut reasoning_content = String::new();
                reasoning_content.push_str(&format!(
                    "🚀 Starting agent processing pipeline for query: \"{}\"\n\n",
                    chat_request
                        .messages
                        .last()
                        .map(|m| match &m.content {
                            hermesllm::apis::openai::MessageContent::Text(text) => text.as_str(),
                            _ => "complex content",
                        })
                        .unwrap_or("unknown")
                ));

                reasoning_content.push_str(&format!(
                    "🎯 Selected agent filter chain: {} {}\n",
                    selected_agent.id,
                    selected_agent.description.as_deref().unwrap_or("")
                ));

                if !selected_agent.filter_chain.is_empty() {
                    reasoning_content.push_str(&format!(
                        "🔄 Processing {} filter agents in sequence:\n",
                        selected_agent.filter_chain.len()
                    ));
                } else {
                    reasoning_content.push_str(
                        "⚡ No filter agents configured, proceeding directly to terminal agent\n",
                    );
                }

                let reasoning_chunk = Self::create_reasoning_chunk(&reasoning_content);
                let _ = tx.send(Ok(Bytes::from(reasoning_chunk))).await;
            }

            // Process filter chain step by step with real-time updates
            let mut current_messages = chat_request.messages.clone();

            if !selected_agent.filter_chain.is_empty() {
                for (index, filter_name) in selected_agent.filter_chain.iter().enumerate() {
                    if debug_mode && is_streaming {
                        // Send reasoning update for starting this filter
                        let filter_start_reasoning = format!(
                            "🔄 Step {}: Calling filter agent '{}'\n",
                            index + 1,
                            filter_name
                        );
                        let filter_chunk = Self::create_reasoning_chunk(&filter_start_reasoning);
                        let _ = tx.send(Ok(Bytes::from(filter_chunk))).await;
                    }

                    if let Some(filter_agent) = agent_map.get(filter_name) {
                        let start_time = std::time::Instant::now();

                        // Process this individual filter
                        match Self::process_single_filter(
                            &current_messages,
                            &chat_request,
                            filter_agent,
                            &request_headers,
                            &url,
                        )
                        .await
                        {
                            Ok(new_messages) => {
                                current_messages = new_messages;
                                if debug_mode && is_streaming {
                                    let duration = start_time.elapsed();

                                    // Send success reasoning
                                    let success_reasoning = format!(
                                    "   ✅ Filter '{}' completed in {}ms\n   📊 Message count: {} → {}\n",
                                    filter_name,
                                    duration.as_millis(),
                                    chat_request.messages.len(),
                                    current_messages.len()
                                );
                                    let success_chunk =
                                        Self::create_reasoning_chunk(&success_reasoning);
                                    let _ = tx.send(Ok(Bytes::from(success_chunk))).await;
                                }
                            }
                            Err(e) => {
                                if debug_mode && is_streaming {
                                    // Send error reasoning
                                    let error_reasoning =
                                        format!("   ❌ Filter '{}' failed: {}\n", filter_name, e);
                                    let error_chunk =
                                        Self::create_reasoning_chunk(&error_reasoning);
                                    let _ = tx.send(Ok(Bytes::from(error_chunk))).await;
                                }
                                return;
                            }
                        }
                    } else {
                        if debug_mode && is_streaming {
                            // Send not found reasoning
                            let not_found_reasoning = format!(
                                "   ⚠️  Filter agent '{}' not found in agent map\n",
                                filter_name
                            );
                            let not_found_chunk =
                                Self::create_reasoning_chunk(&not_found_reasoning);
                            let _ = tx.send(Ok(Bytes::from(not_found_chunk))).await;
                        }
                    }
                }
            }

            if debug_mode && is_streaming {
                // Send terminal agent reasoning
                let terminal_reasoning = format!(
                    "\n🎯 Filter chain completed! Invoking terminal agent for final response...\n\n"
                );
                let terminal_chunk = Self::create_reasoning_chunk(&terminal_reasoning);
                let _ = tx.send(Ok(Bytes::from(terminal_chunk))).await;
            }

            // Get terminal agent and invoke it
            if let Some(terminal_agent) = agent_map.get(&selected_agent.id) {
                // Create request for terminal agent
                let mut terminal_request = chat_request;
                terminal_request.messages = current_messages;

                let request_body = match serde_json::to_string(&terminal_request) {
                    Ok(body) => body,
                    Err(_) => return,
                };

                info!("chat request string: {}", request_body);

                let client = reqwest::Client::new();
                let mut agent_headers = request_headers;
                info!("request headers: {:?}", agent_headers);
                agent_headers.remove(hyper::header::CONTENT_LENGTH);
                agent_headers.insert(
                    common::consts::ARCH_UPSTREAM_HOST_HEADER,
                    match hyper::header::HeaderValue::from_str(&terminal_agent.id) {
                        Ok(val) => val,
                        Err(_) => return,
                    },
                );
                info!("request headers after: {:?}", agent_headers);

                match client
                    .post(&url)
                    .headers(agent_headers)
                    .body(request_body)
                    .send()
                    .await
                {
                    Ok(response) => {
                        // Stream the terminal agent response
                        let mut byte_stream = response.bytes_stream();
                        while let Some(item) = byte_stream.next().await {
                            match item {
                                Ok(chunk) => {
                                    info!("Streaming terminal agent chunk len: {}", chunk.len());
                                    if tx.send(Ok(chunk)).await.is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    Err(e) => {
                        if debug_mode && is_streaming {
                            let error_reasoning = format!("❌ Terminal agent error: {}\n", e);
                            let error_chunk = Self::create_reasoning_chunk(&error_reasoning);
                            let _ = tx.send(Ok(Bytes::from(error_chunk))).await;
                        }
                    }
                }
            }
        });

        // Create streaming response
        let stream =
            ReceiverStream::new(rx).map(|chunk| Ok::<_, hyper::Error>(Frame::data(chunk?)));
        let stream_body = BoxBody::new(StreamBody::new(stream));

        Ok(Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .body(stream_body)?)
    }

    async fn process_single_filter(
        messages: &[hermesllm::apis::openai::Message],
        original_request: &ChatCompletionsRequest,
        agent: &Agent,
        request_headers: &hyper::HeaderMap,
        url: &str,
    ) -> Result<Vec<hermesllm::apis::openai::Message>, Box<dyn std::error::Error + Send + Sync>>
    {
        let mut request = original_request.clone();
        request.messages = messages.to_vec();

        let request_body = serde_json::to_string(&request)?;
        let client = reqwest::Client::new();

        let mut agent_headers = request_headers.clone();
        agent_headers.remove(hyper::header::CONTENT_LENGTH);
        agent_headers.insert(
            common::consts::ARCH_UPSTREAM_HOST_HEADER,
            hyper::header::HeaderValue::from_str(&agent.id)?,
        );

        agent_headers.insert(
            common::consts::ENVOY_RETRY_HEADER,
            hyper::header::HeaderValue::from_str("3")?,
        );

        let response = client
            .post(url)
            .headers(agent_headers)
            .body(request_body)
            .send()
            .await?;

        let response_bytes = response.bytes().await?;
        let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)?;

        let content = response_json
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or("No content in response")?;

        // Parse the response content as new message history
        let new_messages: Vec<hermesllm::apis::openai::Message> = serde_json::from_str(content)?;

        Ok(new_messages)
    }

    fn create_reasoning_chunk(reasoning_content: &str) -> String {
        let reasoning_chunk = serde_json::json!({
            "choices": [{
                "delta": {
                    "reasoning": reasoning_content
                },
                "index": 0,
                "finish_reason": null
            }],
            "created": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "id": format!("reasoning-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()),
            "model": "agent-pipeline",
            "object": "chat.completion.chunk"
        });

        format!("data: {}\n\n", reasoning_chunk)
    }
}

impl Default for ResponseHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::StatusCode;

    #[test]
    fn test_create_bad_request() {
        let response = ResponseHandler::create_bad_request("Invalid request");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_create_internal_error() {
        let response = ResponseHandler::create_internal_error("Server error");
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_create_error_response() {
        let response =
            ResponseHandler::create_error_response(StatusCode::NOT_FOUND, "Resource not found");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_create_json_error_response() {
        let error_json = serde_json::json!({
            "error": {
                "type": "TestError",
                "message": "Test error message"
            }
        });

        let response = ResponseHandler::create_json_error_response(&error_json);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }

    #[tokio::test]
    async fn test_create_streaming_response_with_mock() {
        use mockito::Server;

        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("streaming response")
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let llm_response = client.get(&(server.url() + "/test")).send().await.unwrap();

        let handler = ResponseHandler::new();
        let result = handler.create_streaming_response(llm_response).await;

        mock.assert_async().await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("content-type"));
    }
}
