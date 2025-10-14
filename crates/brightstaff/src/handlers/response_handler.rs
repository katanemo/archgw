use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::{Response, StatusCode};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::warn;

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
