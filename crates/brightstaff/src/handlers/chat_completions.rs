use std::sync::Arc;

use bytes::Bytes;
use common::configuration::ModelUsagePreference;
use common::consts::ARCH_PROVIDER_HINT_HEADER;
use hermesllm::apis::openai::ChatCompletionsRequest;
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

pub async fn chat_completions(
    request: Request<hyper::body::Incoming>,
    router_service: Arc<RouterService>,
    llm_provider_endpoint: String,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let request_path = request.uri().path().to_string();
    let mut request_headers = request.headers().clone();

    let chat_request_bytes = request.collect().await?.to_bytes();

    debug!("Received request body (raw utf8): {}", String::from_utf8_lossy(&chat_request_bytes));

    let chat_request_parsed = serde_json::from_slice::<serde_json::Value>(&chat_request_bytes)
        .inspect_err(|err| {
            warn!(
                "Failed to parse request body as JSON: err: {}, str: {}",
                err,
                String::from_utf8_lossy(&chat_request_bytes)
            )
        })
        .unwrap_or_else(|_| {
            warn!(
                "Failed to parse request body as JSON: {}",
                String::from_utf8_lossy(&chat_request_bytes)
            );
            serde_json::Value::Null
        });

    if chat_request_parsed == serde_json::Value::Null {
        warn!("Request body is not valid JSON");
        let err_msg = "Request body is not valid JSON".to_string();
        let mut bad_request = Response::new(full(err_msg));
        *bad_request.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(bad_request);
    }

    let chat_completion_request: ChatCompletionsRequest =
        serde_json::from_value(chat_request_parsed.clone()).unwrap();

    // remove metadata from the request
    let mut chat_request_user_preferences_removed = chat_request_parsed;
    if let Some(metadata) = chat_request_user_preferences_removed.get_mut("metadata") {
        debug!("Removing metadata from request");
        if let Some(m) = metadata.as_object_mut() {
            m.remove("archgw_preference_config");
            debug!("Removed archgw_preference_config from metadata");
        }

        // if metadata is empty, remove it
        if metadata.as_object().map_or(false, |m| m.is_empty()) {
            debug!("Removing empty metadata from request");
            chat_request_user_preferences_removed
                .as_object_mut()
                .map(|m| m.remove("metadata"));
        }
    }

    debug!(
        "arch-router request received: {}",
        &serde_json::to_string(&chat_completion_request).unwrap()
    );

    let trace_parent = request_headers
        .iter()
        .find(|(ty, _)| ty.as_str() == "traceparent")
        .map(|(_, value)| value.to_str().unwrap_or_default().to_string());

    let usage_preferences_str: Option<String> =
        chat_completion_request.metadata.and_then(|metadata| {
            metadata
                .get("archgw_preference_config")
                .map(|value| value.to_string())
        });

    let usage_preferences: Option<Vec<ModelUsagePreference>> = usage_preferences_str
        .as_ref()
        .and_then(|s| serde_yaml::from_str(s).ok());

    let latest_message_for_log =
        chat_completion_request
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
            &chat_completion_request.messages,
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
                    chat_completion_request.model
                );
                chat_completion_request.model.clone()
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
        "sending request to llm provider: {}, with model hint: {}",
        llm_provider_endpoint, model_name
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

    let chat_request_parsed_bytes =
        serde_json::to_string(&chat_request_user_preferences_removed).unwrap();

    // remove content-length header if it exists
    request_headers.remove(header::CONTENT_LENGTH);

    let llm_response = match reqwest::Client::new()
        .post(llm_provider_endpoint)
        .headers(request_headers)
        .body(chat_request_parsed_bytes)
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
