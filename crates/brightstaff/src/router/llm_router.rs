use std::sync::Arc;

use common::{
    api::open_ai::{ChatCompletionsResponse, Message},
    configuration::LlmProvider,
    consts::ARCH_PROVIDER_HINT_HEADER,
    utils::shorten_string,
};
use hyper::header;
use thiserror::Error;
use tracing::{info, warn};

use super::router_model::RouterModel;

pub struct RouterService {
    router_url: String,
    client: reqwest::Client,
    router_model: Arc<dyn RouterModel>,
    routing_model_name: String,
    llm_usage_defined: bool,
}

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("Failed to send request: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Failed to parse JSON: {0}, JSON: {1}")]
    JsonError(serde_json::Error, String),

    #[error("Router model error: {0}")]
    RouterModelError(#[from] super::router_model::RoutingModelError),
}

pub type Result<T> = std::result::Result<T, RoutingError>;

impl RouterService {
    pub fn new(
        providers: Vec<LlmProvider>,
        router_url: String,
        routing_model_name: String,
    ) -> Self {
        let providers_with_usage = providers
            .iter()
            .filter(|provider| provider.usage.is_some())
            .cloned()
            .collect::<Vec<LlmProvider>>();

        // convert the llm_providers to yaml string but only include name and usage
        let llm_providers_with_usage_yaml = providers_with_usage
            .iter()
            .map(|provider| {
                format!(
                    "- name: {}\n  description: {}",
                    provider.name,
                    provider.usage.as_ref().unwrap_or(&"".to_string())
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        info!(
            "llm_providers from config with usage: {}...",
            shorten_string(&llm_providers_with_usage_yaml.replace("\n", "\\n"))
        );

        let router_model = Arc::new(super::router_model_v1::RouterModelV1::new(
            llm_providers_with_usage_yaml.clone(),
            routing_model_name.clone(),
        ));

        RouterService {
            router_url,
            client: reqwest::Client::new(),
            router_model,
            routing_model_name,
            llm_usage_defined: !providers_with_usage.is_empty(),
        }
    }

    pub async fn determine_route(
        &self,
        messages: &[Message],
        trace_parent: Option<String>,
    ) -> Result<Option<String>> {

        if !self.llm_usage_defined {
            return Ok(None);
        }

        let router_request = self.router_model.generate_request(messages);

        info!(
            "router_request: {}",
            shorten_string(&serde_json::to_string(&router_request).unwrap()),
        );

        let mut llm_route_request_headers = header::HeaderMap::new();
        llm_route_request_headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        llm_route_request_headers.insert(
            header::HeaderName::from_static(ARCH_PROVIDER_HINT_HEADER),
            header::HeaderValue::from_str(&self.routing_model_name).unwrap(),
        );

        if let Some(trace_parent) = trace_parent {
            llm_route_request_headers.insert(
                header::HeaderName::from_static("traceparent"),
                header::HeaderValue::from_str(&trace_parent).unwrap(),
            );
        }

        let res = self
            .client
            .post(&self.router_url)
            .headers(llm_route_request_headers)
            .body(serde_json::to_string(&router_request).unwrap())
            .send()
            .await?;

        let body = res.text().await?;

        let chat_completion_response: ChatCompletionsResponse = match serde_json::from_str(&body) {
            Ok(response) => response,
            Err(err) => {
                warn!(
                    "Failed to parse JSON: {}. Body: {}",
                    err,
                    &serde_json::to_string(&body).unwrap()
                );
                return Err(RoutingError::JsonError(
                    err,
                    format!("Failed to parse JSON: {}", body),
                ));
            }
        };

        let selected_llm = self.router_model.parse_response(
            chat_completion_response.choices[0]
                .message
                .content
                .as_ref()
                .unwrap(),
        )?;

        Ok(selected_llm)
    }
}
