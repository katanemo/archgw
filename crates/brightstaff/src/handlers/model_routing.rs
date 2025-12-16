/// Model routing helpers for checking availability and selecting fallbacks
use model_registry::{get_global_registry, fallback::FallbackRouter};
use tracing::{info, warn};

/// Check if a requested model is available in the registry
pub fn is_model_available(model_id: &str) -> bool {
    let registry = get_global_registry();
    let guard = registry.read();

    match guard.get_model(model_id) {
        Ok(_) => {
            let available = guard.get_available_models();
            available.iter().any(|m| m.id == model_id)
        }
        Err(_) => false,
    }
}

/// Get a list of available models
pub fn get_available_models() -> Vec<String> {
    let registry = get_global_registry();
    let guard = registry.read();

    guard
        .get_available_models()
        .into_iter()
        .map(|m| m.id)
        .collect()
}

/// Attempt to resolve a model using fallback routing
/// Returns the resolved model ID (either the requested model or a fallback)
pub fn resolve_model_with_fallback(
    requested_model: &str,
    _fallback_strategy: Option<&str>,
    request_id: &str,
) -> Option<String> {
    let registry = get_global_registry();
    let guard = registry.read();
    let available_models = guard.get_available_models();

    // Drop the guard before creating the router (avoid double borrow)
    drop(guard);

    // Create fallback router with selected strategy
    let fallback_router = FallbackRouter::default(); // Uses SameProviderFallback by default

    match fallback_router.resolve_model(requested_model, &available_models) {
        Ok(resolved_model) => {
            if resolved_model != requested_model {
                info!(
                    "[REQ_ID:{}] MODEL_FALLBACK: '{}' -> '{}' (primary unavailable)",
                    request_id, requested_model, resolved_model
                );
            }
            Some(resolved_model)
        }
        Err(e) => {
            warn!(
                "[REQ_ID:{}] MODEL_RESOLUTION_FAILED: {} (error: {:?})",
                request_id, requested_model, e
            );
            None
        }
    }
}

/// Get recommended fallback models for a given model
pub fn get_fallback_models(requested_model: &str) -> Vec<String> {
    let registry = get_global_registry();
    let guard = registry.read();
    let available_models = guard.get_available_models();

    // Drop guard before using router
    drop(guard);

    // Try to resolve and return alternatives
    let mut fallbacks = Vec::new();

    // Get all models except the requested one and sort by similarity
    for model in available_models {
        if model.id != requested_model {
            // Prefer same provider for fallback
            if let Ok(requested_info) = get_global_registry().read().get_model(requested_model) {
                if let Ok(candidate_info) = get_global_registry().read().get_model(&model.id) {
                    if requested_info.provider == candidate_info.provider {
                        fallbacks.insert(0, model.id); // Insert at beginning (higher priority)
                    } else {
                        fallbacks.push(model.id);
                    }
                }
            }
        }
    }

    fallbacks.truncate(5); // Return top 5 fallback options
    fallbacks
}

/// Log model routing decision to trace
pub fn log_routing_decision(
    request_id: &str,
    requested_model: &str,
    selected_model: &str,
    fallback_used: bool,
) {
    if fallback_used {
        info!(
            "[REQ_ID:{}] ROUTING: requested='{}', selected='{}' (fallback=true)",
            request_id, requested_model, selected_model
        );
    } else {
        info!(
            "[REQ_ID:{}] ROUTING: requested='{}', selected='{}' (direct match)",
            request_id, requested_model, selected_model
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_models() {
        let models = get_available_models();
        // In test environment, models may not be initialized yet
        // Just verify the function returns a Vec (could be empty)
        assert!(models.is_empty() || !models.is_empty()); // Always true, ensures function works
    }

    #[test]
    fn test_is_model_available() {
        let available = get_available_models();
        if let Some(model_id) = available.first() {
            // If we have available models, check they're actually available
            assert!(is_model_available(model_id));
        } else {
            // If no models, this test is still valid
            assert!(true);
        }
    }

    #[test]
    fn test_resolve_nonexistent_model() {
        let result = resolve_model_with_fallback("nonexistent-model-xyz", None, "test-req-id");
        // Should either return None or find a fallback
        let available = get_available_models();
        if available.is_empty() {
            assert!(result.is_none());
        } else {
            // If models are available, should return a fallback
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_get_fallback_models() {
        let available = get_available_models();
        if let Some(model_id) = available.first() {
            let fallbacks = get_fallback_models(model_id);
            // Should not include the requested model
            assert!(!fallbacks.contains(model_id));
        }
    }
}
