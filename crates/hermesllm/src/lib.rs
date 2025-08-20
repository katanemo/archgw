//! hermesllm: A library for translating LLM API requests and responses
//! between Mistral, Grok, Gemini, and OpenAI-compliant formats.

pub mod providers;
pub mod apis;
pub mod clients;

// Re-export important types and traits
pub use providers::request::{ProviderRequestType, ProviderRequest, ProviderRequestError};
pub use providers::response::{ProviderResponseType, ProviderResponse, ProviderStreamResponse, ProviderStreamResponseIter, ProviderResponseError, TokenUsage};
pub use providers::id::ProviderId;
pub use providers::adapters::{has_compatible_api, supported_apis};

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
    fn test_provider_api_compatibility() {
        assert!(has_compatible_api(&ProviderId::OpenAI, "/v1/chat/completions"));
        assert!(!has_compatible_api(&ProviderId::OpenAI, "/v1/embeddings"));
    }

    #[test]
    fn test_provider_supported_apis() {
        let apis = supported_apis(&ProviderId::OpenAI);
        assert!(apis.contains(&"/v1/chat/completions"));

        // Test that provider supports the expected API endpoints
        assert!(has_compatible_api(&ProviderId::OpenAI, "/v1/chat/completions"));
    }

    #[test]
    fn test_provider_request_parsing() {
        // Test with a sample JSON request
        let json_request = r#"{
            "model": "gpt-4",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant"
                },
                {
                    "role": "user",
                    "content": "Hello!"
                }
            ]
        }"#;

        let result: Result<ProviderRequestType, std::io::Error> = ProviderRequestType::try_from(json_request.as_bytes());
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.model(), "gpt-4");
        assert_eq!(request.get_recent_user_message(), Some("Hello!".to_string()));
    }

    #[test]
    fn test_provider_streaming_response() {
        // Test streaming response parsing with sample SSE data
        let sse_data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

data: [DONE]
"#;

        let result = ProviderStreamResponseIter::try_from((sse_data.as_bytes(), &ProviderId::OpenAI));
        assert!(result.is_ok());

        let mut streaming_response = result.unwrap();

        // Test that we can iterate over chunks - it's just an iterator now!
        let first_chunk = streaming_response.next();
        assert!(first_chunk.is_some());

        let chunk_result = first_chunk.unwrap();
        assert!(chunk_result.is_ok());

        let chunk = chunk_result.unwrap();
        assert_eq!(chunk.content_delta(), Some("Hello"));
        assert!(!chunk.is_final());

        // Test that stream ends properly
        let final_chunk = streaming_response.next();
        assert!(final_chunk.is_none());
    }
}
