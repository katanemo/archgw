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
pub enum AgentChatError {
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
            warn!("Agent chat error: {}", err);
            Ok(ResponseHandler::create_internal_error(&format!(
                "Internal error: {}",
                err
            )))
        }
    }
}

async fn handle_agent_chat(
    request: Request<hyper::body::Incoming>,
    router_service: Arc<RouterService>,
    agents_list: Arc<tokio::sync::RwLock<Option<Vec<common::configuration::Agent>>>>,
    listeners: Arc<tokio::sync::RwLock<Vec<common::configuration::Listener>>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, AgentChatError> {
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
            AgentChatError::RequestParsing(err)
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

    debug!("Processing agent pipeline: {}", selected_agent.name);

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
    let terminal_agent_name = selected_agent.agent;
    let terminal_agent = agent_map.get(&terminal_agent_name).unwrap();

    debug!("Processing terminal agent: {}", terminal_agent_name);
    debug!("Terminal agent details: {:?}", terminal_agent);

    let llm_response = pipeline_processor
        .send_terminal_request(
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
        .map_err(AgentChatError::from)
}
