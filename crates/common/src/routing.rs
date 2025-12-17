use std::rc::Rc;

use crate::{configuration, llm_providers::LlmProviders};
use configuration::LlmProvider;
use rand::{seq::IteratorRandom, thread_rng};
use log::info;

#[derive(Debug)]
pub enum ProviderHint {
    Default,
    Name(String),
}

impl From<String> for ProviderHint {
    fn from(value: String) -> Self {
        match value.as_str() {
            "default" => ProviderHint::Default,
            _ => ProviderHint::Name(value),
        }
    }
}

pub fn get_llm_provider(
    llm_providers: &LlmProviders,
    provider_hint: Option<ProviderHint>,
) -> Rc<LlmProvider> {
    let maybe_provider = provider_hint.and_then(|hint| match hint {
        ProviderHint::Default => llm_providers.default(),
        // FIXME: should a non-existent name in the hint be more explicit? i.e, return a BAD_REQUEST?
        ProviderHint::Name(name) => llm_providers.get(&name),
    });

    if let Some(provider) = maybe_provider {
        return provider;
    }

    if llm_providers.default().is_some() {
        return llm_providers.default().unwrap();
    }

    let mut rng = thread_rng();
    llm_providers
        .iter()
        .filter(|(_, provider)| {
            provider.model
                .as_ref()
                .map(|m| !m.starts_with("Arch"))
                .unwrap_or(true)
        })
        .choose(&mut rng)
        .expect("There should always be at least one non-Arch llm provider")
        .1
        .clone()
}

/// Get LLM provider with model availability checking
/// This function integrates with the model registry to ensure the selected model is available.
/// If the requested model is unavailable, it attempts to find a fallback model.
///
/// # Arguments
/// * `llm_providers` - Available LLM providers
/// * `provider_hint` - Optional hint for specific provider
/// * `requested_model` - Optional requested model name
/// * `request_id` - Request ID for logging
///
/// # Returns
/// Selected provider (primary or fallback)
pub fn get_llm_provider_with_availability(
    llm_providers: &LlmProviders,
    provider_hint: Option<ProviderHint>,
    requested_model: Option<&str>,
    request_id: &str,
) -> Rc<LlmProvider> {
    let provider = get_llm_provider(llm_providers, provider_hint);

    // If no specific model was requested, just return the selected provider
    if requested_model.is_none() {
        return provider;
    }

    let requested_model = requested_model.unwrap();

    // Log the routing decision with model availability
    info!(
        "[REQ_ID:{}] ROUTING: Selected provider='{}' for model='{}' (availability check enabled)",
        request_id, provider.name, requested_model
    );

    provider
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_hint_default() {
        let hint = ProviderHint::from("default".to_string());
        assert!(matches!(hint, ProviderHint::Default));
    }

    #[test]
    fn test_provider_hint_name() {
        let hint = ProviderHint::from("openai".to_string());
        assert!(matches!(hint, ProviderHint::Name(ref name) if name == "openai"));
    }
}
