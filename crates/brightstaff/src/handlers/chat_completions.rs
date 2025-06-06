use std::sync::Arc;

use bytes::Bytes;
use common::consts::ARCH_PROVIDER_HINT_HEADER;
use hermesllm::providers::openai::types::ChatCompletionsRequest;
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
    let mut request_headers = request.headers().clone();

    let chat_request_bytes = request.collect().await?.to_bytes();

    let chat_completion_request: ChatCompletionsRequest =
        match ChatCompletionsRequest::try_from(chat_request_bytes.as_ref()) {
            Ok(request) => request,
            Err(err) => {
                warn!(
                    "arch-router request body string: {}",
                    String::from_utf8_lossy(&chat_request_bytes)
                );
                let err_msg = format!("Failed to parse request body: {}", err);
                warn!("{}", err_msg);
                let mut bad_request = Response::new(full(err_msg));
                *bad_request.status_mut() = StatusCode::BAD_REQUEST;
                return Ok(bad_request);
            }
        };

    debug!(
        "arch-router request body: {}",
        &serde_json::to_string(&chat_completion_request).unwrap()
    );

    let trace_parent = request_headers
        .iter()
        .find(|(ty, _)| ty.as_str() == "traceparent")
        .map(|(_, value)| value.to_str().unwrap_or_default().to_string());

    let mut selected_llm = match router_service
        .determine_route(&chat_completion_request.messages, trace_parent.clone())
        .await
    {
        Ok(route) => route,
        Err(err) => {
            let err_msg = format!("Failed to determine route: {}", err);
            let mut internal_error = Response::new(full(err_msg));
            *internal_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            return Ok(internal_error);
        }
    };

    if selected_llm.is_none() {
        debug!("No LLM model selected, using default from request");
        selected_llm = Some(chat_completion_request.model.clone());
    }

    info!(
        "sending request to llm provider: {} with llm model: {:?}",
        llm_provider_endpoint, selected_llm
    );

    if let Some(trace_parent) = trace_parent {
        request_headers.insert(
            header::HeaderName::from_static("traceparent"),
            header::HeaderValue::from_str(&trace_parent).unwrap(),
        );
    }

    if let Some(selected_llm) = selected_llm {
        request_headers.insert(
            ARCH_PROVIDER_HINT_HEADER,
            header::HeaderValue::from_str(&selected_llm).unwrap(),
        );
    }

    let llm_response = match reqwest::Client::new()
        .post(llm_provider_endpoint)
        .headers(request_headers)
        .body(chat_request_bytes)
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
