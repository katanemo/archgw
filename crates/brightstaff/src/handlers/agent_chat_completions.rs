use std::sync::Arc;

use bytes::Bytes;
use hermesllm::apis::openai::ChatCompletionsRequest;
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::{Request, Response};
use tracing::{debug, info, warn};

use super::agent_selector::{AgentSelectionError, AgentSelector};
use super::pipeline_processor::{PipelineError, PipelineProcessor};
use super::response_handler::ResponseHandler;
use crate::router::llm_router::RouterService;

/// Main errors for agent chat completions
#[derive(Debug, thiserror::Error)]
pub enum AgentFilterChainError {
    #[error("Agent selection error: {0}")]
    Selection(#[from] AgentSelectionError),
    #[error("Pipeline processing error: {0}")]
    Pipeline(#[from] PipelineError),
    #[error("Response handling error: {0}")]
    Response(#[from] super::response_handler::ResponseError),
    #[error("Request parsing error: {0}")]
    RequestParsing(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),
}

pub async fn agent_chat(
    request: Request<hyper::body::Incoming>,
    router_service: Arc<RouterService>,
    _: String,
    agents_list: Arc<tokio::sync::RwLock<Option<Vec<common::configuration::Agent>>>>,
    listeners: Arc<tokio::sync::RwLock<Vec<common::configuration::Listener>>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match handle_agent_chat(request, router_service, agents_list, listeners).await {
        Ok(response) => Ok(response),
        Err(err) => {
            // Check if this is a client error from the pipeline that should be cascaded
            if let AgentFilterChainError::Pipeline(PipelineError::ClientError {
                agent,
                status,
                body,
            }) = &err
            {
                warn!(
                    "Client error from agent '{}' (HTTP {}): {}",
                    agent, status, body
                );

                // Create error response with the original status code and body
                let error_json = serde_json::json!({
                    "error": "ClientError",
                    "agent": agent,
                    "status": status,
                    "agent_response": body
                });

                let json_string = error_json.to_string();
                let mut response = Response::new(ResponseHandler::create_full_body(json_string));
                *response.status_mut() = hyper::StatusCode::from_u16(*status)
                    .unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);
                response.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    "application/json".parse().unwrap(),
                );
                return Ok(response);
            }

            // Print detailed error information with full error chain for other errors
            let mut error_chain = Vec::new();
            let mut current_error: &dyn std::error::Error = &err;

            // Collect the full error chain
            loop {
                error_chain.push(current_error.to_string());
                match current_error.source() {
                    Some(source) => current_error = source,
                    None => break,
                }
            }

            // Log the complete error chain
            warn!("Agent chat error chain: {:#?}", error_chain);
            warn!("Root error: {:?}", err);

            // Create structured error response as JSON
            let error_json = serde_json::json!({
                "error": {
                    "type": "AgentFilterChainError",
                    "message": err.to_string(),
                    "error_chain": error_chain,
                    "debug_info": format!("{:?}", err)
                }
            });

            // Log the error for debugging
            info!("Structured error info: {}", error_json);

            // Return JSON error response
            Ok(ResponseHandler::create_json_error_response(&error_json))
        }
    }
}

async fn handle_agent_chat(
    request: Request<hyper::body::Incoming>,
    router_service: Arc<RouterService>,
    agents_list: Arc<tokio::sync::RwLock<Option<Vec<common::configuration::Agent>>>>,
    listeners: Arc<tokio::sync::RwLock<Vec<common::configuration::Listener>>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, AgentFilterChainError> {
    // Initialize services
    let agent_selector = AgentSelector::new(router_service);
    let pipeline_processor = PipelineProcessor::default();
    let response_handler = ResponseHandler::new();

    // Extract listener name from headers
    let listener_name = request
        .headers()
        .get("x-arch-agent-listener-name")
        .and_then(|name| name.to_str().ok());

    // Find the appropriate listener
    let listener = {
        let listeners = listeners.read().await;
        agent_selector
            .find_listener(listener_name, &listeners)
            .await?
    };

    info!("Handling request for listener: {}", listener.name);

    // Parse request body
    let request_headers = request.headers().clone();
    let chat_request_bytes = request.collect().await?.to_bytes();

    debug!(
        "Received request body (raw utf8): {}",
        String::from_utf8_lossy(&chat_request_bytes)
    );

    let chat_completions_request: ChatCompletionsRequest =
        serde_json::from_slice(&chat_request_bytes).map_err(|err| {
            warn!(
                "Failed to parse request body as ChatCompletionsRequest: {}",
                err
            );
            AgentFilterChainError::RequestParsing(err)
        })?;

    // Extract trace parent for routing
    let trace_parent = request_headers
        .iter()
        .find(|(key, _)| key.as_str() == "traceparent")
        .map(|(_, value)| value.to_str().unwrap_or_default().to_string());

    // Select appropriate agent using arch router llm model
    let selected_agent = agent_selector
        .select_agent(&chat_completions_request.messages, &listener, trace_parent)
        .await?;

    debug!("Processing agent pipeline: {}", selected_agent.id);

    // Create agent map for pipeline processing
    let agent_map = {
        let agents = agents_list.read().await;
        let agents = agents.as_ref().unwrap();
        agent_selector.create_agent_map(agents)
    };

    // Process the filter chain
    let processed_messages = pipeline_processor
        .process_filter_chain(
            &chat_completions_request,
            &selected_agent,
            &agent_map,
            &request_headers,
        )
        .await?;

    // Get terminal agent and send final response
    let terminal_agent_name = selected_agent.id;
    let terminal_agent = agent_map.get(&terminal_agent_name).unwrap();

    debug!("Processing terminal agent: {}", terminal_agent_name);
    debug!("Terminal agent details: {:?}", terminal_agent);

    let llm_response = pipeline_processor
        .invoke_upstream_agent(
            &processed_messages,
            &chat_completions_request,
            terminal_agent,
            &request_headers,
        )
        .await?;

    // Create streaming response
    response_handler
        .create_streaming_response(llm_response)
        .await
        .map_err(AgentFilterChainError::from)
}
