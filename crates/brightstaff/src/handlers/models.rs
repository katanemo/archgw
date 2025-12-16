use bytes::Bytes;
use common::configuration::LlmProvider;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{Response, StatusCode};
use serde_json;
use std::sync::Arc;
use model_registry::{get_global_registry, definitions};

pub async fn list_models(
    _llm_providers: Arc<tokio::sync::RwLock<Vec<LlmProvider>>>,
) -> Response<BoxBody<Bytes, hyper::Error>> {
    let registry = get_global_registry();
    let registry_guard = registry.read();

    // Get all available models from registry
    let models = registry_guard.get_available_models();

    // Convert to OpenAI-compatible format
    let model_list: Vec<serde_json::Value> = models
        .into_iter()
        .map(|model| {
            serde_json::json!({
                "id": model.id,
                "object": model.object,
                "created": model.created.timestamp(),
                "owned_by": model.owned_by,
                "permission": [],
                "root": model.id,
                "parent": serde_json::Value::Null,
            })
        })
        .collect();

    let response_json = serde_json::json!({
        "object": "list",
        "data": model_list,
    });

    match serde_json::to_string(&response_json) {
        Ok(json) => {
            let body = Full::new(Bytes::from(json))
                .map_err(|never| match never {})
                .boxed();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(body)
                .unwrap()
        }
        Err(_) => {
            let body = Full::new(Bytes::from_static(
                b"{\"error\":\"Failed to serialize models\"}",
            ))
            .map_err(|never| match never {})
            .boxed();
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(body)
                .unwrap()
        }
    }
}

pub async fn get_model(
    model_id: &str,
) -> Response<BoxBody<Bytes, hyper::Error>> {
    let registry = get_global_registry();
    let registry_guard = registry.read();

    match registry_guard.get_model(model_id) {
        Ok(model) => {
            let response_json = serde_json::json!({
                "id": model.id,
                "object": model.object,
                "created": model.created.timestamp(),
                "owned_by": model.owned_by,
                "permission": [],
                "root": model.id,
                "parent": serde_json::Value::Null,
                "display_name": model.display_name,
                "description": model.description,
                "context_window": model.context_window,
                "max_completion_tokens": model.max_completion_tokens,
                "supports_vision": model.supports_vision,
                "supports_function_calling": model.supports_function_calling,
                "supports_streaming": model.supports_streaming,
                "pricing": model.pricing,
                "thinking": model.thinking,
                "status": model.status,
            });

            match serde_json::to_string(&response_json) {
                Ok(json) => {
                    let body = Full::new(Bytes::from(json))
                        .map_err(|never| match never {})
                        .boxed();
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .body(body)
                        .unwrap()
                }
                Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize model"),
            }
        }
        Err(_) => error_response(StatusCode::NOT_FOUND, "Model not found"),
    }
}

pub async fn list_available_models() -> Response<BoxBody<Bytes, hyper::Error>> {
    let registry = get_global_registry();
    let registry_guard = registry.read();

    // Get all available models from registry
    let models = registry_guard.get_available_models();

    // Convert to OpenAI-compatible format
    let model_list: Vec<serde_json::Value> = models
        .into_iter()
        .map(|model| {
            serde_json::json!({
                "id": model.id,
                "object": model.object,
                "created": model.created.timestamp(),
                "owned_by": model.owned_by,
                "permission": [],
                "root": model.id,
                "parent": serde_json::Value::Null,
                "status": model.status,
            })
        })
        .collect();

    let response_json = serde_json::json!({
        "object": "list",
        "data": model_list,
    });

    match serde_json::to_string(&response_json) {
        Ok(json) => {
            let body = Full::new(Bytes::from(json))
                .map_err(|never| match never {})
                .boxed();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(body)
                .unwrap()
        }
        Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize models"),
    }
}

pub async fn initialize_default_models() {
    let registry = get_global_registry();
    let registry_guard = registry.write();

    // Register all default models from definitions
    let default_models = definitions::get_all_default_models();
    if let Err(e) = registry_guard.register_models(default_models) {
        eprintln!("Failed to register default models: {:?}", e);
    }
}

fn error_response(status: StatusCode, message: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let body = Full::new(Bytes::from(format!(
        "{{\"error\":\"{}\"}}",
        message
    )))
    .map_err(|never| match never {})
    .boxed();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(body)
        .unwrap()
}
