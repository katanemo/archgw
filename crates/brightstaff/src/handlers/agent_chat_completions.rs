use std::sync::Arc;
use std::time::{Instant, SystemTime};

use bytes::Bytes;
use common::consts::TRACE_PARENT_HEADER;
use common::traces::{SpanBuilder, SpanKind, parse_traceparent, generate_random_span_id};
use hermesllm::apis::OpenAIMessage;
use hermesllm::clients::SupportedAPIsFromClient;
use hermesllm::providers::request::ProviderRequest;
use hermesllm::ProviderRequestType;
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::{Request, Response};
use serde::ser::Error as SerError;
use tracing::{debug, info, warn};

use super::agent_selector::{AgentSelectionError, AgentSelector};
use super::pipeline_processor::{PipelineError, PipelineProcessor};
use super::response_handler::ResponseHandler;
use crate::router::llm_router::RouterService;
use crate::tracing::{OperationNameBuilder, operation_component, http};

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
    trace_collector: Arc<common::traces::TraceCollector>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match handle_agent_chat(
        request,
        router_service,
        agents_list,
        listeners,
        trace_collector,
    )
    .await
    {
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
    trace_collector: Arc<common::traces::TraceCollector>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, AgentFilterChainError> {
    // Initialize services
    let agent_selector = AgentSelector::new(router_service);
    let mut pipeline_processor = PipelineProcessor::default();
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
    let request_path = request
        .uri()
        .path()
        .to_string()
        .strip_prefix("/agents")
        .unwrap()
        .to_string();
    let request_headers = request.headers().clone();
    let chat_request_bytes = request.collect().await?.to_bytes();

    debug!(
        "Received request body (raw utf8): {}",
        String::from_utf8_lossy(&chat_request_bytes)
    );

    // Determine the API type from the endpoint
    let api_type =
        SupportedAPIsFromClient::from_endpoint(request_path.as_str()).ok_or_else(|| {
            let err_msg = format!("Unsupported endpoint: {}", request_path);
            warn!("{}", err_msg);
            AgentFilterChainError::RequestParsing(serde_json::Error::custom(err_msg))
        })?;

    let client_request = match ProviderRequestType::try_from((&chat_request_bytes[..], &api_type)) {
        Ok(request) => request,
        Err(err) => {
            warn!("Failed to parse request as ProviderRequestType: {}", err);
            let err_msg = format!("Failed to parse request: {}", err);
            return Err(AgentFilterChainError::RequestParsing(
                serde_json::Error::custom(err_msg),
            ));
        }
    };

    let message: Vec<OpenAIMessage> = client_request.get_messages();

    // let chat_completions_request: ChatCompletionsRequest =
    //     serde_json::from_slice(&chat_request_bytes).map_err(|err| {
    //         warn!(
    //             "Failed to parse request body as ChatCompletionsRequest: {}",
    //             err
    //         );
    //         AgentFilterChainError::RequestParsing(err)
    //     })?;

    // Extract trace parent for routing
    let trace_parent = request_headers
        .iter()
        .find(|(key, _)| key.as_str() == TRACE_PARENT_HEADER)
        .map(|(_, value)| value.to_str().unwrap_or_default().to_string());

    // Create agent map for pipeline processing and agent selection
    let agent_map = {
        let agents = agents_list.read().await;
        let agents = agents.as_ref().unwrap();
        agent_selector.create_agent_map(agents)
    };

    // Parse trace parent to get trace_id and parent_span_id
    let (trace_id, parent_span_id) = if let Some(ref tp) = trace_parent {
        parse_traceparent(tp)
    } else {
        (String::new(), None)
    };

    // Select appropriate agent using arch router llm model
    let selected_agent = agent_selector
        .select_agent(&message, &listener, trace_parent.clone())
        .await?;

    debug!("Processing agent pipeline: {}", selected_agent.id);

    // Record the start time for agent span
    let agent_start_time = SystemTime::now();
    let agent_start_instant = Instant::now();
    // let (span_id, trace_id) = trace_collector.start_span(
    //     trace_parent.clone(),
    //     operation_component::AGENT,
    //     &format!("/agents{}", request_path),
    //     &selected_agent.id,
    // );

    let span_id = generate_random_span_id();

    // Process the filter chain
    let chat_history = pipeline_processor
        .process_filter_chain(
            &message,
            &selected_agent,
            &agent_map,
            &request_headers,
            Some(&trace_collector),
            trace_id.clone(),
            span_id.clone(),
        )
        .await?;

    // Get terminal agent and send final response
    let terminal_agent_name = selected_agent.id.clone();
    let terminal_agent = agent_map.get(&terminal_agent_name).unwrap();

    debug!("Processing terminal agent: {}", terminal_agent_name);
    debug!("Terminal agent details: {:?}", terminal_agent);

    let llm_response = pipeline_processor
        .invoke_agent(
            &chat_history,
            client_request,
            terminal_agent,
            &request_headers,
            trace_id.clone(),
            span_id.clone(),
        )
        .await?;

    // Record agent span after processing is complete
    let agent_end_time = SystemTime::now();
    let agent_elapsed = agent_start_instant.elapsed();

    // Build full path with /agents prefix
    let full_path = format!("/agents{}", request_path);

    // Build operation name: POST {full_path} {agent_name}
    let operation_name = OperationNameBuilder::new()
        .with_method("POST")
        .with_path(&full_path)
        .with_target(&terminal_agent_name)
        .build();

    let mut span_builder = SpanBuilder::new(&operation_name)
        .with_span_id(span_id)
        .with_kind(SpanKind::Internal)
        .with_start_time(agent_start_time)
        .with_end_time(agent_end_time)
        .with_attribute(http::METHOD, "POST")
        .with_attribute(http::TARGET, full_path)
        .with_attribute("agent.name", terminal_agent_name.clone())
        .with_attribute("duration_ms", format!("{:.2}", agent_elapsed.as_secs_f64() * 1000.0));

    if !trace_id.is_empty() {
        span_builder = span_builder.with_trace_id(trace_id);
    }
    if let Some(parent_id) = parent_span_id {
        span_builder = span_builder.with_parent_span_id(parent_id);
    }

    let span = span_builder.build();
    // Use plano(agent) as service name for the agent processing span
    trace_collector.record_span(operation_component::AGENT, span);

    // Create streaming response
    response_handler
        .create_streaming_response(llm_response)
        .await
        .map_err(AgentFilterChainError::from)
}
