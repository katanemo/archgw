use brightstaff::handlers::chat_completions::chat_completions;
use brightstaff::router::llm_router::RouterService;
use bytes::Bytes;
use common::configuration::Configuration;
use common::utils::shorten_string;
use http_body_util::{combinators::BoxBody, BodyExt, Empty};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use opentelemetry::global::BoxedTracer;
use opentelemetry::trace::FutureExt;
use opentelemetry::{
    global,
    trace::{SpanKind, Tracer},
    Context,
};
use opentelemetry_http::HeaderExtractor;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use opentelemetry_stdout::SpanExporter;
use std::sync::{Arc, OnceLock};
use std::{env, fs};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub mod router;

const BIND_ADDRESS: &str = "0.0.0.0:9091";

fn get_tracer() -> &'static BoxedTracer {
    static TRACER: OnceLock<BoxedTracer> = OnceLock::new();
    TRACER.get_or_init(|| global::tracer("archgw/router"))
}

// Utility function to extract the context from the incoming request headers
fn extract_context_from_request(req: &Request<Incoming>) -> Context {
    global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    })
}

fn init_tracer() -> SdkTracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());
    // Install stdout exporter pipeline to be able to retrieve the collected spans.
    // For the demonstration, use `Sampler::AlwaysOn` sampler to sample all traces.
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(SpanExporter::default())
        .build();

    global::set_tracer_provider(provider.clone());
    provider
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _tracer_provider = init_tracer();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| BIND_ADDRESS.to_string());

    //loading arch_config.yaml file
    let arch_config_path =
        env::var("ARCH_CONFIG_PATH").unwrap_or_else(|_| "./arch_config.yaml".to_string());
    info!("Loading arch_config.yaml from {}", arch_config_path);

    let config_contents =
        fs::read_to_string(&arch_config_path).expect("Failed to read arch_config.yaml");

    let config: Configuration =
        serde_yaml::from_str(&config_contents).expect("Failed to parse arch_config.yaml");

    let arch_config = Arc::new(config);

    info!(
        "arch_config: {:?}",
        shorten_string(&serde_json::to_string(arch_config.as_ref()).unwrap())
    );

    let llm_provider_endpoint = env::var("LLM_PROVIDER_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:12001/v1/chat/completions".to_string());

    info!("llm provider endpoint: {}", llm_provider_endpoint);
    info!("Listening on http://{}", bind_address);
    let listener = TcpListener::bind(bind_address).await?;


    // if routing is null then return gpt-4o as model name
    let model = arch_config.routing.as_ref().map_or_else(
        || "gpt-4o".to_string(),
        |routing| routing.model.clone(),
    );

    let router_service: Arc<RouterService> = Arc::new(RouterService::new(
        arch_config.llm_providers.clone(),
        llm_provider_endpoint.clone(),
        model,
    ));

    loop {
        let (stream, _) = listener.accept().await?;
        let peer_addr = stream.peer_addr()?;
        let io = TokioIo::new(stream);

        let router_service = Arc::clone(&router_service);
        let llm_provider_endpoint = llm_provider_endpoint.clone();

        let service = service_fn(move |req| {
            let router_service = Arc::clone(&router_service);
            let parent_cx = extract_context_from_request(&req);
            info!("parent_cx: {:?}", parent_cx);
            let tracer = get_tracer();
            let _span = tracer
                .span_builder("request")
                .with_kind(SpanKind::Server)
                .start_with_context(tracer, &parent_cx);
            let llm_provider_endpoint = llm_provider_endpoint.clone();

            async move {
                match (req.method(), req.uri().path()) {
                    (&Method::POST, "/v1/chat/completions") => {
                        chat_completions(req, router_service, llm_provider_endpoint)
                            .with_context(parent_cx)
                            .await
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
            info!("Accepted connection from {:?}", peer_addr);
            if let Err(err) = http1::Builder::new()
                // .serve_connection(io, service_fn(chat_completion))
                .serve_connection(io, service)
                .await
            {
                info!("Error serving connection: {:?}", err);
            }
        });
    }
}
