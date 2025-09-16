use std::sync::Arc;

use bytes::Bytes;
use common::api::open_ai::{ChatCompletionsResponse, Choice};
use common::configuration::{AgentPipeline, ModelUsagePreference, RoutingPreference};
use common::consts::{ARCH_PROVIDER_HINT_HEADER, ARCH_UPSTREAM_HOST_HEADER};
use hermesllm::apis::openai::ChatCompletionsRequest;
use hermesllm::apis::{Role, Usage};
use hermesllm::clients::SupportedAPIs;
use hermesllm::{ProviderRequest, ProviderRequestType};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::header::{self};
use hyper::{Request, Response, StatusCode, Uri};
use serde::{ser::SerializeMap, Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};

use crate::router::llm_router::RouterService;

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub async fn agent_chat(
    request: Request<hyper::body::Incoming>,
    router_service: Arc<RouterService>,
    full_qualified_llm_provider_url: String,
    agents_list: Arc<tokio::sync::RwLock<Option<Vec<common::configuration::Agent>>>>,
    listeners: Arc<tokio::sync::RwLock<Vec<common::configuration::Listener>>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    // find listener that is running at port 8001 for agents
    let listener_name = request.headers().get("x-arch-agent-listener-name");
    let listener = {
        let listeners = listeners.read().await;
        listeners
            .iter()
            .find(|l| {
                listener_name
                    .and_then(|name| name.to_str().ok())
                    .map(|name| l.name == name)
                    .unwrap_or(false)
            })
            .cloned()
    }
    .unwrap();

    info!("Handling request for listener: {}", listener.name);

    let request_path = request.uri().path().to_string();
    let mut request_headers = request.headers().clone();
    let chat_request_bytes = request.collect().await?.to_bytes();

    debug!(
        "Received request body (raw utf8): {}",
        String::from_utf8_lossy(&chat_request_bytes)
    );

    let chat_completions_request: ChatCompletionsRequest =
        match serde_json::from_slice(&chat_request_bytes) {
            Ok(req) => req,
            Err(err) => {
                warn!(
                    "Failed to parse request body as ChatCompletionsRequest: {}",
                    err
                );
                let err_msg = format!("Failed to parse request body: {}", err);
                let mut bad_request = Response::new(full(err_msg));
                *bad_request.status_mut() = StatusCode::BAD_REQUEST;
                return Ok(bad_request);
            }
        };

    let agent_name_map = {
        let agents = agents_list.read().await;
        let agents = agents.as_ref().unwrap();
        let mut map = std::collections::HashMap::new();
        for agent in agents.iter() {
            map.insert(agent.name.clone(), agent.clone());
        }
        map
    };

    let trace_parent = request_headers
        .iter()
        .find(|(ty, _)| ty.as_str() == "traceparent")
        .map(|(_, value)| value.to_str().unwrap_or_default().to_string());

    let agents_usage_preferences: Vec<ModelUsagePreference> =
        convert_agent_description_to_routing_preferences(&listener.agents.as_ref().unwrap());

    debug!(
        "Agents usage preferences for agent routing: {:?}",
        agents_usage_preferences
    );

    let agent_pipeline = match agents_usage_preferences.len() > 1 {
        false => {
            debug!("Only one agent available, skipping routing");
            listener.agents.as_ref().unwrap()[0].clone()
        }
        true => {
            let selected_agent = match router_service
                .determine_route(
                    &chat_completions_request.messages,
                    trace_parent.clone(),
                    Some(agents_usage_preferences),
                )
                .await
            {
                Ok(route) => {
                    match route {
                        Some((_, agent_name)) => {
                            debug!("Determined agent: {}", agent_name);
                            listener
                                .agents
                                .as_ref()
                                .unwrap()
                                .iter()
                                .find(|a| a.name == agent_name)
                                .cloned()
                                // selected agent must exist in the agent map
                                .unwrap()
                        }
                        None => {
                            debug!("No agent determined using routing preferences, using default agent");
                            listener
                                .agents
                                .as_ref()
                                .unwrap()
                                .iter()
                                .find(|a| a.default.unwrap_or(false))
                                .cloned()
                                .unwrap_or_else(|| {
                                    warn!(
                                    "No default agent found, routing request to first agent: {}",
                                    listener.agents.as_ref().unwrap()[0].name
                                );
                                    listener.agents.as_ref().unwrap()[0].clone()
                                })
                        }
                    }
                }
                Err(err) => {
                    let err_msg = format!("Failed to determine route: {}", err);
                    let mut internal_error = Response::new(full(err_msg));
                    *internal_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return Ok(internal_error);
                }
            };
            selected_agent
        }
    };

    debug!("Processing agent pipeline: {}", agent_pipeline.name);

    let mut chat_completions_history = chat_completions_request.messages.clone();

    request_headers.remove(header::CONTENT_LENGTH);

    for agent_name in agent_pipeline.filter_chain {
        debug!("Processing agent: {}", agent_name);
        let agent = agent_name_map.get(&agent_name).unwrap();
        debug!("Agent details: {:?}", agent);

        let mut request = chat_completions_request.clone();
        request.messages = chat_completions_history.clone();

        let request_str = serde_json::to_string(&request).unwrap();
        debug!("Sending request to agent {}: {}", agent_name, request_str);

        let mut agent_request_headers = request_headers.clone();
        agent_request_headers.insert(
            ARCH_UPSTREAM_HOST_HEADER,
            hyper::header::HeaderValue::from_str(agent.name.as_str()).unwrap(),
        );

        let response = match reqwest::Client::new()
            .post("http://localhost:11000/v1/chat/completions")
            .headers(agent_request_headers)
            .body(request_str)
            .send()
            .await
        {
            Ok(res) => res,
            Err(err) => {
                let err_msg = format!("Failed to send request: {}", err);
                let mut internal_error = Response::new(full(err_msg));
                *internal_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                return Ok(internal_error);
            }
        };

        let response_bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => {
                let err_msg = format!("Failed to read response bytes: {}", err);
                let mut internal_error = Response::new(full(err_msg));
                *internal_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                return Ok(internal_error);
            }
        };

        let chat_completions_response: hermesllm::apis::openai::ChatCompletionsResponse =
            match serde_json::from_slice(&response_bytes) {
                Ok(res) => res,
                Err(err) => {
                    let err_msg = format!("Failed to parse response body: {}", err);
                    let mut internal_error = Response::new(full(err_msg));
                    *internal_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return Ok(internal_error);
                }
            };

        let response_str = chat_completions_response.choices[0]
            .message
            .content
            .clone()
            .unwrap();

        debug!(
            "Received response from agent {}: {}",
            agent_name, response_str
        );

        chat_completions_history = serde_json::from_str(response_str.as_str()).unwrap_or(vec![]);
    }

    let last_response: Option<String> = match chat_completions_history.last() {
        Some(msg) => Some(msg.content.clone().to_string()),
        None => None,
    };

    let chat_completion_response: hermesllm::apis::openai::ChatCompletionsResponse =
        hermesllm::apis::openai::ChatCompletionsResponse {
            model: "arch-agent".to_string(),
            choices: vec![hermesllm::apis::openai::Choice {
                message: {
                    hermesllm::apis::openai::ResponseMessage {
                        role: hermesllm::apis::openai::Role::Assistant,
                        content: last_response,
                        ..Default::default()
                    }
                },
                ..Default::default()
            }],
            usage: hermesllm::apis::openai::Usage {
                ..Default::default()
            },
            ..Default::default()
        };

    let response_body = serde_json::to_string(&chat_completion_response).unwrap();

    return Ok(Response::new(full(response_body)));
}

fn convert_agent_description_to_routing_preferences(
    agents: &Vec<AgentPipeline>,
) -> Vec<ModelUsagePreference> {
    agents
        .iter()
        .map(|agent| ModelUsagePreference {
            model: agent.name.clone(),
            routing_preferences: vec![RoutingPreference {
                name: agent.name.clone(),
                description: agent
                    .description
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .clone(),
            }],
        })
        .collect()
}
