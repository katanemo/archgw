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

use crate::metrics::Metrics;
use common::configuration::{LlmProvider, LlmProviderType, Overrides};
use common::consts::{
    ARCH_PROVIDER_HINT_HEADER, ARCH_ROUTING_HEADER, HEALTHZ_PATH, RATELIMIT_SELECTOR_HEADER_KEY,
    REQUEST_ID_HEADER, TRACE_PARENT_HEADER,
};
use common::errors::ServerError;
use common::llm_providers::LlmProviders;
use common::ratelimit::Header;
use common::stats::{IncrementingMetric, RecordingMetric};
use common::tracing::{Event, Span, TraceData, Traceparent};
use common::{ratelimit, routing, tokenizer};
use hermesllm::clients::endpoints::SupportedAPIs;
use hermesllm::providers::response::{ProviderResponse, ProviderStreamResponseIter};
use hermesllm::{ProviderId, ProviderRequest, ProviderRequestType, ProviderResponseType};

pub struct StreamContext {
    context_id: u32,
    metrics: Rc<Metrics>,
    ratelimit_selector: Option<Header>,
    streaming_response: bool,
    response_tokens: usize,
    /// The API that is requested by the client (before compatibility mapping)
    client_api: Option<SupportedAPIs>,
    /// The API that should be used for the upstream provider (after compatibility mapping)
    resolved_api: Option<SupportedAPIs>,
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
            client_api: None,
            resolved_api: None,
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

    //This function assumes that the provider has been set.
    fn update_upstream_path(&mut self, request_path: &str) {
        let hermes_provider_id = self.llm_provider().to_provider_id();
        if let Some(api) = &self.client_api {
            let target_endpoint =
                api.target_endpoint_for_provider(&hermes_provider_id, request_path);
            if target_endpoint != request_path {
                self.set_http_request_header(":path", Some(&target_endpoint));
            }
        }
    }

    fn select_llm_provider(&mut self) {
        let provider_hint = self
            .get_http_request_header(ARCH_PROVIDER_HINT_HEADER)
            .map(|llm_name| llm_name.into());

        self.llm_provider = Some(routing::get_llm_provider(
            &self.llm_providers,
            provider_hint,
        ));

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

    // === Helper methods extracted from on_http_response_body (no behavior change) ===
    #[inline]
    fn record_ttft_if_needed(&mut self) {
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
                    self.metrics.time_to_first_token.record(duration_ms as u64);
                }
                Err(e) => {
                    warn!("SystemTime error: {:?}", e);
                }
            }
        }
    }
    fn handle_end_of_stream_metrics_and_traces(&mut self, current_time: SystemTime) {
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
                    llm_span
                        .add_attribute("model".to_string(), self.llm_provider().name.to_string());

                    if let Some(user_message) = &self.user_message {
                        llm_span.add_attribute("user_message".to_string(), user_message.clone());
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
    }

    fn read_raw_response_body(&mut self, body_size: usize) -> Result<Vec<u8>, Action> {
        if self.streaming_response {
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
                    return Err(Action::Continue);
                }
            };

            if streaming_chunk.len() != chunk_size {
                warn!(
                    "chunk size mismatch: read: {} != requested: {}",
                    streaming_chunk.len(),
                    chunk_size
                );
            }
            Ok(streaming_chunk)
        } else {
            if body_size == 0 {
                return Err(Action::Continue);
            }
            debug!("non streaming response bytes read: 0:{}", body_size);
            match self.get_http_response_body(0, body_size) {
                Some(body) => Ok(body),
                None => {
                    warn!("non streaming response body empty");
                    Err(Action::Continue)
                }
            }
        }
    }

    fn debug_log_body(&self, body: &[u8]) {
        if log::log_enabled!(log::Level::Debug) {
            debug!(
                "response data (converted to utf8): {}",
                String::from_utf8_lossy(body)
            );
        }
    }

    fn handle_streaming_response(
        &mut self,
        body: &[u8],
        provider_id: ProviderId,
    ) -> Result<Vec<u8>, Action> {
        debug!("processing streaming response");
        match self.client_api.as_ref() {
            Some(client_api) => {
                match ProviderStreamResponseIter::try_from((body, client_api, &provider_id)) {
                    Ok(mut streaming_response) => {
                        while let Some(chunk_result) = streaming_response.next() {
                            match chunk_result {
                                Ok(chunk) => {
                                    self.record_ttft_if_needed();

                                    if chunk.is_final() {
                                        debug!("Received final streaming chunk");
                                    }
                                    if let Some(content) = chunk.content_delta() {
                                        let estimated_tokens = content.len() / 4;
                                        self.response_tokens += estimated_tokens.max(1);
                                    }
                                }
                                Err(e) => {
                                    warn!("Error processing streaming chunk: {}", e);
                                    return Err(Action::Continue);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse streaming response: {}", e);
                    }
                }
            }
            None => {
                warn!("Missing client_api for non-streaming response");
                return Err(Action::Continue);
            }
        };
        // NOTE:
        // We currently pass-through the original SSE bytes for streaming responses.
        // Non-streaming responses are parsed into ProviderResponseType and re-serialized to
        // normalize the payload to the client API. Doing the same for streaming would require
        // a streaming serializer that emits normalized SSE events for the target client API.
        // That doesn't exist yet in hermesllm; implementing it is a follow-up.
        // TODO(salmanap): Add a normalized SSE serializer in hermesllm and use it here so both
        // streaming and non-streaming paths perform the same compatibility mapping.
        // Until then, we keep behavior unchanged and forward upstream SSE as-is.
        // For consistency of the method contract, still return Vec<u8>.
        Ok(body.to_vec())
    }

    fn handle_non_streaming_response(
        &mut self,
        body: &[u8],
        provider_id: ProviderId,
    ) -> Result<Vec<u8>, Action> {
        let response: ProviderResponseType = match self.client_api.as_ref() {
            Some(client_api) => {
                match ProviderResponseType::try_from((body, client_api, &provider_id)) {
                    Ok(response) => response,
                    Err(e) => {
                        warn!(
                            "could not parse response: {}, body str: {}",
                            e,
                            String::from_utf8_lossy(body)
                        );
                        debug!(
                            "on_http_response_body: S[{}], response body: {}",
                            self.context_id,
                            String::from_utf8_lossy(body)
                        );
                        self.send_server_error(
                            ServerError::LogicError(format!("Response parsing error: {}", e)),
                            Some(StatusCode::BAD_REQUEST),
                        );
                        return Err(Action::Continue);
                    }
                }
            }
            None => {
                warn!("Missing client_api for non-streaming response");
                return Err(Action::Continue);
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
        // Serialize the normalized response back to JSON bytes
        match serde_json::to_vec(&response) {
            Ok(bytes) => {
                debug!(
                        "non streaming response data after serialization. length: {},  converted to utf8: {}",
                        bytes.len(),
                        String::from_utf8_lossy(&bytes)
                    );
                Ok(bytes)
            }
            Err(e) => {
                warn!("Failed to serialize normalized response: {}", e);
                self.send_server_error(
                    ServerError::LogicError(format!("Response serialization error: {}", e)),
                    Some(StatusCode::INTERNAL_SERVER_ERROR),
                );
                Err(Action::Continue)
            }
        }
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
                ..Default::default() //TODO: THiS IS BROKEN. WHY ARE WE ASSUMING OPENAI FOR UPSTREAM?
            }));
        } else {
            //TODO: Fix this brittle code path. We need to return values and have compile time
            self.select_llm_provider();

            // Check if this is a supported API endpoint
            if SupportedAPIs::from_endpoint(&request_path).is_none() {
                self.send_http_response(404, vec![], Some(b"Unsupported endpoint"));
                return Action::Continue;
            }

            // Get the SupportedApi for routing decisions
            let supported_api: Option<SupportedAPIs> = SupportedAPIs::from_endpoint(&request_path);
            self.client_api = supported_api;

            // Debug: log provider, client API, resolved API, and request path
            if let (Some(api), Some(provider)) =
                (self.client_api.as_ref(), self.llm_provider.as_ref())
            {
                let provider_id = provider.to_provider_id();
                self.resolved_api = Some(provider_id.compatible_api_for_client(api));
            } else {
                self.resolved_api = None;
            }

            //We need to update the upstream path if there is a variation for a provider like Gemini/Groq, etc.
            self.update_upstream_path(&request_path);

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

        //We need to deserialize the request body based on the resolved API
        let mut deserialized_client_request: ProviderRequestType = match self.client_api.as_ref() {
            Some(the_client_api) => {
                match ProviderRequestType::try_from((&body_bytes[..], the_client_api)) {
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
                }
            }
            None => {
                self.send_server_error(
                    ServerError::LogicError("No resolved API for provider".to_string()),
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
        let model_requested = deserialized_client_request.model().to_string();

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
        deserialized_client_request.set_model(resolved_model.clone());

        // Extract user message for tracing
        self.user_message = deserialized_client_request.get_recent_user_message();

        info!(
            "on_http_request_body: provider: {}, model requested (in body): {}, model selected: {}",
            self.llm_provider().name,
            model_requested,
            model_name.unwrap_or(&"None".to_string()),
        );

        // Use provider interface for streaming detection and setup
        self.streaming_response = deserialized_client_request.is_streaming();

        // Use provider interface for text extraction (after potential mutation)
        let input_tokens_str = deserialized_client_request.extract_messages_text();
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
        let serialized_body_bytes_upstream = match self.resolved_api.as_ref() {
            Some(upstream) => {
                match ProviderRequestType::try_from((&deserialized_client_request, upstream)) {
                    Ok(request) => match request.to_bytes() {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            warn!("Failed to serialize request body: {}", e);
                            self.send_server_error(
                                ServerError::LogicError(format!(
                                    "Request serialization error: {}",
                                    e
                                )),
                                Some(StatusCode::BAD_REQUEST),
                            );
                            return Action::Pause;
                        }
                    },
                    Err(e) => {
                        warn!("Failed to create provider request: {}", e);
                        self.send_server_error(
                            ServerError::LogicError(format!("Provider request error: {}", e)),
                            Some(StatusCode::BAD_REQUEST),
                        );
                        return Action::Pause;
                    }
                }
            }
            None => {
                warn!("No upstream API resolved");
                self.send_server_error(
                    ServerError::LogicError("No upstream API resolved".into()),
                    Some(StatusCode::BAD_REQUEST),
                );
                return Action::Pause;
            }
        };

        self.set_http_request_body(0, body_size, &serialized_body_bytes_upstream);
        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, end_of_stream: bool) -> Action {
        debug!(
            "on_http_response_headers [S={}] end_stream={}",
            self.context_id, end_of_stream
        );

        self.remove_http_response_header("content-length");
        // If upstream may compress, drop encoding so our new bytes are sent as-is.
        self.remove_http_response_header("content-encoding");

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

        match self.client_api {
            Some(SupportedAPIs::OpenAIChatCompletions(_)) => {}
            Some(SupportedAPIs::AnthropicMessagesAPI(_)) => {}
            _ => {
                info!("on_http_response_body: non-chatcompletion request");
                return Action::Continue;
            }
        }

        let current_time = get_current_time().unwrap();
        if end_of_stream && body_size == 0 {
            self.handle_end_of_stream_metrics_and_traces(current_time);
            return Action::Continue;
        }

        let body = match self.read_raw_response_body(body_size) {
            Ok(bytes) => bytes,
            Err(action) => return action,
        };

        self.debug_log_body(&body);

        let provider_id = self.get_provider_id();
        if self.streaming_response {
            match self.handle_streaming_response(&body, provider_id) {
                Ok(serialized_body) => {
                    self.set_http_response_body(0, body_size, &serialized_body);
                }
                Err(action) => return action,
            }
        } else {
            match self.handle_non_streaming_response(&body, provider_id) {
                Ok(serialized_body) => {
                    self.set_http_response_body(0, body_size, &serialized_body);
                }
                Err(action) => return action,
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
