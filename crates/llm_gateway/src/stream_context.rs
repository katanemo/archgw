use crate::metrics::Metrics;
use common::configuration::{LlmProvider, LlmProviderType, Overrides};
use common::consts::{
    ARCH_PROVIDER_HINT_HEADER, ARCH_ROUTING_HEADER, CHAT_COMPLETIONS_PATH, HEALTHZ_PATH,
    RATELIMIT_SELECTOR_HEADER_KEY, REQUEST_ID_HEADER, TRACE_PARENT_HEADER,
};
use common::errors::ServerError;
use common::llm_providers::LlmProviders;
use common::ratelimit::Header;
use common::stats::{IncrementingMetric, RecordingMetric};
use common::tracing::{Event, Span, TraceData, Traceparent};
use common::{ratelimit, routing, tokenizer};
use hermesllm::providers::response::ProviderStreamResponseIter;
use hermesllm::{
    ProviderId, ProviderRequest, ProviderRequestType, ProviderResponse, ProviderResponseType,
};
use http::StatusCode;
use log::{debug, info, warn};
use proxy_wasm::hostcalls::get_current_time;
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use std::collections::VecDeque;
use std::num::NonZero;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct StreamContext {
    context_id: u32,
    metrics: Rc<Metrics>,
    ratelimit_selector: Option<Header>,
    streaming_response: bool,
    response_tokens: usize,
    is_chat_completions_request: bool,
    llm_providers: Rc<LlmProviders>,
    llm_provider: Option<Rc<LlmProvider>>,
    request_id: Option<String>,
    start_time: SystemTime,
    ttft_duration: Option<Duration>,
    ttft_time: Option<u128>,
    traceparent: Option<String>,
    request_body_sent_time: Option<u128>,
    traces_queue: Arc<Mutex<VecDeque<TraceData>>>,
    overrides: Rc<Option<Overrides>>,
    user_message: Option<String>,
}

impl StreamContext {
    pub fn new(
        context_id: u32,
        metrics: Rc<Metrics>,
        llm_providers: Rc<LlmProviders>,
        traces_queue: Arc<Mutex<VecDeque<TraceData>>>,
        overrides: Rc<Option<Overrides>>,
    ) -> Self {
        StreamContext {
            context_id,
            metrics,
            overrides,
            ratelimit_selector: None,
            streaming_response: false,
            response_tokens: 0,
            is_chat_completions_request: false,
            llm_providers,
            llm_provider: None,
            request_id: None,
            start_time: SystemTime::now(),
            ttft_duration: None,
            traceparent: None,
            ttft_time: None,
            traces_queue,
            request_body_sent_time: None,
            user_message: None,
        }
    }
    fn llm_provider(&self) -> &LlmProvider {
        self.llm_provider
            .as_ref()
            .expect("the provider should be set when asked for it")
    }

    fn get_provider_id(&self) -> ProviderId {
        self.llm_provider().to_provider_id()
    }

    fn select_llm_provider(&mut self) {
        let provider_hint = self
            .get_http_request_header(ARCH_PROVIDER_HINT_HEADER)
            .map(|llm_name| llm_name.into());

        self.llm_provider = Some(routing::get_llm_provider(
            &self.llm_providers,
            provider_hint,
        ));

        match self.llm_provider.as_ref().unwrap().provider_interface {
            LlmProviderType::Groq => {
                if let Some(path) = self.get_http_request_header(":path") {
                    if path.starts_with("/v1/") {
                        let new_path = format!("/openai{}", path);
                        self.set_http_request_header(":path", Some(new_path.as_str()));
                    }
                }
            }
            LlmProviderType::Gemini => {
                if let Some(path) = self.get_http_request_header(":path") {
                    if path == "/v1/chat/completions" {
                        self.set_http_request_header(
                            ":path",
                            Some("/v1beta/openai/chat/completions"),
                        );
                    }
                }
            }
            _ => {}
        }

        debug!(
            "request received: llm provider hint: {}, selected provider: {}",
            self.get_http_request_header(ARCH_PROVIDER_HINT_HEADER)
                .unwrap_or_default(),
            self.llm_provider.as_ref().unwrap().name
        );
    }

    fn modify_auth_headers(&mut self) -> Result<(), ServerError> {
        let llm_provider_api_key_value =
            self.llm_provider()
                .access_key
                .as_ref()
                .ok_or(ServerError::BadRequest {
                    why: format!(
                        "No access key configured for selected LLM Provider \"{}\"",
                        self.llm_provider()
                    ),
                })?;

        let authorization_header_value = format!("Bearer {}", llm_provider_api_key_value);

        self.set_http_request_header("Authorization", Some(&authorization_header_value));

        Ok(())
    }

    fn delete_content_length_header(&mut self) {
        // Remove the Content-Length header because further body manipulations in the gateway logic will invalidate it.
        // Server's generally throw away requests whose body length do not match the Content-Length header.
        // However, a missing Content-Length header is not grounds for bad requests given that intermediary hops could
        // manipulate the body in benign ways e.g., compression.
        self.set_http_request_header("content-length", None);
    }

    fn save_ratelimit_header(&mut self) {
        self.ratelimit_selector = self
            .get_http_request_header(RATELIMIT_SELECTOR_HEADER_KEY)
            .and_then(|key| {
                self.get_http_request_header(&key)
                    .map(|value| Header { key, value })
            });
    }

    fn send_server_error(&self, error: ServerError, override_status_code: Option<StatusCode>) {
        warn!("server error occurred: {}", error);
        self.send_http_response(
            override_status_code
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                .as_u16()
                .into(),
            vec![],
            Some(format!("{error}").as_bytes()),
        );
    }

    fn enforce_ratelimits(
        &mut self,
        model: &str,
        json_string: &str,
    ) -> Result<(), ratelimit::Error> {
        // Tokenize and record token count.
        let token_count = tokenizer::token_count(model, json_string).unwrap_or(0);

        debug!("Recorded input token count: {}", token_count);
        // Record the token count to metrics.
        self.metrics
            .input_sequence_length
            .record(token_count as u64);

        // Check if rate limiting needs to be applied.
        if let Some(selector) = self.ratelimit_selector.take() {
            log::debug!("Applying ratelimit for model: {}", model);
            ratelimit::ratelimits(None).read().unwrap().check_limit(
                model.to_owned(),
                selector,
                NonZero::new(token_count as u32).unwrap(),
            )?;
        } else {
            debug!("No rate limit applied for model: {}", model);
        }

        Ok(())
    }
}

// HttpContext is the trait that allows the Rust code to interact with HTTP objects.
impl HttpContext for StreamContext {
    // Envoy's HTTP model is event driven. The WASM ABI has given implementors events to hook onto
    // the lifecycle of the http request and response.
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        let request_path = self.get_http_request_header(":path").unwrap_or_default();
        if request_path == HEALTHZ_PATH {
            self.send_http_response(200, vec![], None);
            return Action::Continue;
        }

        self.is_chat_completions_request = CHAT_COMPLETIONS_PATH == request_path;

        let use_agent_orchestrator = match self.overrides.as_ref() {
            Some(overrides) => overrides.use_agent_orchestrator.unwrap_or_default(),
            None => false,
        };

        let routing_header_value = self.get_http_request_header(ARCH_ROUTING_HEADER);

        if routing_header_value.is_some() && !routing_header_value.as_ref().unwrap().is_empty() {
            let routing_header_value = routing_header_value.as_ref().unwrap();
            info!("routing header already set: {}", routing_header_value);
            self.llm_provider = Some(Rc::new(LlmProvider {
                name: routing_header_value.to_string(),
                provider_interface: LlmProviderType::OpenAI,
                ..Default::default()
            }));
        } else {
            self.select_llm_provider();
            if self.llm_provider().endpoint.is_some() {
                self.add_http_request_header(
                    ARCH_ROUTING_HEADER,
                    &self.llm_provider().name.to_string(),
                );
            } else {
                self.add_http_request_header(
                    ARCH_ROUTING_HEADER,
                    &self.llm_provider().provider_interface.to_string(),
                );
            }
            if let Err(error) = self.modify_auth_headers() {
                // ensure that the provider has an endpoint if the access key is missing else return a bad request
                if self.llm_provider.as_ref().unwrap().endpoint.is_none()
                    && !use_agent_orchestrator
                    && self.llm_provider.as_ref().unwrap().provider_interface
                        != LlmProviderType::Arch
                {
                    self.send_server_error(error, Some(StatusCode::BAD_REQUEST));
                }
            }
        }

        self.delete_content_length_header();
        self.save_ratelimit_header();

        self.request_id = self.get_http_request_header(REQUEST_ID_HEADER);
        self.traceparent = self.get_http_request_header(TRACE_PARENT_HEADER);

        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, end_of_stream: bool) -> Action {
        debug!(
            "on_http_request_body [S={}] bytes={} end_stream={}",
            self.context_id, body_size, end_of_stream
        );

        // Let the client send the gateway all the data before sending to the LLM_provider.
        // TODO: consider a streaming API.

        if self.request_body_sent_time.is_none() {
            self.request_body_sent_time = Some(current_time_ns());
        }

        if !end_of_stream {
            return Action::Pause;
        }

        if body_size == 0 {
            return Action::Continue;
        }

        let body_bytes = match self.get_http_request_body(0, body_size) {
            Some(body_bytes) => body_bytes,
            None => {
                self.send_server_error(
                    ServerError::LogicError(format!(
                        "Failed to obtain body bytes even though body_size is {}",
                        body_size
                    )),
                    None,
                );
                return Action::Pause;
            }
        };

        let provider_id = self.get_provider_id();

        let mut deserialized_body =
            match ProviderRequestType::try_from((&body_bytes[..], &provider_id)) {
                Ok(deserialized) => deserialized,
                Err(e) => {
                    debug!(
                        "on_http_request_body: request body: {}",
                        String::from_utf8_lossy(&body_bytes)
                    );
                    self.send_server_error(
                        ServerError::LogicError(format!("Request parsing error: {}", e)),
                        Some(StatusCode::BAD_REQUEST),
                    );
                    return Action::Pause;
                }
            };

        let model_name = match self.llm_provider.as_ref() {
            Some(llm_provider) => llm_provider.model.as_ref(),
            None => None,
        };

        let use_agent_orchestrator = match self.overrides.as_ref() {
            Some(overrides) => overrides.use_agent_orchestrator.unwrap_or_default(),
            None => false,
        };

        // Store the original model for logging
        let model_requested = deserialized_body.model().to_string();

        // Apply model name resolution logic using the trait method
        let resolved_model = match model_name {
            Some(model_name) => model_name.clone(),
            None => {
                if use_agent_orchestrator {
                    "agent_orchestrator".to_string()
                } else {
                    self.send_server_error(
                        ServerError::BadRequest {
                            why: format!(
                                "No model specified in request and couldn't determine model name from arch_config. Model name in req: {}, arch_config, provider: {}, model: {:?}",
                                model_requested,
                                self.llm_provider().name,
                                self.llm_provider().model
                            ),
                        },
                        Some(StatusCode::BAD_REQUEST),
                    );
                    return Action::Continue;
                }
            }
        };

        // Set the resolved model using the trait method
        deserialized_body.set_model(resolved_model.clone());

        // Extract user message for tracing
        self.user_message = deserialized_body.get_recent_user_message();

        info!(
            "on_http_request_body: provider: {}, model requested (in body): {}, model selected: {}",
            self.llm_provider().name,
            model_requested,
            model_name.unwrap_or(&"None".to_string()),
        );

        // Use provider interface for streaming detection and setup
        self.streaming_response = deserialized_body.is_streaming();

        // Use provider interface for text extraction (after potential mutation)
        let input_tokens_str = deserialized_body.extract_messages_text();
        // enforce ratelimits on ingress
        if let Err(e) = self.enforce_ratelimits(&resolved_model, input_tokens_str.as_str()) {
            self.send_server_error(
                ServerError::ExceededRatelimit(e),
                Some(StatusCode::TOO_MANY_REQUESTS),
            );
            self.metrics.ratelimited_rq.increment(1);
            return Action::Continue;
        }

        // Convert chat completion request to llm provider specific request using provider interface
        let deserialized_body_bytes = match deserialized_body.to_bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to serialize request body: {}", e);
                self.send_server_error(
                    ServerError::LogicError(format!("Request serialization error: {}", e)),
                    Some(StatusCode::BAD_REQUEST),
                );
                return Action::Pause;
            }
        };

        self.set_http_request_body(0, body_size, &deserialized_body_bytes);

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, end_of_stream: bool) -> Action {
        debug!(
            "on_http_response_headers [S={}] end_stream={}",
            self.context_id, end_of_stream
        );

        self.set_property(
            vec!["metadata", "filter_metadata", "llm_filter", "user_prompt"],
            Some("hello world from filter".as_bytes()),
        );

        Action::Continue
    }

    fn on_http_response_body(&mut self, body_size: usize, end_of_stream: bool) -> Action {
        debug!(
            "on_http_response_body [S={}] bytes={} end_stream={}",
            self.context_id, body_size, end_of_stream
        );

        if self.request_body_sent_time.is_none() {
            debug!("on_http_response_body: request body not sent, not doing any processing in llm filter");
            return Action::Continue;
        }

        if !self.is_chat_completions_request {
            info!("on_http_response_body: non-chatcompletion request");
            return Action::Continue;
        }

        let current_time = get_current_time().unwrap();
        if end_of_stream && body_size == 0 {
            // All streaming responses end with bytes=0 and end_stream=true
            // Record the latency for the request
            match current_time.duration_since(self.start_time) {
                Ok(duration) => {
                    // Convert the duration to milliseconds
                    let duration_ms = duration.as_millis();
                    info!("on_http_response_body: request latency: {}ms", duration_ms);
                    // Record the latency to the latency histogram
                    self.metrics.request_latency.record(duration_ms as u64);

                    if self.response_tokens > 0 {
                        // Compute the time per output token
                        let tpot = duration_ms as u64 / self.response_tokens as u64;

                        // Record the time per output token
                        self.metrics.time_per_output_token.record(tpot);

                        debug!(
                            "time per token: {}ms, tokens per second: {}",
                            tpot,
                            1000 / tpot
                        );
                        // Record the tokens per second
                        self.metrics.tokens_per_second.record(1000 / tpot);
                    }
                }
                Err(e) => {
                    warn!("SystemTime error: {:?}", e);
                }
            }
            // Record the output sequence length
            self.metrics
                .output_sequence_length
                .record(self.response_tokens as u64);

            if let Some(traceparent) = self.traceparent.as_ref() {
                let current_time_ns = current_time_ns();

                match Traceparent::try_from(traceparent.to_string()) {
                    Err(e) => {
                        warn!("traceparent header is invalid: {}", e);
                    }
                    Ok(traceparent) => {
                        let mut trace_data = common::tracing::TraceData::new();
                        let mut llm_span = Span::new(
                            "egress_traffic".to_string(),
                            Some(traceparent.trace_id),
                            Some(traceparent.parent_id),
                            self.request_body_sent_time.unwrap(),
                            current_time_ns,
                        );
                        llm_span.add_attribute(
                            "model".to_string(),
                            self.llm_provider().name.to_string(),
                        );

                        if let Some(user_message) = &self.user_message {
                            llm_span
                                .add_attribute("user_message".to_string(), user_message.clone());
                        }

                        if self.ttft_time.is_some() {
                            llm_span.add_event(Event::new(
                                "time_to_first_token".to_string(),
                                self.ttft_time.unwrap(),
                            ));
                            trace_data.add_span(llm_span);
                        }

                        self.traces_queue.lock().unwrap().push_back(trace_data);
                    }
                };
            }

            return Action::Continue;
        }

        let body = if self.streaming_response {
            let chunk_start = 0;
            let chunk_size = body_size;
            debug!(
                "on_http_response_body: streaming response reading, {}..{}",
                chunk_start, chunk_size
            );
            let streaming_chunk = match self.get_http_response_body(0, chunk_size) {
                Some(chunk) => chunk,
                None => {
                    warn!(
                        "response body empty, chunk_start: {}, chunk_size: {}",
                        chunk_start, chunk_size
                    );
                    return Action::Continue;
                }
            };

            if streaming_chunk.len() != chunk_size {
                warn!(
                    "chunk size mismatch: read: {} != requested: {}",
                    streaming_chunk.len(),
                    chunk_size
                );
            }
            streaming_chunk
        } else {
            if body_size == 0 {
                return Action::Continue;
            }
            debug!("non streaming response bytes read: 0:{}", body_size);
            match self.get_http_response_body(0, body_size) {
                Some(body) => body,
                None => {
                    warn!("non streaming response body empty");
                    return Action::Continue;
                }
            }
        };

        if log::log_enabled!(log::Level::Debug) {
            debug!(
                "response data (converted to utf8): {}",
                String::from_utf8_lossy(&body)
            );
        }

        if self.streaming_response {
            debug!("processing streaming response");
            match ProviderStreamResponseIter::try_from((&body[..], &self.get_provider_id())) {
                Ok(mut streaming_response) => {
                    // Process each streaming chunk
                    while let Some(chunk_result) = streaming_response.next() {
                        match chunk_result {
                            Ok(chunk) => {
                                // Compute TTFT on first chunk
                                if self.ttft_duration.is_none() {
                                    let current_time = get_current_time().unwrap();
                                    self.ttft_time = Some(current_time_ns());
                                    match current_time.duration_since(self.start_time) {
                                        Ok(duration) => {
                                            let duration_ms = duration.as_millis();
                                            info!(
                                                "on_http_response_body: time to first token: {}ms",
                                                duration_ms
                                            );
                                            self.ttft_duration = Some(duration);
                                            self.metrics
                                                .time_to_first_token
                                                .record(duration_ms as u64);
                                        }
                                        Err(e) => {
                                            warn!("SystemTime error: {:?}", e);
                                        }
                                    }
                                }

                                // For streaming responses, we handle token counting differently
                                // The ProviderStreamResponse trait provides content_delta, is_final, and role
                                // Token counting for streaming responses typically happens with final usage chunk
                                if chunk.is_final() {
                                    // For now, we'll implement basic token estimation
                                    // In a complete implementation, the final chunk would contain usage information
                                    debug!("Received final streaming chunk");
                                }

                                // For now, estimate tokens from content delta
                                if let Some(content) = chunk.content_delta() {
                                    // Rough estimation: ~4 characters per token
                                    let estimated_tokens = content.len() / 4;
                                    self.response_tokens += estimated_tokens.max(1);
                                }
                            }
                            Err(e) => {
                                warn!("Error processing streaming chunk: {}", e);
                                return Action::Continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse streaming response: {}", e);
                }
            }
        } else {
            debug!("non streaming response");
            let provider_id = self.get_provider_id();
            let response: ProviderResponseType =
                match ProviderResponseType::try_from((&body[..], provider_id)) {
                    Ok(response) => response,
                    Err(e) => {
                        warn!(
                            "could not parse response: {}, body str: {}",
                            e,
                            String::from_utf8_lossy(&body)
                        );
                        debug!(
                            "on_http_response_body: S[{}], response body: {}",
                            self.context_id,
                            String::from_utf8_lossy(&body)
                        );
                        self.send_server_error(
                            ServerError::LogicError(format!("Response parsing error: {}", e)),
                            Some(StatusCode::BAD_REQUEST),
                        );
                        return Action::Continue;
                    }
                };

            // Use provider interface to extract usage information
            if let Some((prompt_tokens, completion_tokens, total_tokens)) =
                response.extract_usage_counts()
            {
                debug!(
                    "Response usage: prompt={}, completion={}, total={}",
                    prompt_tokens, completion_tokens, total_tokens
                );
                self.response_tokens = completion_tokens;
            } else {
                warn!("No usage information found in response");
            }
        }

        debug!(
            "recv [S={}] total_tokens={} end_stream={}",
            self.context_id, self.response_tokens, end_of_stream
        );

        Action::Continue
    }
}

fn current_time_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

impl Context for StreamContext {}
