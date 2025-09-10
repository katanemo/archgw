use std::sync::Arc;

use bytes::Bytes;
use common::configuration::ModelUsagePreference;
use common::consts::ARCH_PROVIDER_HINT_HEADER;
use hermesllm::apis::openai::ChatCompletionsRequest;
use hermesllm::clients::SupportedAPIs;
use hermesllm::{ProviderRequest, ProviderRequestType};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::header::{self};
use hyper::{Request, Response, StatusCode};
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

pub async fn chat(
    request: Request<hyper::body::Incoming>,
    router_service: Arc<RouterService>,
    full_qualified_llm_provider_url: String,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {

    let request_path = request.uri().path().to_string();
    let mut request_headers = request.headers().clone();
    let chat_request_bytes = request.collect().await?.to_bytes();

    debug!("Received request body (raw utf8): {}", String::from_utf8_lossy(&chat_request_bytes));
    let mut client_request = match ProviderRequestType::try_from((&chat_request_bytes[..], &SupportedAPIs::from_endpoint(request_path.as_str()).unwrap())) {
        Ok(request) => request,
        Err(err) => {
            warn!("Failed to parse request as ProviderRequestType: {}", err);
            let err_msg = format!("Failed to parse request: {}", err);
            let mut bad_request = Response::new(full(err_msg));
            *bad_request.status_mut() = StatusCode::BAD_REQUEST;
            return Ok(bad_request);
        }
    };

    // Clone metadata for routing and remove archgw_preference_config from original
    let routing_metadata = client_request.metadata().clone();

    if client_request.remove_metadata_key("archgw_preference_config") {
        debug!("Removed archgw_preference_config from metadata");
    }

    let client_request_bytes_for_upstream = ProviderRequestType::to_bytes(&client_request).unwrap();

    // Convert to ChatCompletionsRequest regardless of input type (clone to avoid moving original)
    let chat_completions_request_for_arch_router: ChatCompletionsRequest =
        match ProviderRequestType::try_from((client_request, &SupportedAPIs::OpenAIChatCompletions(hermesllm::apis::OpenAIApi::ChatCompletions))) {
            Ok(ProviderRequestType::ChatCompletionsRequest(req)) => req,
            Ok(ProviderRequestType::MessagesRequest(_)) => {
                // This should not happen after conversion to OpenAI format
                warn!("Unexpected: got MessagesRequest after converting to OpenAI format");
                let err_msg = "Request conversion failed".to_string();
                let mut bad_request = Response::new(full(err_msg));
                *bad_request.status_mut() = StatusCode::BAD_REQUEST;
                return Ok(bad_request);
            },
            Err(err) => {
                warn!("Failed to convert request to ChatCompletionsRequest: {}", err);
                let err_msg = format!("Failed to convert request: {}", err);
                let mut bad_request = Response::new(full(err_msg));
                *bad_request.status_mut() = StatusCode::BAD_REQUEST;
                return Ok(bad_request);
            }
        };

    debug!(
        "[BRIGHTSTAFF -> ARCH_ROUTER] REQ: {}",
        &serde_json::to_string(&chat_completions_request_for_arch_router).unwrap()
    );

    let trace_parent = request_headers
        .iter()
        .find(|(ty, _)| ty.as_str() == "traceparent")
        .map(|(_, value)| value.to_str().unwrap_or_default().to_string());

    let usage_preferences_str: Option<String> =
        routing_metadata.as_ref().and_then(|metadata| {
            metadata
                .get("archgw_preference_config")
                .map(|value| value.to_string())
        });

    let usage_preferences: Option<Vec<ModelUsagePreference>> = usage_preferences_str
        .as_ref()
        .and_then(|s| serde_yaml::from_str(s).ok());

    let latest_message_for_log =
        chat_completions_request_for_arch_router
            .messages
            .last()
            .map_or("None".to_string(), |msg| {
                msg.content.to_string().replace('\n', "\\n")
            });

    const MAX_MESSAGE_LENGTH: usize = 50;
    let latest_message_for_log = if latest_message_for_log.len() > MAX_MESSAGE_LENGTH {
        format!("{}...", &latest_message_for_log[..MAX_MESSAGE_LENGTH])
    } else {
        latest_message_for_log
    };

    info!(
        "request received, request type: chat_completion, usage preferences from request: {}, request path: {}, latest message: {}",
        usage_preferences.is_some(),
        request_path,
        latest_message_for_log
    );

    debug!("usage preferences from request: {:?}", usage_preferences);

    let model_name = match router_service
        .determine_route(
            &chat_completions_request_for_arch_router.messages,
            trace_parent.clone(),
            usage_preferences,
        )
        .await
    {
        Ok(route) => match route {
            Some((_, model_name)) => model_name,
            None => {
                debug!(
                    "No route determined, using default model from request: {}",
                    chat_completions_request_for_arch_router.model
                );
                chat_completions_request_for_arch_router.model.clone()
            }
        },
        Err(err) => {
            let err_msg = format!("Failed to determine route: {}", err);
            let mut internal_error = Response::new(full(err_msg));
            *internal_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            return Ok(internal_error);
        }
    };

    debug!(
        "[BRIGHTSTAFF -> ARCH_ROUTER] URL: {}, Model Hint: {}",
        full_qualified_llm_provider_url, model_name
    );

    request_headers.insert(
        ARCH_PROVIDER_HINT_HEADER,
        header::HeaderValue::from_str(&model_name).unwrap(),
    );

    if let Some(trace_parent) = trace_parent {
        request_headers.insert(
            header::HeaderName::from_static("traceparent"),
            header::HeaderValue::from_str(&trace_parent).unwrap(),
        );
    }
    // remove content-length header if it exists
    request_headers.remove(header::CONTENT_LENGTH);

    let llm_response = match reqwest::Client::new()
        .post(full_qualified_llm_provider_url)
        .headers(request_headers)
        .body(client_request_bytes_for_upstream)
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

    // copy over the headers from the original response
    let response_headers = llm_response.headers().clone();
    let mut response = Response::builder();
    let headers = response.headers_mut().unwrap();
    for (header_name, header_value) in response_headers.iter() {
        headers.insert(header_name, header_value.clone());
    }

    // channel to create async stream
    let (tx, rx) = mpsc::channel::<Bytes>(16);

    // Spawn a task to send data as it becomes available
    tokio::spawn(async move {
        let mut byte_stream = llm_response.bytes_stream();

        while let Some(item) = byte_stream.next().await {
            let item = match item {
                Ok(item) => item,
                Err(err) => {
                    warn!("Error receiving chunk: {:?}", err);
                    break;
                }
            };

            if tx.send(item).await.is_err() {
                warn!("Receiver dropped");
                break;
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(|chunk| Ok::<_, hyper::Error>(Frame::data(chunk)));

    let stream_body = BoxBody::new(StreamBody::new(stream));

    match response.body(stream_body) {
        Ok(response) => Ok(response),
        Err(err) => {
            let err_msg = format!("Failed to create response: {}", err);
            let mut internal_error = Response::new(full(err_msg));
            *internal_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(internal_error)
        }
    }
}
