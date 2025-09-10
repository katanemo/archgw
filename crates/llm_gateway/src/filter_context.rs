use crate::metrics::Metrics;
use crate::stream_context::StreamContext;
use common::configuration::Configuration;
use common::configuration::Overrides;
use common::consts::OTEL_COLLECTOR_HTTP;
use common::consts::OTEL_POST_PATH;
use common::http::CallArgs;
use common::http::Client;
use common::llm_providers::LlmProviders;
use common::ratelimit;
use common::stats::Gauge;
use common::tracing::TraceData;
use log::trace;
use log::warn;
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct CallContext {}

#[derive(Debug)]
pub struct FilterContext {
    metrics: Rc<Metrics>,
    // callouts stores token_id to request mapping that we use during #on_http_call_response to match the response to the request.
    callouts: RefCell<HashMap<u32, CallContext>>,
    llm_providers: Option<Rc<LlmProviders>>,
    traces_queue: Arc<Mutex<VecDeque<TraceData>>>,
    overrides: Rc<Option<Overrides>>,
}

impl FilterContext {
    pub fn new() -> FilterContext {
        FilterContext {
            callouts: RefCell::new(HashMap::new()),
            metrics: Rc::new(Metrics::new()),
            llm_providers: None,
            traces_queue: Arc::new(Mutex::new(VecDeque::new())),
            overrides: Rc::new(None),
        }
    }
}

impl Client for FilterContext {
    type CallContext = CallContext;

    fn callouts(&self) -> &RefCell<HashMap<u32, Self::CallContext>> {
        &self.callouts
    }

    fn active_http_calls(&self) -> &Gauge {
        &self.metrics.active_http_calls
    }
}

// RootContext allows the Rust code to reach into the Envoy Config
impl RootContext for FilterContext {
    fn on_configure(&mut self, _: usize) -> bool {
        let config_bytes = self
            .get_plugin_configuration()
            .expect("Arch config cannot be empty");

        let config: Configuration = match serde_yaml::from_slice(&config_bytes) {
            Ok(config) => config,
            Err(err) => panic!("Invalid arch config \"{:?}\"", err),
        };

        ratelimit::ratelimits(Some(config.ratelimits.unwrap_or_default()));
        self.overrides = Rc::new(config.overrides);

        match config.llm_providers.try_into() {
            Ok(llm_providers) => self.llm_providers = Some(Rc::new(llm_providers)),
            Err(err) => panic!("{err}"),
        }

        true
    }

    fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
        trace!(
            "||| create_http_context called with context_id: {:?} |||",
            context_id
        );

        Some(Box::new(StreamContext::new(
            Rc::clone(&self.metrics),
            Rc::clone(
                self.llm_providers
                    .as_ref()
                    .expect("LLM Providers must exist when Streams are being created"),
            ),
            Arc::clone(&self.traces_queue),
            Rc::clone(&self.overrides),
        )))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn on_vm_start(&mut self, _vm_configuration_size: usize) -> bool {
        self.set_tick_period(Duration::from_secs(1));
        true
    }

    fn on_tick(&mut self) {
        let _ = self.traces_queue.try_lock().map(|mut traces_queue| {
            while let Some(trace) = traces_queue.pop_front() {
                let trace_str = serde_json::to_string(&trace).unwrap();
                trace!("trace details: {}", trace_str);
                let call_args = CallArgs::new(
                    OTEL_COLLECTOR_HTTP,
                    OTEL_POST_PATH,
                    vec![
                        (":method", http::Method::POST.as_str()),
                        (":path", OTEL_POST_PATH),
                        (":authority", OTEL_COLLECTOR_HTTP),
                        ("content-type", "application/json"),
                    ],
                    Some(trace_str.as_bytes()),
                    vec![],
                    Duration::from_secs(60),
                );
                if let Err(error) = self.http_call(call_args, CallContext {}) {
                    warn!(
                        "failed to schedule http call to otel-collector: {:?}",
                        error
                    );
                }
            }
        });
    }
}

impl Context for FilterContext {
    fn on_http_call_response(
        &mut self,
        token_id: u32,
        _num_headers: usize,
        _body_size: usize,
        _num_trailers: usize,
    ) {
        trace!(
            "||| on_http_call_response called with token_id: {:?} |||",
            token_id
        );

        let _callout_data = self
            .callouts
            .borrow_mut()
            .remove(&token_id)
            .expect("invalid token_id");

        if let Some(status) = self.get_http_call_response_header(":status") {
            trace!("trace response status: {:?}", status);
        };
    }
}
