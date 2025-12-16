use crate::apis::anthropic::MessagesRequest;
use crate::apis::openai::ChatCompletionsRequest;

use crate::apis::amazon_bedrock::{ConverseRequest, ConverseStreamRequest};
use crate::apis::openai_responses::ResponsesAPIRequest;
use crate::clients::endpoints::SupportedAPIsFromClient;
use crate::clients::endpoints::SupportedUpstreamAPIs;

use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
#[derive(Clone, Debug)]
pub enum ProviderRequestType {
    ChatCompletionsRequest(ChatCompletionsRequest),
    MessagesRequest(MessagesRequest),
    BedrockConverse(ConverseRequest),
    BedrockConverseStream(ConverseStreamRequest),
    ResponsesAPIRequest(ResponsesAPIRequest),
    //add more request types here
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

    /// Get tool names if tools are defined in the request
    fn get_tool_names(&self) -> Option<Vec<String>>;

    /// Convert the request to bytes for transmission
    fn to_bytes(&self) -> Result<Vec<u8>, ProviderRequestError>;

    fn metadata(&self) -> &Option<HashMap<String, Value>>;

    /// Remove a metadata key from the request and return true if the key was present
    fn remove_metadata_key(&mut self, key: &str) -> bool;

    fn get_temperature(&self) -> Option<f32>;
}

impl ProviderRequestType {
    /// Get message history as OpenAI Message format
    /// This is useful for processing chat history across different provider formats
    pub fn get_message_history(&self) -> Vec<crate::apis::openai::Message> {
        use crate::apis::openai::{Message, MessageContent, Role};

        match self {
            Self::ChatCompletionsRequest(r) => r.messages.clone(),
            Self::MessagesRequest(r) => {
                // Convert Anthropic messages to OpenAI format
                let mut openai_messages = Vec::new();

                // Add system prompt as system message if present
                if let Some(system) = &r.system {
                    openai_messages.push(system.clone().into());
                }

                // Convert each Anthropic message to OpenAI format
                for msg in &r.messages {
                    if let Ok(converted_msgs) = TryInto::<Vec<Message>>::try_into(msg.clone()) {
                        openai_messages.extend(converted_msgs);
                    }
                }

                openai_messages
            }
            Self::BedrockConverse(r) => {
                // Convert Bedrock messages to OpenAI format
                let mut openai_messages = Vec::new();

                // Add system messages if present
                if let Some(system) = &r.system {
                    for sys_block in system {
                        match sys_block {
                            crate::apis::amazon_bedrock::SystemContentBlock::Text { text } => {
                                openai_messages.push(Message {
                                    role: Role::System,
                                    content: MessageContent::Text(text.clone()),
                                    name: None,
                                    tool_calls: None,
                                    tool_call_id: None,
                                });
                            }
                            _ => {} // Skip other system content types
                        }
                    }
                }

                // Convert conversation messages
                if let Some(messages) = &r.messages {
                    for msg in messages {
                        let role = match msg.role {
                            crate::apis::amazon_bedrock::ConversationRole::User => Role::User,
                            crate::apis::amazon_bedrock::ConversationRole::Assistant => Role::Assistant,
                        };

                        // Extract text from content blocks
                        let content = msg.content.iter()
                            .filter_map(|block| {
                                if let crate::apis::amazon_bedrock::ContentBlock::Text { text } = block {
                                    Some(text.clone())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        openai_messages.push(Message {
                            role,
                            content: MessageContent::Text(content),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                }

                openai_messages
            }
            Self::BedrockConverseStream(r) => {
                // Same as BedrockConverse
                let mut openai_messages = Vec::new();

                if let Some(system) = &r.system {
                    for sys_block in system {
                        match sys_block {
                            crate::apis::amazon_bedrock::SystemContentBlock::Text { text } => {
                                openai_messages.push(Message {
                                    role: Role::System,
                                    content: MessageContent::Text(text.clone()),
                                    name: None,
                                    tool_calls: None,
                                    tool_call_id: None,
                                });
                            }
                            _ => {} // Skip other system content types
                        }
                    }
                }

                if let Some(messages) = &r.messages {
                    for msg in messages {
                        let role = match msg.role {
                            crate::apis::amazon_bedrock::ConversationRole::User => Role::User,
                            crate::apis::amazon_bedrock::ConversationRole::Assistant => Role::Assistant,
                        };

                        let content = msg.content.iter()
                            .filter_map(|block| {
                                if let crate::apis::amazon_bedrock::ContentBlock::Text { text } = block {
                                    Some(text.clone())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        openai_messages.push(Message {
                            role,
                            content: MessageContent::Text(content),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                }

                openai_messages
            }
            Self::ResponsesAPIRequest(r) => {
                // Convert ResponsesAPIRequest input to a user message
                let mut openai_messages = Vec::new();

                // Add instructions as system message if present
                if let Some(instructions) = &r.instructions {
                    openai_messages.push(Message {
                        role: Role::System,
                        content: MessageContent::Text(instructions.clone()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }

                // Convert input to messages
                use crate::apis::openai_responses::{InputParam, InputItem};
                match &r.input {
                    InputParam::Text(text) => {
                        openai_messages.push(Message {
                            role: Role::User,
                            content: MessageContent::Text(text.clone()),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                    InputParam::Items(items) => {
                        for item in items {
                            match item {
                                InputItem::Message(msg) => {
                                    // Convert message role
                                    let role = match msg.role {
                                        crate::apis::openai_responses::MessageRole::User => Role::User,
                                        crate::apis::openai_responses::MessageRole::Assistant => Role::Assistant,
                                        crate::apis::openai_responses::MessageRole::System => Role::System,
                                        crate::apis::openai_responses::MessageRole::Developer => Role::System, // Map developer to system
                                    };

                                    // Extract text from message content
                                    let content = msg.content.iter()
                                        .filter_map(|c| {
                                            if let crate::apis::openai_responses::InputContent::InputText { text } = c {
                                                Some(text.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");

                                    openai_messages.push(Message {
                                        role,
                                        content: MessageContent::Text(content),
                                        name: None,
                                        tool_calls: None,
                                        tool_call_id: None,
                                    });
                                }
                            }
                        }
                    }
                }

                openai_messages
            }
        }
    }

    /// Set message history from OpenAI Message format
    /// This converts OpenAI messages to the appropriate format for each provider type
    pub fn set_messages(&mut self, messages: &[crate::apis::openai::Message]) {
        match self {
            Self::ChatCompletionsRequest(r) => {
                r.messages = messages.to_vec();
            }
            Self::MessagesRequest(r) => {
                // Convert OpenAI messages to Anthropic format
                // Separate system messages from regular messages
                let mut system_messages = Vec::new();
                let mut regular_messages = Vec::new();

                for msg in messages {
                    if msg.role == crate::apis::openai::Role::System {
                        system_messages.push(msg.clone());
                    } else {
                        regular_messages.push(msg.clone());
                    }
                }

                // Set system prompt if there are system messages
                if !system_messages.is_empty() {
                    // Combine all system messages into one
                    let system_text = system_messages.iter()
                        .filter_map(|msg| {
                            if let crate::apis::openai::MessageContent::Text(text) = &msg.content {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    r.system = Some(crate::apis::anthropic::MessagesSystemPrompt::Single(system_text));
                }

                // Convert regular messages
                r.messages = regular_messages.iter()
                    .filter_map(|msg| {
                        msg.clone().try_into().ok()
                    })
                    .collect();
            }
            Self::BedrockConverse(r) | Self::BedrockConverseStream(r) => {
                // Convert OpenAI messages to Bedrock format
                use crate::apis::amazon_bedrock::{ContentBlock, ConversationRole, SystemContentBlock};

                let mut system_blocks = Vec::new();
                let mut bedrock_messages = Vec::new();

                for msg in messages {
                    match msg.role {
                        crate::apis::openai::Role::System => {
                            if let crate::apis::openai::MessageContent::Text(text) = &msg.content {
                                system_blocks.push(SystemContentBlock::Text { text: text.clone() });
                            }
                        }
                        crate::apis::openai::Role::User | crate::apis::openai::Role::Assistant => {
                            let role = match msg.role {
                                crate::apis::openai::Role::User => ConversationRole::User,
                                crate::apis::openai::Role::Assistant => ConversationRole::Assistant,
                                _ => continue,
                            };

                            let content = if let crate::apis::openai::MessageContent::Text(text) = &msg.content {
                                vec![ContentBlock::Text { text: text.clone() }]
                            } else {
                                vec![]
                            };

                            bedrock_messages.push(crate::apis::amazon_bedrock::Message {
                                role,
                                content,
                            });
                        }
                        _ => {}
                    }
                }

                if !system_blocks.is_empty() {
                    r.system = Some(system_blocks);
                }
                r.messages = Some(bedrock_messages);
            }
            Self::ResponsesAPIRequest(r) => {
                // For ResponsesAPI, we need to convert messages back to input format
                // Extract system messages as instructions
                let system_text = messages.iter()
                    .filter(|msg| msg.role == crate::apis::openai::Role::System)
                    .filter_map(|msg| {
                        if let crate::apis::openai::MessageContent::Text(text) = &msg.content {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !system_text.is_empty() {
                    r.instructions = Some(system_text);
                }

                // Convert user/assistant messages to InputParam
                // For simplicity, we'll use the last user message as the input
                // or combine all non-system messages
                let input_messages: Vec<_> = messages.iter()
                    .filter(|msg| msg.role != crate::apis::openai::Role::System)
                    .collect();

                if !input_messages.is_empty() {
                    // If there's only one message, use Text format
                    if input_messages.len() == 1 {
                        if let crate::apis::openai::MessageContent::Text(text) = &input_messages[0].content {
                            r.input = crate::apis::openai_responses::InputParam::Text(text.clone());
                        }
                    } else {
                        // Multiple messages - combine them as text for now
                        // A more sophisticated approach would use InputParam::Items
                        let combined_text = input_messages.iter()
                            .filter_map(|msg| {
                                if let crate::apis::openai::MessageContent::Text(text) = &msg.content {
                                    Some(format!("{}: {}",
                                        match msg.role {
                                            crate::apis::openai::Role::User => "User",
                                            crate::apis::openai::Role::Assistant => "Assistant",
                                            _ => "Unknown",
                                        },
                                        text
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        r.input = crate::apis::openai_responses::InputParam::Text(combined_text);
                    }
                }
            }
        }
    }
}

impl ProviderRequest for ProviderRequestType {
    fn model(&self) -> &str {
        match self {
            Self::ChatCompletionsRequest(r) => r.model(),
            Self::MessagesRequest(r) => r.model(),
            Self::BedrockConverse(r) => r.model(),
            Self::BedrockConverseStream(r) => r.model(),
            Self::ResponsesAPIRequest(r) => r.model(),
        }
    }

    fn set_model(&mut self, model: String) {
        match self {
            Self::ChatCompletionsRequest(r) => r.set_model(model),
            Self::MessagesRequest(r) => r.set_model(model),
            Self::BedrockConverse(r) => r.set_model(model),
            Self::BedrockConverseStream(r) => r.set_model(model),
            Self::ResponsesAPIRequest(r) => r.set_model(model),
        }
    }

    fn is_streaming(&self) -> bool {
        match self {
            Self::ChatCompletionsRequest(r) => r.is_streaming(),
            Self::MessagesRequest(r) => r.is_streaming(),
            Self::BedrockConverse(_) => false,
            Self::BedrockConverseStream(_) => true,
            Self::ResponsesAPIRequest(r) => r.is_streaming(),
        }
    }

    fn extract_messages_text(&self) -> String {
        match self {
            Self::ChatCompletionsRequest(r) => r.extract_messages_text(),
            Self::MessagesRequest(r) => r.extract_messages_text(),
            Self::BedrockConverse(r) => r.extract_messages_text(),
            Self::BedrockConverseStream(r) => r.extract_messages_text(),
            Self::ResponsesAPIRequest(r) => r.extract_messages_text(),
        }
    }

    fn get_recent_user_message(&self) -> Option<String> {
        match self {
            Self::ChatCompletionsRequest(r) => r.get_recent_user_message(),
            Self::MessagesRequest(r) => r.get_recent_user_message(),
            Self::BedrockConverse(r) => r.get_recent_user_message(),
            Self::BedrockConverseStream(r) => r.get_recent_user_message(),
            Self::ResponsesAPIRequest(r) => r.get_recent_user_message(),
        }
    }

    fn get_tool_names(&self) -> Option<Vec<String>> {
        match self {
            Self::ChatCompletionsRequest(r) => r.get_tool_names(),
            Self::MessagesRequest(r) => r.get_tool_names(),
            Self::BedrockConverse(r) => r.get_tool_names(),
            Self::BedrockConverseStream(r) => r.get_tool_names(),
            Self::ResponsesAPIRequest(r) => r.get_tool_names(),
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>, ProviderRequestError> {
        match self {
            Self::ChatCompletionsRequest(r) => r.to_bytes(),
            Self::MessagesRequest(r) => r.to_bytes(),
            Self::BedrockConverse(r) => r.to_bytes(),
            Self::BedrockConverseStream(r) => r.to_bytes(),
            Self::ResponsesAPIRequest(r) => r.to_bytes(),
        }
    }

    fn metadata(&self) -> &Option<HashMap<String, Value>> {
        match self {
            Self::ChatCompletionsRequest(r) => r.metadata(),
            Self::MessagesRequest(r) => r.metadata(),
            Self::BedrockConverse(r) => r.metadata(),
            Self::BedrockConverseStream(r) => r.metadata(),
            Self::ResponsesAPIRequest(r) => r.metadata(),
        }
    }

    fn remove_metadata_key(&mut self, key: &str) -> bool {
        match self {
            Self::ChatCompletionsRequest(r) => r.remove_metadata_key(key),
            Self::MessagesRequest(r) => r.remove_metadata_key(key),
            Self::BedrockConverse(r) => r.remove_metadata_key(key),
            Self::BedrockConverseStream(r) => r.remove_metadata_key(key),
            Self::ResponsesAPIRequest(r) => r.remove_metadata_key(key),
        }
    }

    fn get_temperature(&self) -> Option<f32> {
        match self {
            Self::ChatCompletionsRequest(r) => r.get_temperature(),
            Self::MessagesRequest(r) => r.get_temperature(),
            Self::BedrockConverse(r) => r.get_temperature(),
            Self::BedrockConverseStream(r) => r.get_temperature(),
            Self::ResponsesAPIRequest(r) => r.get_temperature(),
        }
    }
}

/// Parse the client API from a byte slice.
impl TryFrom<(&[u8], &SupportedAPIsFromClient)> for ProviderRequestType {
    type Error = std::io::Error;

    fn try_from((bytes, client_api): (&[u8], &SupportedAPIsFromClient)) -> Result<Self, Self::Error> {
        // Use SupportedApi to determine the appropriate request type
        match client_api {
            SupportedAPIsFromClient::OpenAIChatCompletions(_) => {
                let chat_completion_request: ChatCompletionsRequest =
                    ChatCompletionsRequest::try_from(bytes)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderRequestType::ChatCompletionsRequest(
                    chat_completion_request,
                ))
            }
            SupportedAPIsFromClient::AnthropicMessagesAPI(_) => {
                let messages_request: MessagesRequest = MessagesRequest::try_from(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderRequestType::MessagesRequest(messages_request))
            }

            SupportedAPIsFromClient::OpenAIResponsesAPI(_) => {
                let responses_apirequest: ResponsesAPIRequest =
                    ResponsesAPIRequest::try_from(bytes)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(ProviderRequestType::ResponsesAPIRequest(
                    responses_apirequest,
                ))
            }
        }
    }
}

/// Conversion from one ProviderRequestType to a different ProviderRequestType (SupportedAPIs)
impl TryFrom<(ProviderRequestType, &SupportedUpstreamAPIs)> for ProviderRequestType {
    type Error = ProviderRequestError;

    fn try_from(
        (client_request, upstream_api): (ProviderRequestType, &SupportedUpstreamAPIs),
    ) -> Result<Self, Self::Error> {
        match (client_request, upstream_api) {
            // ============================================================================
            // ChatCompletionsRequest conversions
            // ============================================================================
            (
                ProviderRequestType::ChatCompletionsRequest(chat_req),
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
            ) => Ok(ProviderRequestType::ChatCompletionsRequest(chat_req)),
            (
                ProviderRequestType::ChatCompletionsRequest(chat_req),
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
            ) => {
                let messages_req =
                    MessagesRequest::try_from(chat_req).map_err(|e| ProviderRequestError {
                        message: format!(
                            "Failed to convert ChatCompletionsRequest to MessagesRequest: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    })?;
                Ok(ProviderRequestType::MessagesRequest(messages_req))
            }
            (
                ProviderRequestType::ChatCompletionsRequest(chat_req),
                SupportedUpstreamAPIs::AmazonBedrockConverse(_),
            ) => {
                let bedrock_req = ConverseRequest::try_from(chat_req)
                    .map_err(|e| ProviderRequestError {
                        message: format!("Failed to convert ChatCompletionsRequest to Amazon Bedrock request: {}", e),
                        source: Some(Box::new(e))
                    })?;
                Ok(ProviderRequestType::BedrockConverse(bedrock_req))
            }
            (
                ProviderRequestType::ChatCompletionsRequest(chat_req),
                SupportedUpstreamAPIs::AmazonBedrockConverseStream(_),
            ) => {
                let bedrock_req = ConverseStreamRequest::try_from(chat_req)
                    .map_err(|e| ProviderRequestError {
                        message: format!("Failed to convert ChatCompletionsRequest to Amazon Bedrock Stream request: {}", e),
                        source: Some(Box::new(e))
                    })?;
                Ok(ProviderRequestType::BedrockConverseStream(bedrock_req))
            }
            (
                ProviderRequestType::ChatCompletionsRequest(_),
                SupportedUpstreamAPIs::OpenAIResponsesAPI(_),
            ) => {
                Err(ProviderRequestError {
                    message: "Conversion from ChatCompletionsRequest to ResponsesAPIRequest is not supported. ResponsesAPI can only be used as a client API, not as an upstream API.".to_string(),
                    source: None,
                })
            }

            // ============================================================================
            // MessagesRequest conversions
            // ============================================================================
            (
                ProviderRequestType::MessagesRequest(messages_req),
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
            ) => Ok(ProviderRequestType::MessagesRequest(messages_req)),
            (
                ProviderRequestType::MessagesRequest(messages_req),
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
            ) => {
                let chat_req = ChatCompletionsRequest::try_from(messages_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert MessagesRequest to ChatCompletionsRequest: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;
                Ok(ProviderRequestType::ChatCompletionsRequest(chat_req))
            }
            (
                ProviderRequestType::MessagesRequest(messages_req),
                SupportedUpstreamAPIs::AmazonBedrockConverse(_),
            ) => {
                let bedrock_req =
                    ConverseRequest::try_from(messages_req).map_err(|e| ProviderRequestError {
                        message: format!(
                            "Failed to convert MessagesRequest to Amazon Bedrock request: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    })?;
                Ok(ProviderRequestType::BedrockConverse(bedrock_req))
            }
            (
                ProviderRequestType::MessagesRequest(messages_req),
                SupportedUpstreamAPIs::AmazonBedrockConverseStream(_),
            ) => {
                let bedrock_req = ConverseStreamRequest::try_from(messages_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert MessagesRequest to Amazon Bedrock Stream request: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;
                Ok(ProviderRequestType::BedrockConverseStream(bedrock_req))
            }
            (
                ProviderRequestType::MessagesRequest(_),
                SupportedUpstreamAPIs::OpenAIResponsesAPI(_),
            ) => {
                Err(ProviderRequestError {
                    message: "Conversion from MessagesRequest to ResponsesAPIRequest is not supported. ResponsesAPI can only be used as a client API, not as an upstream API.".to_string(),
                    source: None,
                })
            }

            // ============================================================================
            // ResponsesAPIRequest conversions (only converts TO other formats)
            // ============================================================================
            (
                ProviderRequestType::ResponsesAPIRequest(responses_req),
                SupportedUpstreamAPIs::OpenAIResponsesAPI(_),
            ) => Ok(ProviderRequestType::ResponsesAPIRequest(responses_req)),

            // ResponsesAPI -> ChatCompletions (direct conversion)
            (
                ProviderRequestType::ResponsesAPIRequest(responses_req),
                SupportedUpstreamAPIs::OpenAIChatCompletions(_),
            ) => {
                let chat_req = ChatCompletionsRequest::try_from(responses_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert ResponsesAPIRequest to ChatCompletionsRequest: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;
                Ok(ProviderRequestType::ChatCompletionsRequest(chat_req))
            }

            // ResponsesAPI -> Anthropic Messages (via ChatCompletions)
            (
                ProviderRequestType::ResponsesAPIRequest(responses_req),
                SupportedUpstreamAPIs::AnthropicMessagesAPI(_),
            ) => {
                // Chain: ResponsesAPI -> ChatCompletions -> MessagesRequest
                let chat_req = ChatCompletionsRequest::try_from(responses_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert ResponsesAPIRequest to ChatCompletionsRequest: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;

                let messages_req = MessagesRequest::try_from(chat_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert ChatCompletionsRequest to MessagesRequest: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;
                Ok(ProviderRequestType::MessagesRequest(messages_req))
            }

            // ResponsesAPI -> Bedrock Converse (via ChatCompletions)
            (
                ProviderRequestType::ResponsesAPIRequest(responses_req),
                SupportedUpstreamAPIs::AmazonBedrockConverse(_),
            ) => {
                // Chain: ResponsesAPI -> ChatCompletions -> ConverseRequest
                let chat_req = ChatCompletionsRequest::try_from(responses_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert ResponsesAPIRequest to ChatCompletionsRequest: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;

                let bedrock_req = ConverseRequest::try_from(chat_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert ChatCompletionsRequest to Amazon Bedrock request: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;
                Ok(ProviderRequestType::BedrockConverse(bedrock_req))
            }

            // ResponsesAPI -> Bedrock Converse Stream (via ChatCompletions)
            (
                ProviderRequestType::ResponsesAPIRequest(responses_req),
                SupportedUpstreamAPIs::AmazonBedrockConverseStream(_),
            ) => {
                // Chain: ResponsesAPI -> ChatCompletions -> ConverseStreamRequest
                let chat_req = ChatCompletionsRequest::try_from(responses_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert ResponsesAPIRequest to ChatCompletionsRequest: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;

                let bedrock_req = ConverseStreamRequest::try_from(chat_req).map_err(|e| {
                    ProviderRequestError {
                        message: format!(
                            "Failed to convert ChatCompletionsRequest to Amazon Bedrock Stream request: {}",
                            e
                        ),
                        source: Some(Box::new(e)),
                    }
                })?;
                Ok(ProviderRequestType::BedrockConverseStream(bedrock_req))
            }

            // ============================================================================
            // Amazon Bedrock conversions (not supported as client API)
            // ============================================================================

            (ProviderRequestType::BedrockConverse(_), _) => {
                Err(ProviderRequestError {
                    message: "Amazon Bedrock Converse is not supported as a client API. Only OpenAI ChatCompletions, Anthropic Messages, and OpenAI Responses APIs are supported as client APIs.".to_string(),
                    source: None,
                })
            }

            (ProviderRequestType::BedrockConverseStream(_), _) => {
                Err(ProviderRequestError {
                    message: "Amazon Bedrock Converse Stream is not supported as a client API. Only OpenAI ChatCompletions, Anthropic Messages, and OpenAI Responses APIs are supported as client APIs.".to_string(),
                    source: None,
                })
            }
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
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::anthropic::AnthropicApi::Messages;
    use crate::apis::anthropic::MessagesRequest as AnthropicMessagesRequest;
    use crate::apis::openai::ChatCompletionsRequest;
    use crate::apis::openai::OpenAIApi::ChatCompletions;
    use crate::clients::endpoints::SupportedAPIsFromClient;
    use crate::transforms::lib::ExtractText;
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
        let api = SupportedAPIsFromClient::OpenAIChatCompletions(ChatCompletions);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &api));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ChatCompletionsRequest(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.messages.len(), 2);
            }
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
        let endpoint = SupportedAPIsFromClient::AnthropicMessagesAPI(Messages);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &endpoint));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::MessagesRequest(r) => {
                assert_eq!(r.model, "claude-3-sonnet");
                assert_eq!(r.messages.len(), 1);
            }
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
        let endpoint = SupportedAPIsFromClient::OpenAIChatCompletions(ChatCompletions);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &endpoint));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ChatCompletionsRequest(r) => {
                assert_eq!(r.model, "gpt-4");
                assert_eq!(r.messages.len(), 2);
            }
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
        let endpoint = SupportedAPIsFromClient::OpenAIChatCompletions(ChatCompletions);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &endpoint));
        // Should parse as ChatCompletionsRequest, not error
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ChatCompletionsRequest(r) => {
                assert_eq!(r.model, "claude-3-sonnet");
                assert_eq!(r.messages.len(), 1);
            }
            _ => panic!("Expected ChatCompletionsRequest variant"),
        }
    }

    #[test]
    fn test_v1_messages_to_v1_chat_completions_roundtrip() {
        let anthropic_req = AnthropicMessagesRequest {
            model: "claude-3-sonnet".to_string(),
            system: Some(crate::apis::anthropic::MessagesSystemPrompt::Single(
                "You are a helpful assistant".to_string(),
            )),
            messages: vec![crate::apis::anthropic::MessagesMessage {
                role: crate::apis::anthropic::MessagesRole::User,
                content: crate::apis::anthropic::MessagesMessageContent::Single(
                    "Hello!".to_string(),
                ),
            }],
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

        let openai_req = ChatCompletionsRequest::try_from(anthropic_req.clone())
            .expect("Anthropic->OpenAI conversion failed");
        let anthropic_req2 = AnthropicMessagesRequest::try_from(openai_req)
            .expect("OpenAI->Anthropic conversion failed");

        assert_eq!(anthropic_req.model, anthropic_req2.model);
        // Compare system prompt text if present
        assert_eq!(
            anthropic_req.system.as_ref().and_then(|s| match s {
                crate::apis::anthropic::MessagesSystemPrompt::Single(t) => Some(t),
                _ => None,
            }),
            anthropic_req2.system.as_ref().and_then(|s| match s {
                crate::apis::anthropic::MessagesSystemPrompt::Single(t) => Some(t),
                _ => None,
            })
        );
        assert_eq!(
            anthropic_req.messages[0].role,
            anthropic_req2.messages[0].role
        );
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
        use crate::apis::openai::{ChatCompletionsRequest, Message, MessageContent, Role};

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
                },
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

        let anthropic_req = AnthropicMessagesRequest::try_from(openai_req.clone())
            .expect("OpenAI->Anthropic conversion failed");
        let openai_req2 = ChatCompletionsRequest::try_from(anthropic_req)
            .expect("Anthropic->OpenAI conversion failed");

        assert_eq!(openai_req.model, openai_req2.model);
        assert_eq!(openai_req.messages[0].role, openai_req2.messages[0].role);
        assert_eq!(
            openai_req.messages[0].content.extract_text(),
            openai_req2.messages[0].content.extract_text()
        );
        // After roundtrip, deprecated max_tokens should be converted to max_completion_tokens
        let original_max_tokens = openai_req.max_completion_tokens.or(openai_req.max_tokens);
        let roundtrip_max_tokens = openai_req2.max_completion_tokens.or(openai_req2.max_tokens);
        assert_eq!(original_max_tokens, roundtrip_max_tokens);
    }

    #[test]
    fn test_responses_api_request_from_bytes() {
        use crate::apis::openai::OpenAIApi::Responses;

        let req = json!({
            "model": "gpt-4o",
            "input": "Hello, how are you?"
        });
        let bytes = serde_json::to_vec(&req).unwrap();
        let api = SupportedAPIsFromClient::OpenAIResponsesAPI(Responses);
        let result = ProviderRequestType::try_from((bytes.as_slice(), &api));
        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ResponsesAPIRequest(r) => {
                assert_eq!(r.model, "gpt-4o");
            }
            _ => panic!("Expected ResponsesAPIRequest variant"),
        }
    }

    #[test]
    fn test_responses_api_to_chat_completions_conversion() {
        use crate::apis::openai::OpenAIApi::ChatCompletions;
        use crate::apis::openai_responses::{InputParam, ResponsesAPIRequest};

        let responses_req = ResponsesAPIRequest {
            model: "gpt-4o".to_string(),
            input: InputParam::Text("Hello, world!".to_string()),
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_output_tokens: Some(100),
            stream: Some(false),
            metadata: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            instructions: None,
            modalities: None,
            user: None,
            store: None,
            reasoning_effort: None,
            include: None,
            audio: None,
            text: None,
            service_tier: None,
            top_logprobs: None,
            stream_options: None,
            truncation: None,
            conversation: None,
            previous_response_id: None,
            max_tool_calls: None,
            background: None,
        };

        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(ChatCompletions);
        let result = ProviderRequestType::try_from((
            ProviderRequestType::ResponsesAPIRequest(responses_req),
            &upstream_api,
        ));

        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::ChatCompletionsRequest(chat_req) => {
                assert_eq!(chat_req.model, "gpt-4o");
                assert_eq!(chat_req.temperature, Some(0.7));
                assert_eq!(chat_req.top_p, Some(0.9));
                assert_eq!(chat_req.max_completion_tokens, Some(100));
                assert_eq!(chat_req.messages.len(), 1);
            }
            _ => panic!("Expected ChatCompletionsRequest variant"),
        }
    }

    #[test]
    fn test_responses_api_to_anthropic_messages_conversion() {
        use crate::apis::anthropic::AnthropicApi::Messages;
        use crate::apis::openai_responses::{InputParam, ResponsesAPIRequest};

        let responses_req = ResponsesAPIRequest {
            model: "gpt-4o".to_string(),
            input: InputParam::Text("Hello, Claude!".to_string()),
            temperature: Some(0.8),
            max_output_tokens: Some(150),
            stream: Some(false),
            metadata: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            instructions: Some("You are a helpful assistant".to_string()),
            modalities: None,
            user: None,
            store: None,
            reasoning_effort: None,
            include: None,
            audio: None,
            text: None,
            service_tier: None,
            top_p: None,
            top_logprobs: None,
            stream_options: None,
            truncation: None,
            conversation: None,
            previous_response_id: None,
            max_tool_calls: None,
            background: None,
        };

        let upstream_api = SupportedUpstreamAPIs::AnthropicMessagesAPI(Messages);
        let result = ProviderRequestType::try_from((
            ProviderRequestType::ResponsesAPIRequest(responses_req),
            &upstream_api,
        ));

        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::MessagesRequest(messages_req) => {
                assert_eq!(messages_req.model, "gpt-4o");
                assert_eq!(messages_req.temperature, Some(0.8));
                assert_eq!(messages_req.max_tokens, 150);
                // Instructions should be converted to system prompt via ChatCompletions conversion
                // The conversion chain: ResponsesAPI -> ChatCompletions (system message) -> Anthropic (system prompt)
                // But we need to check if the system prompt was actually set
                assert_eq!(messages_req.messages.len(), 1);
            }
            _ => panic!("Expected MessagesRequest variant"),
        }
    }

    #[test]
    fn test_responses_api_to_bedrock_conversion() {
        use crate::apis::amazon_bedrock::AmazonBedrockApi::Converse;
        use crate::apis::openai_responses::{InputParam, ResponsesAPIRequest};

        let responses_req = ResponsesAPIRequest {
            model: "gpt-4o".to_string(),
            input: InputParam::Text("Hello, Bedrock!".to_string()),
            temperature: Some(0.5),
            max_output_tokens: Some(200),
            stream: Some(false),
            metadata: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            instructions: None,
            modalities: None,
            user: None,
            store: None,
            reasoning_effort: None,
            include: None,
            audio: None,
            text: None,
            service_tier: None,
            top_p: None,
            top_logprobs: None,
            stream_options: None,
            truncation: None,
            conversation: None,
            previous_response_id: None,
            max_tool_calls: None,
            background: None,
        };

        let upstream_api = SupportedUpstreamAPIs::AmazonBedrockConverse(Converse);
        let result = ProviderRequestType::try_from((
            ProviderRequestType::ResponsesAPIRequest(responses_req),
            &upstream_api,
        ));

        assert!(result.is_ok());
        match result.unwrap() {
            ProviderRequestType::BedrockConverse(bedrock_req) => {
                assert_eq!(bedrock_req.model_id, "gpt-4o");
                // Bedrock receives the converted request through ChatCompletions
                assert!(!bedrock_req.messages.is_none());
            }
            _ => panic!("Expected BedrockConverse variant"),
        }
    }

    #[test]
    fn test_chat_completions_to_responses_api_not_supported() {
        use crate::apis::openai::OpenAIApi::Responses;
        use crate::apis::openai::{Message, MessageContent, Role};

        let chat_req = ChatCompletionsRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: MessageContent::Text("Hello!".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            ..Default::default()
        };

        let upstream_api = SupportedUpstreamAPIs::OpenAIResponsesAPI(Responses);
        let result = ProviderRequestType::try_from((
            ProviderRequestType::ChatCompletionsRequest(chat_req),
            &upstream_api,
        ));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("ResponsesAPI can only be used as a client API"));
    }

    #[test]
    fn test_anthropic_messages_to_responses_api_not_supported() {
        use crate::apis::anthropic::MessagesRequest as AnthropicMessagesRequest;
        use crate::apis::openai::OpenAIApi::Responses;

        let messages_req = AnthropicMessagesRequest {
            model: "claude-3-sonnet".to_string(),
            messages: vec![crate::apis::anthropic::MessagesMessage {
                role: crate::apis::anthropic::MessagesRole::User,
                content: crate::apis::anthropic::MessagesMessageContent::Single(
                    "Hello!".to_string(),
                ),
            }],
            max_tokens: 100,
            container: None,
            mcp_servers: None,
            service_tier: None,
            thinking: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stream: None,
            stop_sequences: None,
            system: None,
            tools: None,
            tool_choice: None,
            metadata: None,
        };

        let upstream_api = SupportedUpstreamAPIs::OpenAIResponsesAPI(Responses);
        let result = ProviderRequestType::try_from((
            ProviderRequestType::MessagesRequest(messages_req),
            &upstream_api,
        ));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("ResponsesAPI can only be used as a client API"));
    }

    #[test]
    fn test_bedrock_as_client_api_not_supported() {
        use crate::apis::openai::OpenAIApi::ChatCompletions;

        // Create a simple Bedrock request (we'll use Default if available, or minimal construction)
        let bedrock_req = ConverseRequest::default();

        let upstream_api = SupportedUpstreamAPIs::OpenAIChatCompletions(ChatCompletions);
        let result = ProviderRequestType::try_from((
            ProviderRequestType::BedrockConverse(bedrock_req),
            &upstream_api,
        ));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("not supported as a client API"));
        assert!(err
            .message
            .contains("OpenAI ChatCompletions, Anthropic Messages, and OpenAI Responses"));
    }

    #[test]
    fn test_get_message_history_chat_completions() {
        use crate::apis::openai::{Message, MessageContent, Role};

        let chat_req = ChatCompletionsRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: MessageContent::Text("You are helpful".to_string()),
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
                },
            ],
            ..Default::default()
        };

        let provider_req = ProviderRequestType::ChatCompletionsRequest(chat_req);
        let messages = provider_req.get_message_history();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].role, Role::User);
    }

    #[test]
    fn test_get_message_history_anthropic_messages() {
        use crate::apis::anthropic::{
            MessagesMessage, MessagesMessageContent, MessagesRequest, MessagesRole,
            MessagesSystemPrompt,
        };

        let anthropic_req = MessagesRequest {
            model: "claude-3-sonnet".to_string(),
            messages: vec![MessagesMessage {
                role: MessagesRole::User,
                content: MessagesMessageContent::Single("Hello!".to_string()),
            }],
            system: Some(MessagesSystemPrompt::Single(
                "You are helpful".to_string(),
            )),
            max_tokens: 100,
            container: None,
            mcp_servers: None,
            metadata: None,
            service_tier: None,
            thinking: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stream: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
        };

        let provider_req = ProviderRequestType::MessagesRequest(anthropic_req);
        let messages = provider_req.get_message_history();

        // Should have system message + user message
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[0].role,
            crate::apis::openai::Role::System
        );
        assert_eq!(
            messages[1].role,
            crate::apis::openai::Role::User
        );
    }

    #[test]
    fn test_get_message_history_responses_api() {
        use crate::apis::openai_responses::{InputParam, ResponsesAPIRequest};

        let responses_req = ResponsesAPIRequest {
            model: "gpt-4o".to_string(),
            input: InputParam::Text("Hello, world!".to_string()),
            instructions: Some("Be helpful".to_string()),
            temperature: None,
            max_output_tokens: None,
            stream: None,
            metadata: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            modalities: None,
            user: None,
            store: None,
            reasoning_effort: None,
            include: None,
            audio: None,
            text: None,
            service_tier: None,
            top_p: None,
            top_logprobs: None,
            stream_options: None,
            truncation: None,
            conversation: None,
            previous_response_id: None,
            max_tool_calls: None,
            background: None,
        };

        let provider_req = ProviderRequestType::ResponsesAPIRequest(responses_req);
        let messages = provider_req.get_message_history();

        // Should have system message (instructions) + user message (input)
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[0].role,
            crate::apis::openai::Role::System
        );
        assert_eq!(
            messages[1].role,
            crate::apis::openai::Role::User
        );
    }
}
