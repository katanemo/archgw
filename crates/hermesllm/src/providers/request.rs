use crate::apis::openai::ChatCompletionsRequest;
use crate::apis::anthropic::MessagesRequest;
use crate::clients::endpoints::SupportedApi;
use std::error::Error;
use std::fmt;
pub enum ProviderRequestType {
    ChatCompletionsRequest(ChatCompletionsRequest),
    MessagesRequest(MessagesRequest),
    //add more request types here
}

impl TryFrom<&[u8]> for ProviderRequestType {
    type Error = std::io::Error;

    // if passing bytes without provider id we assume the request is in OpenAI format
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let chat_completion_request: ChatCompletionsRequest = ChatCompletionsRequest::try_from(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(ProviderRequestType::ChatCompletionsRequest(chat_completion_request))
    }
}

/// Parse request based on endpoint and provider information
impl TryFrom<(&[u8], &SupportedApi)> for ProviderRequestType {
    type Error = std::io::Error;

    fn try_from((bytes, endpoint): (&[u8], &SupportedApi)) -> Result<Self, Self::Error> {
        // Use SupportedApi to determine the appropriate request type
        match endpoint {
            SupportedApi::OpenAI(_) => {
                let chat_completion_request: ChatCompletionsRequest = ChatCompletionsRequest::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderRequestType::ChatCompletionsRequest(chat_completion_request))
                }
                SupportedApi::Anthropic(_) => {
                    let messages_request: MessagesRequest = MessagesRequest::try_from(bytes)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    Ok(ProviderRequestType::MessagesRequest(messages_request))
                }
        }
    }
}

pub trait ProviderRequest: Send + Sync {
    /// Extract the model name from the request
    fn model(&self) -> &str;

    /// Set the model name for the request
    fn set_model(&mut self, model: String);

    /// Check if this is a streaming request
    fn is_streaming(&self) -> bool;

    /// Extract text content from messages for token counting
    fn extract_messages_text(&self) -> String;

    /// Extract the user message for tracing/logging purposes
    fn get_recent_user_message(&self) -> Option<String>;

    /// Convert the request to bytes for transmission
    fn to_bytes(&self) -> Result<Vec<u8>, ProviderRequestError>;
}

impl ProviderRequest for ProviderRequestType {
    fn model(&self) -> &str {
        match self {
            Self::ChatCompletionsRequest(r) => r.model(),
            Self::MessagesRequest(r) => r.model(),
        }
    }

    fn set_model(&mut self, model: String) {
        match self {
            Self::ChatCompletionsRequest(r) => r.set_model(model),
            Self::MessagesRequest(r) => r.set_model(model),
        }
    }

    fn is_streaming(&self) -> bool {
        match self {
            Self::ChatCompletionsRequest(r) => r.is_streaming(),
            Self::MessagesRequest(r) => r.is_streaming(),
        }
    }

    fn extract_messages_text(&self) -> String {
        match self {
            Self::ChatCompletionsRequest(r) => r.extract_messages_text(),
            Self::MessagesRequest(r) => r.extract_messages_text(),
        }
    }

    fn get_recent_user_message(&self) -> Option<String> {
        match self {
            Self::ChatCompletionsRequest(r) => r.get_recent_user_message(),
            Self::MessagesRequest(r) => r.get_recent_user_message(),
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>, ProviderRequestError> {
        match self {
            Self::ChatCompletionsRequest(r) => r.to_bytes(),
            Self::MessagesRequest(r) => r.to_bytes(),
        }
    }
}


/// Error types for provider operations
#[derive(Debug)]
pub struct ProviderRequestError {
    pub message: String,
    pub source: Option<Box<dyn Error + Send + Sync>>,
}

impl fmt::Display for ProviderRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Provider request error: {}", self.message)
    }
}

impl Error for ProviderRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}


// ...existing code...

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::endpoints::SupportedApi;
    use crate::apis::anthropic::AnthropicApi::Messages;
    use crate::apis::openai::OpenAIApi::ChatCompletions;
    use crate::apis::anthropic::MessagesRequest as AnthropicMessagesRequest;
    use crate::apis::openai::{ChatCompletionsRequest};
    use crate::clients::transformer::ExtractText;
    use serde_json::json;

    #[test]
    fn test_openai_request_from_bytes() {
        let req = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant"},
                {"role": "user", "content": "Hello!"}
            ]
        });
        let bytes = serde_json::to_vec(&req).unwrap();
        let result = ProviderRequestType::try_from(bytes.as_slice());
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ChatCompletionsRequest(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.messages.len(), 2);
            },
            _ => panic!("Expected ChatCompletionsRequest variant"),
        }
    }

    #[test]
    fn test_anthropic_request_from_bytes_with_endpoint() {
        let req = json!({
            "model": "claude-3-sonnet",
            "system": "You are a helpful assistant",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": "Hello!"}
            ]
        });
        let bytes = serde_json::to_vec(&req).unwrap();
        let endpoint = SupportedApi::Anthropic(Messages);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &endpoint));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::MessagesRequest(r) => {
                assert_eq!(r.model, "claude-3-sonnet");
                assert_eq!(r.messages.len(), 1);
            },
            _ => panic!("Expected MessagesRequest variant"),
        }
    }

    #[test]
    fn test_openai_request_from_bytes_with_endpoint() {
        let req = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant"},
                {"role": "user", "content": "Hello!"}
            ]
        });
        let bytes = serde_json::to_vec(&req).unwrap();
        let endpoint = SupportedApi::OpenAI(ChatCompletions);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &endpoint));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ChatCompletionsRequest(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.messages.len(), 2);
            },
            _ => panic!("Expected ChatCompletionsRequest variant"),
        }
    }

    #[test]
    fn test_anthropic_request_from_bytes_wrong_endpoint() {
        let req = json!({
            "model": "claude-3-sonnet",
            "system": "You are a helpful assistant",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ]
        });
        let bytes = serde_json::to_vec(&req).unwrap();
        // Intentionally use OpenAI endpoint for Anthropic payload
        let endpoint = SupportedApi::OpenAI(ChatCompletions);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &endpoint));
        // Should parse as ChatCompletionsRequest, not error
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ChatCompletionsRequest(r) => {
                assert_eq!(r.model, "claude-3-sonnet");
                assert_eq!(r.messages.len(), 1);
            },
            _ => panic!("Expected ChatCompletionsRequest variant"),
        }
    }

    #[test]
    fn test_v1_messages_to_v1_chat_completions_roundtrip() {
        let anthropic_req = AnthropicMessagesRequest {
            model: "claude-3-sonnet".to_string(),
            system: Some(crate::apis::anthropic::MessagesSystemPrompt::Single("You are a helpful assistant".to_string())),
            messages: vec![
                crate::apis::anthropic::MessagesMessage {
                    role: crate::apis::anthropic::MessagesRole::User,
                    content: crate::apis::anthropic::MessagesMessageContent::Single("Hello!".to_string()),
                }
            ],
            max_tokens: 128,
            container: None,
            mcp_servers: None,
            service_tier: None,
            thinking: None,
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            stream: Some(false),
            stop_sequences: Some(vec!["\n".to_string()]),
            tools: None,
            tool_choice: None,
            metadata: None,
        };

        let openai_req = ChatCompletionsRequest::try_from(anthropic_req.clone()).expect("Anthropic->OpenAI conversion failed");
        let anthropic_req2 = AnthropicMessagesRequest::try_from(openai_req).expect("OpenAI->Anthropic conversion failed");

        assert_eq!(anthropic_req.model, anthropic_req2.model);
        // Compare system prompt text if present
        assert_eq!(
            anthropic_req.system.as_ref().and_then(|s| match s { crate::apis::anthropic::MessagesSystemPrompt::Single(t) => Some(t), _ => None }),
            anthropic_req2.system.as_ref().and_then(|s| match s { crate::apis::anthropic::MessagesSystemPrompt::Single(t) => Some(t), _ => None })
        );
        assert_eq!(anthropic_req.messages[0].role, anthropic_req2.messages[0].role);
        // Compare message content text if present
        assert_eq!(
            anthropic_req.messages[0].content.extract_text(),
            anthropic_req2.messages[0].content.extract_text()
        );
        assert_eq!(anthropic_req.max_tokens, anthropic_req2.max_tokens);
    }

        #[test]
        fn test_v1_chat_completions_to_v1_messages_roundtrip() {
            use crate::apis::anthropic::MessagesRequest as AnthropicMessagesRequest;
            use crate::apis::openai::{ChatCompletionsRequest, Message, Role, MessageContent};

            let openai_req = ChatCompletionsRequest {
                model: "gpt-4".to_string(),
                messages: vec![
                    Message {
                        role: Role::System,
                        content: MessageContent::Text("You are a helpful assistant".to_string()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    Message {
                        role: Role::User,
                        content: MessageContent::Text("Hello!".to_string()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    }
                ],
                temperature: Some(0.7),
                top_p: Some(1.0),
                max_tokens: Some(128),
                stream: Some(false),
                stop: Some(vec!["\n".to_string()]),
                tools: None,
                tool_choice: None,
                parallel_tool_calls: None,
                ..Default::default()
            };

            let anthropic_req = AnthropicMessagesRequest::try_from(openai_req.clone()).expect("OpenAI->Anthropic conversion failed");
            let openai_req2 = ChatCompletionsRequest::try_from(anthropic_req).expect("Anthropic->OpenAI conversion failed");

            assert_eq!(openai_req.model, openai_req2.model);
            assert_eq!(openai_req.messages[0].role, openai_req2.messages[0].role);
            assert_eq!(openai_req.messages[0].content.extract_text(), openai_req2.messages[0].content.extract_text());
            assert_eq!(openai_req.max_tokens, openai_req2.max_tokens);
        }
}
