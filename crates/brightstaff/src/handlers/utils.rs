use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::StreamBody;
use hyper::body::Frame;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::warn;

/// Trait for processing streaming chunks
/// Implementors can inject custom logic during streaming (e.g., hallucination detection, logging)
pub trait StreamProcessor: Send + 'static {
    /// Process an incoming chunk of bytes
    fn process_chunk(&mut self, chunk: Bytes) -> Result<Option<Bytes>, String>;

    /// Called when streaming completes successfully
    fn on_complete(&mut self) {}

    /// Called when streaming encounters an error
    fn on_error(&mut self, _error: &str) {}
}

/// A no-op processor that just forwards chunks as-is
pub struct PassthroughProcessor;

impl StreamProcessor for PassthroughProcessor {
    fn process_chunk(&mut self, chunk: Bytes) -> Result<Option<Bytes>, String> {
        Ok(Some(chunk))
    }
}

/// Result of creating a streaming response
pub struct StreamingResponse {
    pub body: BoxBody<Bytes, hyper::Error>,
    pub processor_handle: tokio::task::JoinHandle<()>,
}

pub fn create_streaming_response<S, P>(
    mut byte_stream: S,
    mut processor: P,
    buffer_size: usize,
) -> StreamingResponse
where
    S: StreamExt<Item = Result<Bytes, reqwest::Error>> + Send + Unpin + 'static,
    P: StreamProcessor,
{
    let (tx, rx) = mpsc::channel::<Bytes>(buffer_size);

    // Spawn a task to process and forward chunks
    let processor_handle = tokio::spawn(async move {
        while let Some(item) = byte_stream.next().await {
            let chunk = match item {
                Ok(chunk) => chunk,
                Err(err) => {
                    let err_msg = format!("Error receiving chunk: {:?}", err);
                    warn!("{}", err_msg);
                    processor.on_error(&err_msg);
                    break;
                }
            };

            // Process the chunk
            match processor.process_chunk(chunk) {
                Ok(Some(processed_chunk)) => {
                    if tx.send(processed_chunk).await.is_err() {
                        warn!("Receiver dropped");
                        break;
                    }
                }
                Ok(None) => {
                    // Skip this chunk
                    continue;
                }
                Err(err) => {
                    warn!("Processor error: {}", err);
                    processor.on_error(&err);
                    break;
                }
            }
        }

        processor.on_complete();
    });

    // Convert channel receiver to HTTP stream
    let stream = ReceiverStream::new(rx).map(|chunk| Ok::<_, hyper::Error>(Frame::data(chunk)));
    let stream_body = BoxBody::new(StreamBody::new(stream));

    StreamingResponse {
        body: stream_body,
        processor_handle,
    }
}
