use brightstaff::handlers::chat_completions::chat_completions;
use brightstaff::handlers::models::list_models;
use brightstaff::router::llm_router::RouterService;
use brightstaff::utils::tracing::init_tracer;
use bytes::Bytes;
use common::configuration::Configuration;
use http_body_util::{combinators::BoxBody, BodyExt, Empty};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use opentelemetry::trace::FutureExt;
use opentelemetry::{global, Context};
use opentelemetry_http::HeaderExtractor;
use std::sync::Arc;
use std::{env, fs};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub mod router;

const BIND_ADDRESS: &str = "0.0.0.0:9091";
const DEFAULT_ROUTING_LLM_PROVIDER: &str = "arch-router";
const DEFAULT_ROUTING_MODEL_NAME: &str = "Arch-Router";

// Utility function to extract the context from the incoming request headers
fn extract_context_from_request(req: &Request<Incoming>) -> Context {
    global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    })
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _tracer_provider = init_tracer();
    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| BIND_ADDRESS.to_string());

    info!(
        "current working directory: {}",
        env::current_dir().unwrap().display()
    );
    // loading arch_config.yaml file
    let arch_config_path = env::var("ARCH_CONFIG_PATH_RENDERED")
        .unwrap_or_else(|_| "./arch_config_rendered.yaml".to_string());
    info!("Loading arch_config.yaml from {}", arch_config_path);

    let config_contents =
        fs::read_to_string(&arch_config_path).expect("Failed to read arch_config.yaml");

    let config: Configuration =
        serde_yaml::from_str(&config_contents).expect("Failed to parse arch_config.yaml");

    let arch_config = Arc::new(config);

    let llm_providers = Arc::new(RwLock::new(arch_config.llm_providers.clone()));

    debug!(
        "arch_config: {:?}",
        &serde_json::to_string(arch_config.as_ref()).unwrap()
    );

    let llm_provider_endpoint = env::var("LLM_PROVIDER_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:12001/v1/chat/completions".to_string());

    info!("llm provider endpoint: {}", llm_provider_endpoint);
    info!("listening on http://{}", bind_address);
    let listener = TcpListener::bind(bind_address).await?;

    let routing_model_name: String = arch_config
        .routing
        .as_ref()
        .and_then(|r| r.model.clone())
        .unwrap_or_else(|| DEFAULT_ROUTING_MODEL_NAME.to_string());

    let routing_llm_provider = arch_config
        .routing
        .as_ref()
        .and_then(|r| r.llm_provider.clone())
        .unwrap_or_else(|| DEFAULT_ROUTING_LLM_PROVIDER.to_string());

    let router_service: Arc<RouterService> = Arc::new(RouterService::new(
        arch_config.llm_providers.clone(),
        llm_provider_endpoint.clone(),
        routing_model_name,
        routing_llm_provider,
    ));

    loop {
        let (stream, _) = listener.accept().await?;
        let peer_addr = stream.peer_addr()?;
        let io = TokioIo::new(stream);

        let router_service: Arc<RouterService> = Arc::clone(&router_service);
        let llm_provider_endpoint = llm_provider_endpoint.clone();

        let llm_providers = llm_providers.clone();
        let service = service_fn(move |req| {
            let router_service = Arc::clone(&router_service);
            let parent_cx = extract_context_from_request(&req);
            let llm_provider_endpoint = llm_provider_endpoint.clone();
            let llm_providers = llm_providers.clone();

            async move {
                match (req.method(), req.uri().path()) {
                    (&Method::POST, "/v1/chat/completions") => {
                        chat_completions(req, router_service, llm_provider_endpoint)
                            .with_context(parent_cx)
                            .await
                    }
                    (&Method::GET, "/v1/models") => Ok(list_models(llm_providers).await),
                    (&Method::OPTIONS, "/v1/models") => {
                        let mut response = Response::new(empty());
                        *response.status_mut() = StatusCode::NO_CONTENT;
                        response
                            .headers_mut()
                            .insert("Allow", "GET, OPTIONS".parse().unwrap());
                        response
                            .headers_mut()
                            .insert("Access-Control-Allow-Origin", "*".parse().unwrap());
                        response.headers_mut().insert(
                            "Access-Control-Allow-Headers",
                            "Authorization, Content-Type".parse().unwrap(),
                        );
                        response.headers_mut().insert(
                            "Access-Control-Allow-Methods",
                            "GET, POST, OPTIONS".parse().unwrap(),
                        );
                        response
                            .headers_mut()
                            .insert("Content-Type", "application/json".parse().unwrap());

                        Ok(response)
                    }
                    _ => {
                        let mut not_found = Response::new(empty());
                        *not_found.status_mut() = StatusCode::NOT_FOUND;
                        Ok(not_found)
                    }
                }
            }
        });

        tokio::task::spawn(async move {
            debug!("Accepted connection from {:?}", peer_addr);
            if let Err(err) = http1::Builder::new()
                // .serve_connection(io, service_fn(chat_completion))
                .serve_connection(io, service)
                .await
            {
                warn!("Error serving connection: {:?}", err);
            }
        });
    }
}
