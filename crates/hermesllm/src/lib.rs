//! hermesllm: A library for translating LLM API requests and responses
//! between Mistral, Grok, Gemini, and OpenAI-compliant formats.

pub mod apis;
pub mod clients;
pub mod providers;
// Re-export important types and traits
pub use providers::id::ProviderId;
pub use providers::request::{ProviderRequest, ProviderRequestError, ProviderRequestType};
pub use providers::response::{
    ProviderResponse, ProviderResponseError, ProviderResponseType, ProviderStreamResponse,
    ProviderStreamResponseType, SseEvent, SseStreamIter, TokenUsage,
};

//TODO: Refactor such that commons doesn't depend on Hermes. For now this will clean up strings
pub const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";
pub const MESSAGES_PATH: &str = "/v1/messages";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_conversion() {
        assert_eq!(ProviderId::from("openai"), ProviderId::OpenAI);
        assert_eq!(ProviderId::from("mistral"), ProviderId::Mistral);
        assert_eq!(ProviderId::from("groq"), ProviderId::Groq);
        assert_eq!(ProviderId::from("arch"), ProviderId::Arch);
    }

    #[test]
    fn test_provider_streaming_response() {
        // Test streaming response parsing with sample SSE data
        let sse_data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

    data: [DONE]
    "#;

        use crate::clients::endpoints::SupportedAPIs;
        let client_api =
            SupportedAPIs::OpenAIChatCompletions(crate::apis::OpenAIApi::ChatCompletions);
        let upstream_api =
            SupportedAPIs::OpenAIChatCompletions(crate::apis::OpenAIApi::ChatCompletions);

        // Test the new simplified architecture - create SseStreamIter directly
        let sse_iter = SseStreamIter::try_from(sse_data.as_bytes());
        assert!(sse_iter.is_ok());

        let mut streaming_iter = sse_iter.unwrap();

        // Test that we can iterate over SseEvents
        let first_event = streaming_iter.next();
        assert!(first_event.is_some());

        let sse_event = first_event.unwrap();

        // Test SseEvent properties
        assert!(!sse_event.is_done());
        assert!(sse_event.data.as_ref().unwrap().contains("Hello"));

        // Test that we can parse the event into a provider stream response
        let transformed_event = SseEvent::try_from((sse_event, &client_api, &upstream_api));
        if let Err(e) = &transformed_event {
            println!("Transform error: {:?}", e);
        }
        assert!(transformed_event.is_ok());

        let transformed_event = transformed_event.unwrap();
        let provider_response = transformed_event.provider_response();
        assert!(provider_response.is_ok());

        let stream_response = provider_response.unwrap();
        assert_eq!(stream_response.content_delta(), Some("Hello"));
        assert!(!stream_response.is_final());

        // Test that stream ends properly with [DONE] (SseStreamIter should stop before [DONE])
        let final_event = streaming_iter.next();
        assert!(final_event.is_none()); // Should be None because iterator stops at [DONE]
    }
}
