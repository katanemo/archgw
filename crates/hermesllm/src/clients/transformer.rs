//! API request/response transformers between Anthropic and OpenAI APIs
//!
//! This module provides clean, bidirectional conversion between different LLM API formats
//! using Rust's standard `TryFrom` and `Into` traits. The organization follows a logical flow:
//!
//! 1. **Main Request Transformations** - Core TryFrom implementations for requests
//! 2. **Main Response Transformations** - Core TryFrom implementations for responses
//! 3. **Streaming Transformations** - Bidirectional streaming event conversion
//! 4. **Standard Rust Trait Implementations** - Into/TryFrom implementations for type conversions
//! 5. **Helper Functions** - Utility functions organized by domain
//!
//! # Examples
//!
//! ```rust
//! use hermesllm::apis::{
//!     MessagesRequest, ChatCompletionsRequest, MessagesRole, MessagesMessage,
//!     MessagesMessageContent, MessagesSystemPrompt,
//! };
//! use hermesllm::clients::TransformError;
//! use std::convert::TryInto;
//!
//! // Transform Anthropic to OpenAI
//! let anthropic_req = MessagesRequest {
//!     model: "claude-3-sonnet".to_string(),
//!     system: None,
//!     messages: vec![],
//!     max_tokens: 1024,
//!     container: None,
//!     mcp_servers: None,
//!     service_tier: None,
//!     thinking: None,
//!     temperature: None,
//!     top_p: None,
//!     top_k: None,
//!     stream: None,
//!     stop_sequences: None,
//!     tools: None,
//!     tool_choice: None,
//!     metadata: None,
//! };
//! let openai_req: Result<ChatCompletionsRequest, TransformError> = anthropic_req.try_into();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::apis::*;
use super::TransformError;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default maximum tokens when converting from OpenAI to Anthropic and no max_tokens is specified
const DEFAULT_MAX_TOKENS: u32 = 4096;

// ============================================================================
// UTILITY TRAITS - Shared traits for content manipulation
// ============================================================================

/// Trait for extracting text content from various types
pub trait ExtractText {
    fn extract_text(&self) -> String;
}

/// Trait for utility functions on content collections
trait ContentUtils<T> {
    fn extract_tool_calls(&self) -> Result<Option<Vec<ToolCall>>, TransformError>;
    fn split_for_openai(&self) -> Result<(Vec<ContentPart>, Vec<ToolCall>, Vec<(String, String, bool)>), TransformError>;
}

// ============================================================================
// MAIN REQUEST TRANSFORMATIONS
// ============================================================================

type AnthropicMessagesRequest = MessagesRequest;


impl TryFrom<AnthropicMessagesRequest> for ChatCompletionsRequest {
    type Error = TransformError;

    fn try_from(req: AnthropicMessagesRequest) -> Result<Self, Self::Error> {
        let mut openai_messages: Vec<Message> = Vec::new();

        // Convert system prompt to system message if present
        if let Some(system) = req.system {
            openai_messages.push(system.into());
        }

        // Convert messages
        for message in req.messages {
            let converted_messages: Vec<Message> = message.try_into()?;
            openai_messages.extend(converted_messages);
        }

        // Convert tools and tool choice
        let openai_tools = req.tools.map(|tools| convert_anthropic_tools(tools));
        let (openai_tool_choice, parallel_tool_calls) = convert_anthropic_tool_choice(req.tool_choice);

        let mut _chat_completions_req: ChatCompletionsRequest = ChatCompletionsRequest {
            model: req.model,
            messages: openai_messages,
            temperature: req.temperature,
            top_p: req.top_p,
            max_completion_tokens: Some(req.max_tokens),
            stream: req.stream,
            stop: req.stop_sequences,
            tools: openai_tools,
            tool_choice: openai_tool_choice,
            parallel_tool_calls,
            ..Default::default()
        };
        _chat_completions_req.suppress_max_tokens_if_o3();
        Ok(_chat_completions_req)
    }
}

impl TryFrom<ChatCompletionsRequest> for AnthropicMessagesRequest {
    type Error = TransformError;

    fn try_from(req: ChatCompletionsRequest) -> Result<Self, Self::Error> {
        let mut system_prompt = None;
        let mut messages = Vec::new();

        for message in req.messages {
            match message.role {
                Role::System => {
                    system_prompt = Some(message.into());
                }
                _ => {
                    let anthropic_message: MessagesMessage = message.try_into()?;
                    messages.push(anthropic_message);
                }
            }
        }

        // Convert tools and tool choice
        let anthropic_tools = req.tools.map(|tools| convert_openai_tools(tools));
        let anthropic_tool_choice = convert_openai_tool_choice(req.tool_choice, req.parallel_tool_calls);

        Ok(AnthropicMessagesRequest {
            model: req.model,
            system: system_prompt,
            messages,
            max_tokens: req.max_completion_tokens
                .or(req.max_tokens)
                .unwrap_or(DEFAULT_MAX_TOKENS),
            container: None,
            mcp_servers: None,
            service_tier: None,
            thinking: None,
            temperature: req.temperature,
            top_p: req.top_p,
            top_k: None, // OpenAI doesn't have top_k
            stream: req.stream,
            stop_sequences: req.stop,
            tools: anthropic_tools,
            tool_choice: anthropic_tool_choice,
            metadata: None,
        })
    }
}

// ============================================================================
// MAIN RESPONSE TRANSFORMATIONS
// ============================================================================

impl TryFrom<MessagesResponse> for ChatCompletionsResponse {
    type Error = TransformError;

    fn try_from(resp: MessagesResponse) -> Result<Self, Self::Error> {
        let content = convert_anthropic_content_to_openai(&resp.content)?;
        let finish_reason: FinishReason = resp.stop_reason.into();
        let tool_calls = resp.content.extract_tool_calls()?;

        // Convert MessageContent to String for response
        let content_string = match content {
            MessageContent::Text(text) => Some(text),
            MessageContent::Parts(parts) => {
                let text = parts.extract_text();
                if text.is_empty() { None } else { Some(text) }
            }
        };

        let message = ResponseMessage {
            role: Role::Assistant,
            content: content_string,
            refusal: None,
            annotations: None,
            audio: None,
            function_call: None,
            tool_calls,
        };

        let choice = Choice {
            index: 0,
            message,
            finish_reason: Some(finish_reason),
            logprobs: None,
        };

        let usage = Usage {
            prompt_tokens: resp.usage.input_tokens,
            completion_tokens: resp.usage.output_tokens,
            total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        Ok(ChatCompletionsResponse {
            id: resp.id,
            object: "chat.completion".to_string(),
            created: current_timestamp(),
            model: resp.model,
            choices: vec![choice],
            usage,
            system_fingerprint: None,
            service_tier: None,
        })
    }
}

impl TryFrom<ChatCompletionsResponse> for MessagesResponse {
    type Error = TransformError;

    fn try_from(resp: ChatCompletionsResponse) -> Result<Self, Self::Error> {
        let choice = resp.choices.into_iter().next()
            .ok_or_else(|| TransformError::MissingField("choices".to_string()))?;

        let content = convert_openai_message_to_anthropic_content(&choice.message.to_message())?;
        let stop_reason = choice.finish_reason
            .map(|fr| fr.into())
            .unwrap_or(MessagesStopReason::EndTurn);

        let usage = MessagesUsage {
            input_tokens: resp.usage.prompt_tokens,
            output_tokens: resp.usage.completion_tokens,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };

        Ok(MessagesResponse {
            id: resp.id,
            obj_type: "message".to_string(),
            role: MessagesRole::Assistant,
            content,
            model: resp.model,
            stop_reason,
            stop_sequence: None,
            usage,
            container: None,
        })
    }
}

// ============================================================================
// STREAMING TRANSFORMATIONS
// ============================================================================

impl TryFrom<MessagesStreamEvent> for ChatCompletionsStreamResponse {
    type Error = TransformError;

    fn try_from(event: MessagesStreamEvent) -> Result<Self, Self::Error> {
        match event {
            MessagesStreamEvent::MessageStart { message } => {
                Ok(create_openai_chunk(
                    &message.id,
                    &message.model,
                    MessageDelta {
                        role: Some(Role::Assistant),
                        content: None,
                        refusal: None,
                        function_call: None,
                        tool_calls: None,
                    },
                    None,
                    None,
                ))
            }

            MessagesStreamEvent::ContentBlockStart { content_block, .. } => {
                convert_content_block_start(content_block)
            }

            MessagesStreamEvent::ContentBlockDelta { delta, .. } => {
                convert_content_delta(delta)
            }

            MessagesStreamEvent::ContentBlockStop { .. } => {
                Ok(create_empty_openai_chunk())
            }

            MessagesStreamEvent::MessageDelta { delta, usage } => {
                let finish_reason: Option<FinishReason> = Some(delta.stop_reason.into());
                let openai_usage: Option<Usage> = Some(usage.into());

                Ok(create_openai_chunk(
                    "stream",
                    "unknown",
                    MessageDelta {
                        role: None,
                        content: None,
                        refusal: None,
                        function_call: None,
                        tool_calls: None,
                    },
                    finish_reason,
                    openai_usage,
                ))
            }

            MessagesStreamEvent::MessageStop => {
                Ok(create_openai_chunk(
                    "stream",
                    "unknown",
                    MessageDelta {
                        role: None,
                        content: None,
                        refusal: None,
                        function_call: None,
                        tool_calls: None,
                    },
                    Some(FinishReason::Stop),
                    None,
                ))
            }

            MessagesStreamEvent::Ping => {
                Ok(ChatCompletionsStreamResponse {
                    id: "stream".to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: current_timestamp(),
                    model: "unknown".to_string(),
                    choices: vec![],
                    usage: None,
                    system_fingerprint: None,
                    service_tier: None,
                })
            }
        }
    }
}

impl TryFrom<ChatCompletionsStreamResponse> for MessagesStreamEvent {
    type Error = TransformError;

    fn try_from(resp: ChatCompletionsStreamResponse) -> Result<Self, Self::Error> {
        if resp.choices.is_empty() {
            return Ok(MessagesStreamEvent::Ping);
        }

        let choice = &resp.choices[0];

        // Handle final chunk with usage
        if let Some(usage) = resp.usage {
            if let Some(finish_reason) = &choice.finish_reason {
                let anthropic_stop_reason: MessagesStopReason = finish_reason.clone().into();
                return Ok(MessagesStreamEvent::MessageDelta {
                    delta: MessagesMessageDelta {
                        stop_reason: anthropic_stop_reason,
                        stop_sequence: None,
                    },
                    usage: usage.into(),
                });
            }
        }

        // Handle role start
        if let Some(Role::Assistant) = choice.delta.role {
            return Ok(MessagesStreamEvent::MessageStart {
                message: MessagesStreamMessage {
                    id: resp.id,
                    obj_type: "message".to_string(),
                    role: MessagesRole::Assistant,
                    content: vec![],
                    model: resp.model,
                    stop_reason: None,
                    stop_sequence: None,
                    usage: MessagesUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                    },
                },
            });
        }

        // Handle content delta
        if let Some(content) = &choice.delta.content {
            if !content.is_empty() {
                return Ok(MessagesStreamEvent::ContentBlockDelta {
                    index: 0,
                    delta: MessagesContentDelta::TextDelta {
                        text: content.clone(),
                    },
                });
            }
        }

        // Handle tool calls
        if let Some(tool_calls) = &choice.delta.tool_calls {
            return convert_tool_call_deltas(tool_calls.clone());
        }

        // Handle finish reason
        if let Some(finish_reason) = &choice.finish_reason {
            if *finish_reason == FinishReason::Stop {
                return Ok(MessagesStreamEvent::MessageStop);
            }
        }

        // Default to ping for unhandled cases
        Ok(MessagesStreamEvent::Ping)
    }
}

// ============================================================================
// STANDARD RUST TRAIT IMPLEMENTATIONS - Using Into/TryFrom for conversions
// ============================================================================

// System Prompt Conversions
impl Into<Message> for MessagesSystemPrompt {
    fn into(self) -> Message {
        let system_content = match self {
            MessagesSystemPrompt::Single(text) => MessageContent::Text(text),
            MessagesSystemPrompt::Blocks(blocks) => {
                MessageContent::Text(blocks.extract_text())
            }
        };

        Message {
            role: Role::System,
            content: system_content,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

impl Into<MessagesSystemPrompt> for Message {
    fn into(self) -> MessagesSystemPrompt {
        let system_text = match self.content {
            MessageContent::Text(text) => text,
            MessageContent::Parts(parts) => parts.extract_text()
        };
        MessagesSystemPrompt::Single(system_text)
    }
}

// Message Conversions
impl TryFrom<MessagesMessage> for Vec<Message> {
    type Error = TransformError;

    fn try_from(message: MessagesMessage) -> Result<Self, Self::Error> {
        let mut result = Vec::new();

        match message.content {
            MessagesMessageContent::Single(text) => {
                result.push(Message {
                    role: message.role.into(),
                    content: MessageContent::Text(text),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            MessagesMessageContent::Blocks(blocks) => {
                let (content_parts, tool_calls, tool_results) = blocks.split_for_openai()?;

                // Create main message
                let content = build_openai_content(content_parts, &tool_calls);
                let main_message = Message {
                    role: message.role.into(),
                    content,
                    name: None,
                    tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                    tool_call_id: None,
                };
                result.push(main_message);

                // Add tool result messages
                for (tool_use_id, result_text, _is_error) in tool_results {
                    result.push(Message {
                        role: Role::Tool,
                        content: MessageContent::Text(result_text),
                        name: None,
                        tool_calls: None,
                        tool_call_id: Some(tool_use_id),
                    });
                }
            }
        }

        Ok(result)
    }
}

impl TryFrom<Message> for MessagesMessage {
    type Error = TransformError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let role = match message.role {
            Role::User => MessagesRole::User,
            Role::Assistant => MessagesRole::Assistant,
            Role::Tool => {
                // Tool messages become user messages with tool results
                let tool_call_id = message.tool_call_id
                    .ok_or_else(|| TransformError::MissingField("tool_call_id required for Tool messages".to_string()))?;

                return Ok(MessagesMessage {
                    role: MessagesRole::User,
                    content: MessagesMessageContent::Blocks(vec![
                        MessagesContentBlock::ToolResult {
                            tool_use_id: tool_call_id,
                            is_error: None,
                            content: vec![MessagesContentBlock::Text {
                                text: message.content.extract_text(),
                            }],
                        },
                    ]),
                });
            }
            Role::System => {
                return Err(TransformError::UnsupportedConversion("System messages should be handled separately".to_string()));
            }
        };

        let content_blocks = convert_openai_message_to_anthropic_content(&message)?;
        let content = build_anthropic_content(content_blocks);

        Ok(MessagesMessage { role, content })
    }
}

// Role Conversions
impl Into<Role> for MessagesRole {
    fn into(self) -> Role {
        match self {
            MessagesRole::User => Role::User,
            MessagesRole::Assistant => Role::Assistant,
        }
    }
}

// Content Utilities
impl ContentUtils<ToolCall> for Vec<MessagesContentBlock> {
    fn extract_tool_calls(&self) -> Result<Option<Vec<ToolCall>>, TransformError> {
        let mut tool_calls = Vec::new();

        for block in self {
            match block {
                MessagesContentBlock::ToolUse { id, name, input } |
                MessagesContentBlock::ServerToolUse { id, name, input } |
                MessagesContentBlock::McpToolUse { id, name, input } => {
                    let arguments = serde_json::to_string(&input)?;
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        call_type: "function".to_string(),
                        function: FunctionCall { name: name.clone(), arguments },
                    });
                }
                _ => continue,
            }
        }

        Ok(if tool_calls.is_empty() { None } else { Some(tool_calls) })
    }

    fn split_for_openai(&self) -> Result<(Vec<ContentPart>, Vec<ToolCall>, Vec<(String, String, bool)>), TransformError> {
        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut tool_results = Vec::new();

        for block in self {
            match block {
                MessagesContentBlock::Text { text } => {
                    content_parts.push(ContentPart::Text { text: text.clone() });
                }
                MessagesContentBlock::Image { source } => {
                    let url = convert_image_source_to_url(source);
                    content_parts.push(ContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url,
                            detail: Some("auto".to_string()),
                        },
                    });
                }
                MessagesContentBlock::ToolUse { id, name, input } |
                MessagesContentBlock::ServerToolUse { id, name, input } |
                MessagesContentBlock::McpToolUse { id, name, input } => {
                    let arguments = serde_json::to_string(&input)?;
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        call_type: "function".to_string(),
                        function: FunctionCall { name: name.clone(), arguments },
                    });
                }
                MessagesContentBlock::ToolResult { tool_use_id, content, is_error } |
                MessagesContentBlock::WebSearchToolResult { tool_use_id, content, is_error } |
                MessagesContentBlock::CodeExecutionToolResult { tool_use_id, content, is_error } |
                MessagesContentBlock::McpToolResult { tool_use_id, content, is_error } => {
                    let result_text = content.extract_text();
                    tool_results.push((tool_use_id.clone(), result_text, is_error.unwrap_or(false)));
                }
                _ => {
                    // Skip unsupported content types
                    continue;
                }
            }
        }

        Ok((content_parts, tool_calls, tool_results))
    }
}

// Stop Reason Conversions
impl Into<FinishReason> for MessagesStopReason {
    fn into(self) -> FinishReason {
        match self {
            MessagesStopReason::EndTurn => FinishReason::Stop,
            MessagesStopReason::MaxTokens => FinishReason::Length,
            MessagesStopReason::StopSequence => FinishReason::Stop,
            MessagesStopReason::ToolUse => FinishReason::ToolCalls,
            MessagesStopReason::PauseTurn => FinishReason::Stop,
            MessagesStopReason::Refusal => FinishReason::ContentFilter,
        }
    }
}

impl Into<MessagesStopReason> for FinishReason {
    fn into(self) -> MessagesStopReason {
        match self {
            FinishReason::Stop => MessagesStopReason::EndTurn,
            FinishReason::Length => MessagesStopReason::MaxTokens,
            FinishReason::ToolCalls => MessagesStopReason::ToolUse,
            FinishReason::ContentFilter => MessagesStopReason::Refusal,
            FinishReason::FunctionCall => MessagesStopReason::ToolUse,
        }
    }
}

// Usage Conversions
impl Into<Usage> for MessagesUsage {
    fn into(self) -> Usage {
        Usage {
            prompt_tokens: self.input_tokens,
            completion_tokens: self.output_tokens,
            total_tokens: self.input_tokens + self.output_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }
    }
}

impl Into<MessagesUsage> for Usage {
    fn into(self) -> MessagesUsage {
        MessagesUsage {
            input_tokens: self.prompt_tokens,
            output_tokens: self.completion_tokens,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS - Organized by domain
// ============================================================================

/// Helper to create a current unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Helper to create OpenAI streaming chunk
fn create_openai_chunk(
    id: &str,
    model: &str,
    delta: MessageDelta,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>
) -> ChatCompletionsStreamResponse {
    ChatCompletionsStreamResponse {
        id: id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created: current_timestamp(),
        model: model.to_string(),
        choices: vec![StreamChoice {
            index: 0,
            delta,
            finish_reason,
            logprobs: None,
        }],
        usage,
        system_fingerprint: None,
        service_tier: None,
    }
}

/// Helper to create empty OpenAI streaming chunk
fn create_empty_openai_chunk() -> ChatCompletionsStreamResponse {
    create_openai_chunk(
        "stream",
        "unknown",
        MessageDelta {
            role: None,
            content: None,
            refusal: None,
            function_call: None,
            tool_calls: None,
        },
        None,
        None,
    )
}

/// Convert Anthropic tools to OpenAI format
fn convert_anthropic_tools(tools: Vec<MessagesTool>) -> Vec<Tool> {
    tools.into_iter()
        .map(|tool| Tool {
            tool_type: "function".to_string(),
            function: Function {
                name: tool.name,
                description: tool.description,
                parameters: tool.input_schema,
                strict: None,
            },
        })
        .collect()
}

/// Convert OpenAI tools to Anthropic format
fn convert_openai_tools(tools: Vec<Tool>) -> Vec<MessagesTool> {
    tools.into_iter()
        .map(|tool| MessagesTool {
            name: tool.function.name,
            description: tool.function.description,
            input_schema: tool.function.parameters,
        })
        .collect()
}

/// Convert Anthropic tool choice to OpenAI format
fn convert_anthropic_tool_choice(tool_choice: Option<MessagesToolChoice>) -> (Option<ToolChoice>, Option<bool>) {
    match tool_choice {
        Some(choice) => {
            let openai_choice = match choice.kind {
                MessagesToolChoiceType::Auto => ToolChoice::Type(ToolChoiceType::Auto),
                MessagesToolChoiceType::Any => ToolChoice::Type(ToolChoiceType::Required),
                MessagesToolChoiceType::None => ToolChoice::Type(ToolChoiceType::None),
                MessagesToolChoiceType::Tool => {
                    if let Some(name) = choice.name {
                        ToolChoice::Function {
                            choice_type: "function".to_string(),
                            function: FunctionChoice { name },
                        }
                    } else {
                        ToolChoice::Type(ToolChoiceType::Auto)
                    }
                }
            };
            let parallel = choice.disable_parallel_tool_use.map(|disable| !disable);
            (Some(openai_choice), parallel)
        }
        None => (None, None)
    }
}

/// Convert OpenAI tool choice to Anthropic format
fn convert_openai_tool_choice(
    tool_choice: Option<ToolChoice>,
    parallel_tool_calls: Option<bool>
) -> Option<MessagesToolChoice> {
    tool_choice.map(|choice| {
        match choice {
            ToolChoice::Type(tool_type) => match tool_type {
                ToolChoiceType::Auto => MessagesToolChoice {
                    kind: MessagesToolChoiceType::Auto,
                    name: None,
                    disable_parallel_tool_use: parallel_tool_calls.map(|p| !p),
                },
                ToolChoiceType::Required => MessagesToolChoice {
                    kind: MessagesToolChoiceType::Any,
                    name: None,
                    disable_parallel_tool_use: parallel_tool_calls.map(|p| !p),
                },
                ToolChoiceType::None => MessagesToolChoice {
                    kind: MessagesToolChoiceType::None,
                    name: None,
                    disable_parallel_tool_use: None,
                },
            },
            ToolChoice::Function { function, .. } => MessagesToolChoice {
                kind: MessagesToolChoiceType::Tool,
                name: Some(function.name),
                disable_parallel_tool_use: parallel_tool_calls.map(|p| !p),
            },
        }
    })
}

/// Build OpenAI message content from parts and tool calls
fn build_openai_content(content_parts: Vec<ContentPart>, tool_calls: &[ToolCall]) -> MessageContent {
    if content_parts.len() == 1 && tool_calls.is_empty() {
        match &content_parts[0] {
            ContentPart::Text { text } => MessageContent::Text(text.clone()),
            _ => MessageContent::Parts(content_parts),
        }
    } else if content_parts.is_empty() {
        MessageContent::Text("".to_string())
    } else {
        MessageContent::Parts(content_parts)
    }
}

/// Build Anthropic message content from content blocks
fn build_anthropic_content(content_blocks: Vec<MessagesContentBlock>) -> MessagesMessageContent {
    if content_blocks.len() == 1 {
        match &content_blocks[0] {
            MessagesContentBlock::Text { text } => MessagesMessageContent::Single(text.clone()),
            _ => MessagesMessageContent::Blocks(content_blocks),
        }
    } else if content_blocks.is_empty() {
        MessagesMessageContent::Single("".to_string())
    } else {
        MessagesMessageContent::Blocks(content_blocks)
    }
}

/// Convert Anthropic content blocks to OpenAI message content
fn convert_anthropic_content_to_openai(content: &[MessagesContentBlock]) -> Result<MessageContent, TransformError> {
    let mut text_parts = Vec::new();

    for block in content {
        match block {
            MessagesContentBlock::Text { text } => {
                text_parts.push(text.clone());
            }
            MessagesContentBlock::Thinking { text } => {
                // Include thinking as regular text for OpenAI
                text_parts.push(format!("[Thinking: {}]", text));
            }
            _ => {
                // Skip other content types for basic text conversion
                continue;
            }
        }
    }

    Ok(MessageContent::Text(text_parts.join("\n")))
}

/// Convert OpenAI message to Anthropic content blocks
fn convert_openai_message_to_anthropic_content(message: &Message) -> Result<Vec<MessagesContentBlock>, TransformError> {
    let mut blocks = Vec::new();

    // Handle regular content
    match &message.content {
        MessageContent::Text(text) => {
            if !text.is_empty() {
                blocks.push(MessagesContentBlock::Text { text: text.clone() });
            }
        }
        MessageContent::Parts(parts) => {
            for part in parts {
                match part {
                    ContentPart::Text { text } => {
                        blocks.push(MessagesContentBlock::Text { text: text.clone() });
                    }
                    ContentPart::ImageUrl { image_url } => {
                        let source = convert_image_url_to_source(image_url);
                        blocks.push(MessagesContentBlock::Image { source });
                    }
                }
            }
        }
    }

    // Handle tool calls
    if let Some(tool_calls) = &message.tool_calls {
        for tool_call in tool_calls {
            let input: Value = serde_json::from_str(&tool_call.function.arguments)?;
            blocks.push(MessagesContentBlock::ToolUse {
                id: tool_call.id.clone(),
                name: tool_call.function.name.clone(),
                input,
            });
        }
    }

    Ok(blocks)
}

/// Convert image source to URL
fn convert_image_source_to_url(source: &MessagesImageSource) -> String {
    match source {
        MessagesImageSource::Base64 { media_type, data } => {
            format!("data:{};base64,{}", media_type, data)
        }
        MessagesImageSource::Url { url } => url.clone(),
    }
}

/// Convert image URL to Anthropic image source
fn convert_image_url_to_source(image_url: &ImageUrl) -> MessagesImageSource {
    if image_url.url.starts_with("data:") {
        // Parse data URL
        let parts: Vec<&str> = image_url.url.splitn(2, ',').collect();
        if parts.len() == 2 {
            let header = parts[0];
            let data = parts[1];
            let media_type = header
                .strip_prefix("data:")
                .and_then(|s| s.split(';').next())
                .unwrap_or("image/jpeg")
                .to_string();

            MessagesImageSource::Base64 {
                media_type,
                data: data.to_string(),
            }
        } else {
            MessagesImageSource::Url { url: image_url.url.clone() }
        }
    } else {
        MessagesImageSource::Url { url: image_url.url.clone() }
    }
}

/// Convert content block start to OpenAI chunk
fn convert_content_block_start(content_block: MessagesContentBlock) -> Result<ChatCompletionsStreamResponse, TransformError> {
    match content_block {
        MessagesContentBlock::Text { .. } => {
            // No immediate output for text block start
            Ok(create_empty_openai_chunk())
        }
        MessagesContentBlock::ToolUse { id, name, .. } |
        MessagesContentBlock::ServerToolUse { id, name, .. } |
        MessagesContentBlock::McpToolUse { id, name, .. } => {
            // Tool use start → OpenAI chunk with tool_calls
            Ok(create_openai_chunk(
                "stream",
                "unknown",
                MessageDelta {
                    role: None,
                    content: None,
                    refusal: None,
                    function_call: None,
                    tool_calls: Some(vec![ToolCallDelta {
                        index: 0,
                        id: Some(id),
                        call_type: Some("function".to_string()),
                        function: Some(FunctionCallDelta {
                            name: Some(name),
                            arguments: Some("".to_string()),
                        }),
                    }]),
                },
                None,
                None,
            ))
        }
        _ => Err(TransformError::UnsupportedContent("Unsupported content block type in stream start".to_string())),
    }
}

/// Convert content delta to OpenAI chunk
fn convert_content_delta(delta: MessagesContentDelta) -> Result<ChatCompletionsStreamResponse, TransformError> {
    match delta {
        MessagesContentDelta::TextDelta { text } => {
            Ok(create_openai_chunk(
                "stream",
                "unknown",
                MessageDelta {
                    role: None,
                    content: Some(text),
                    refusal: None,
                    function_call: None,
                    tool_calls: None,
                },
                None,
                None,
            ))
        }
        MessagesContentDelta::InputJsonDelta { partial_json } => {
            Ok(create_openai_chunk(
                "stream",
                "unknown",
                MessageDelta {
                    role: None,
                    content: None,
                    refusal: None,
                    function_call: None,
                    tool_calls: Some(vec![ToolCallDelta {
                        index: 0,
                        id: None,
                        call_type: None,
                        function: Some(FunctionCallDelta {
                            name: None,
                            arguments: Some(partial_json),
                        }),
                    }]),
                },
                None,
                None,
            ))
        }
    }
}

/// Convert tool call deltas to Anthropic stream events
fn convert_tool_call_deltas(tool_calls: Vec<ToolCallDelta>) -> Result<MessagesStreamEvent, TransformError> {
    for tool_call in tool_calls {
        if let Some(id) = &tool_call.id {
            // Tool call start
            if let Some(function) = &tool_call.function {
                if let Some(name) = &function.name {
                    return Ok(MessagesStreamEvent::ContentBlockStart {
                        index: tool_call.index,
                        content_block: MessagesContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: Value::Object(serde_json::Map::new()),
                        },
                    });
                }
            }
        } else if let Some(function) = &tool_call.function {
            if let Some(arguments) = &function.arguments {
                // Tool arguments delta
                return Ok(MessagesStreamEvent::ContentBlockDelta {
                    index: tool_call.index,
                    delta: MessagesContentDelta::InputJsonDelta {
                        partial_json: arguments.clone(),
                    },
                });
            }
        }
    }

    // Fallback to ping if no valid tool call found
    Ok(MessagesStreamEvent::Ping)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_anthropic_to_openai_basic_request() {
        let anthropic_req = AnthropicMessagesRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            system: Some(MessagesSystemPrompt::Single("You are helpful".to_string())),
            messages: vec![MessagesMessage {
                role: MessagesRole::User,
                content: MessagesMessageContent::Single("Hello, world!".to_string()),
            }],
            max_tokens: 1024,
            container: None,
            mcp_servers: None,
            service_tier: None,
            thinking: None,
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(50),
            stream: Some(false),
            stop_sequences: Some(vec!["STOP".to_string()]),
            tools: None,
            tool_choice: None,
            metadata: None,
        };

        let openai_req: ChatCompletionsRequest = anthropic_req.try_into().unwrap();

        assert_eq!(openai_req.model, "claude-3-sonnet-20240229");
        assert_eq!(openai_req.messages.len(), 2); // system + user message
        assert_eq!(openai_req.max_completion_tokens, Some(1024));
        assert_eq!(openai_req.temperature, Some(0.7));
        assert_eq!(openai_req.top_p, Some(0.9));
        assert_eq!(openai_req.stream, Some(false));
        assert_eq!(openai_req.stop, Some(vec!["STOP".to_string()]));
    }

    #[test]
    fn test_roundtrip_consistency() {
        // Test that converting back and forth maintains consistency
        let original_anthropic = AnthropicMessagesRequest {
            model: "claude-3-sonnet".to_string(),
            system: Some(MessagesSystemPrompt::Single("System prompt".to_string())),
            messages: vec![MessagesMessage {
                role: MessagesRole::User,
                content: MessagesMessageContent::Single("User message".to_string()),
            }],
            max_tokens: 1000,
            container: None,
            mcp_servers: None,
            service_tier: None,
            thinking: None,
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            stream: Some(false),
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            metadata: None,
        };

        // Convert to OpenAI and back
        let openai_req: ChatCompletionsRequest = original_anthropic.clone().try_into().unwrap();
        let roundtrip_anthropic: AnthropicMessagesRequest = openai_req.try_into().unwrap();

        // Check key fields are preserved
        assert_eq!(original_anthropic.model, roundtrip_anthropic.model);
        assert_eq!(original_anthropic.max_tokens, roundtrip_anthropic.max_tokens);
        assert_eq!(original_anthropic.temperature, roundtrip_anthropic.temperature);
        assert_eq!(original_anthropic.top_p, roundtrip_anthropic.top_p);
        assert_eq!(original_anthropic.stream, roundtrip_anthropic.stream);
        assert_eq!(original_anthropic.messages.len(), roundtrip_anthropic.messages.len());
    }

    #[test]
    fn test_tool_choice_auto() {
        let anthropic_req = AnthropicMessagesRequest {
            model: "claude-3".to_string(),
            system: None,
            messages: vec![],
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
            tools: Some(vec![MessagesTool {
                name: "test_tool".to_string(),
                description: Some("A test tool".to_string()),
                input_schema: json!({"type": "object"}),
            }]),
            tool_choice: Some(MessagesToolChoice {
                kind: MessagesToolChoiceType::Auto,
                name: None,
                disable_parallel_tool_use: Some(true),
            }),
            metadata: None,
        };

        let openai_req: ChatCompletionsRequest = anthropic_req.try_into().unwrap();

        assert!(openai_req.tools.is_some());
        assert_eq!(openai_req.tools.as_ref().unwrap().len(), 1);

        if let Some(ToolChoice::Type(choice)) = openai_req.tool_choice {
            assert_eq!(choice, ToolChoiceType::Auto);
        } else {
            panic!("Expected auto tool choice");
        }

        assert_eq!(openai_req.parallel_tool_calls, Some(false));
    }

    #[test]
    fn test_default_max_tokens_used_when_openai_has_none() {
        // Test that DEFAULT_MAX_TOKENS is used when OpenAI request has no max_tokens
        let openai_req = ChatCompletionsRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: None, // No max_tokens specified
            ..Default::default()
        };

        let anthropic_req: AnthropicMessagesRequest = openai_req.try_into().unwrap();

        assert_eq!(anthropic_req.max_tokens, DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_anthropic_message_start_streaming() {
        let event = MessagesStreamEvent::MessageStart {
            message: MessagesStreamMessage {
                id: "msg_stream_123".to_string(),
                obj_type: "message".to_string(),
                role: MessagesRole::Assistant,
                content: vec![],
                model: "claude-3".to_string(),
                stop_reason: None,
                stop_sequence: None,
                usage: MessagesUsage {
                    input_tokens: 5,
                    output_tokens: 0,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                },
            },
        };

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        assert_eq!(openai_resp.id, "msg_stream_123");
        assert_eq!(openai_resp.object, "chat.completion.chunk");
        assert_eq!(openai_resp.model, "claude-3");
        assert_eq!(openai_resp.choices.len(), 1);

        let choice = &openai_resp.choices[0];
        assert_eq!(choice.index, 0);
        assert_eq!(choice.delta.role, Some(Role::Assistant));
        assert_eq!(choice.delta.content, None);
        assert_eq!(choice.finish_reason, None);
    }

    #[test]
    fn test_anthropic_content_block_delta_streaming() {
        let event = MessagesStreamEvent::ContentBlockDelta {
            index: 0,
            delta: MessagesContentDelta::TextDelta {
                text: "Hello, world!".to_string(),
            },
        };

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        assert_eq!(openai_resp.object, "chat.completion.chunk");
        assert_eq!(openai_resp.choices.len(), 1);

        let choice = &openai_resp.choices[0];
        assert_eq!(choice.index, 0);
        assert_eq!(choice.delta.content, Some("Hello, world!".to_string()));
        assert_eq!(choice.delta.role, None);
        assert_eq!(choice.finish_reason, None);
    }

    #[test]
    fn test_anthropic_tool_use_streaming() {
        // Test tool use start
        let tool_start = MessagesStreamEvent::ContentBlockStart {
            index: 0,
            content_block: MessagesContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                input: json!({}),
            },
        };

        let openai_resp: ChatCompletionsStreamResponse = tool_start.try_into().unwrap();

        assert_eq!(openai_resp.choices.len(), 1);
        let choice = &openai_resp.choices[0];
        assert!(choice.delta.tool_calls.is_some());

        let tool_calls = choice.delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, Some("call_123".to_string()));
        assert_eq!(tool_calls[0].function.as_ref().unwrap().name, Some("get_weather".to_string()));
    }

    #[test]
    fn test_anthropic_tool_input_delta_streaming() {
        let event = MessagesStreamEvent::ContentBlockDelta {
            index: 0,
            delta: MessagesContentDelta::InputJsonDelta {
                partial_json: r#"{"location": "San Francisco"#.to_string(),
            },
        };

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        assert_eq!(openai_resp.choices.len(), 1);
        let choice = &openai_resp.choices[0];
        assert!(choice.delta.tool_calls.is_some());

        let tool_calls = choice.delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.as_ref().unwrap().arguments, Some(r#"{"location": "San Francisco"#.to_string()));
    }

    #[test]
    fn test_anthropic_message_delta_with_usage() {
        let event = MessagesStreamEvent::MessageDelta {
            delta: MessagesMessageDelta {
                stop_reason: MessagesStopReason::EndTurn,
                stop_sequence: None,
            },
            usage: MessagesUsage {
                input_tokens: 10,
                output_tokens: 25,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            },
        };

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        assert_eq!(openai_resp.choices.len(), 1);
        let choice = &openai_resp.choices[0];
        assert_eq!(choice.finish_reason, Some(FinishReason::Stop));

        assert!(openai_resp.usage.is_some());
        let usage = openai_resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 25);
        assert_eq!(usage.total_tokens, 35);
    }

    #[test]
    fn test_anthropic_message_stop_streaming() {
        let event = MessagesStreamEvent::MessageStop;

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        assert_eq!(openai_resp.choices.len(), 1);
        let choice = &openai_resp.choices[0];
        assert_eq!(choice.finish_reason, Some(FinishReason::Stop));
    }

    #[test]
    fn test_anthropic_ping_streaming() {
        let event = MessagesStreamEvent::Ping;

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        assert_eq!(openai_resp.object, "chat.completion.chunk");
        assert_eq!(openai_resp.choices.len(), 0); // Ping has no choices
    }

    #[test]
    fn test_openai_to_anthropic_streaming_role_start() {
        let openai_resp = ChatCompletionsStreamResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: MessageDelta {
                    role: Some(Role::Assistant),
                    content: None,
                    refusal: None,
                    function_call: None,
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        };

        let anthropic_event: MessagesStreamEvent = openai_resp.try_into().unwrap();

        match anthropic_event {
            MessagesStreamEvent::MessageStart { message } => {
                assert_eq!(message.id, "chatcmpl-123");
                assert_eq!(message.role, MessagesRole::Assistant);
                assert_eq!(message.model, "gpt-4");
            }
            _ => panic!("Expected MessageStart event"),
        }
    }

    #[test]
    fn test_openai_to_anthropic_streaming_content_delta() {
        let openai_resp = ChatCompletionsStreamResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: MessageDelta {
                    role: None,
                    content: Some("Hello there!".to_string()),
                    refusal: None,
                    function_call: None,
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        };

        let anthropic_event: MessagesStreamEvent = openai_resp.try_into().unwrap();

        match anthropic_event {
            MessagesStreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    MessagesContentDelta::TextDelta { text } => {
                        assert_eq!(text, "Hello there!");
                    }
                    _ => panic!("Expected TextDelta"),
                }
            }
            _ => panic!("Expected ContentBlockDelta event"),
        }
    }

    #[test]
    fn test_openai_to_anthropic_streaming_tool_calls() {
        let openai_resp = ChatCompletionsStreamResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: MessageDelta {
                    role: None,
                    content: None,
                    refusal: None,
                    function_call: None,
                    tool_calls: Some(vec![ToolCallDelta {
                        index: 0,
                        id: Some("call_abc123".to_string()),
                        call_type: Some("function".to_string()),
                        function: Some(FunctionCallDelta {
                            name: Some("get_current_weather".to_string()),
                            arguments: Some("".to_string()),
                        }),
                    }]),
                },
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        };

        let anthropic_event: MessagesStreamEvent = openai_resp.try_into().unwrap();

        match anthropic_event {
            MessagesStreamEvent::ContentBlockStart { index, content_block } => {
                assert_eq!(index, 0);
                match content_block {
                    MessagesContentBlock::ToolUse { id, name, .. } => {
                        assert_eq!(id, "call_abc123");
                        assert_eq!(name, "get_current_weather");
                    }
                    _ => panic!("Expected ToolUse content block"),
                }
            }
            _ => panic!("Expected ContentBlockStart event"),
        }
    }

    #[test]
    fn test_openai_to_anthropic_streaming_final_usage() {
        let openai_resp = ChatCompletionsStreamResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: MessageDelta {
                    role: None,
                    content: None,
                    refusal: None,
                    function_call: None,
                    tool_calls: None,
                },
                finish_reason: Some(FinishReason::Stop),
                logprobs: None,
            }],
            usage: Some(Usage {
                prompt_tokens: 15,
                completion_tokens: 30,
                total_tokens: 45,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            }),
            system_fingerprint: None,
            service_tier: None,
        };

        let anthropic_event: MessagesStreamEvent = openai_resp.try_into().unwrap();

        match anthropic_event {
            MessagesStreamEvent::MessageDelta { delta, usage } => {
                assert_eq!(delta.stop_reason, MessagesStopReason::EndTurn);
                assert_eq!(usage.input_tokens, 15);
                assert_eq!(usage.output_tokens, 30);
            }
            _ => panic!("Expected MessageDelta event"),
        }
    }

    #[test]
    fn test_openai_empty_choices_to_anthropic_ping() {
        let openai_resp = ChatCompletionsStreamResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![], // Empty choices
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        };

        let anthropic_event: MessagesStreamEvent = openai_resp.try_into().unwrap();

        match anthropic_event {
            MessagesStreamEvent::Ping => {
                // Expected behavior
            }
            _ => panic!("Expected Ping event for empty choices"),
        }
    }

    #[test]
    fn test_streaming_roundtrip_consistency() {
        // Test that streaming events can roundtrip through conversions
        let original_event = MessagesStreamEvent::ContentBlockDelta {
            index: 0,
            delta: MessagesContentDelta::TextDelta {
                text: "Test message".to_string(),
            },
        };

        // Convert to OpenAI and back
        let openai_resp: ChatCompletionsStreamResponse = original_event.try_into().unwrap();
        let roundtrip_event: MessagesStreamEvent = openai_resp.try_into().unwrap();

        // Verify the roundtrip maintains the essential information
        match roundtrip_event {
            MessagesStreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    MessagesContentDelta::TextDelta { text } => {
                        assert_eq!(text, "Test message");
                    }
                    _ => panic!("Expected TextDelta after roundtrip"),
                }
            }
            _ => panic!("Expected ContentBlockDelta after roundtrip"),
        }
    }

    #[test]
    fn test_streaming_tool_argument_accumulation() {
        // Test multiple tool argument deltas that should accumulate
        let tool_start = MessagesStreamEvent::ContentBlockStart {
            index: 0,
            content_block: MessagesContentBlock::ToolUse {
                id: "call_weather".to_string(),
                name: "get_weather".to_string(),
                input: json!({}),
            },
        };

        let arg_delta1 = MessagesStreamEvent::ContentBlockDelta {
            index: 0,
            delta: MessagesContentDelta::InputJsonDelta {
                partial_json: r#"{"location": "#.to_string(),
            },
        };

        let arg_delta2 = MessagesStreamEvent::ContentBlockDelta {
            index: 0,
            delta: MessagesContentDelta::InputJsonDelta {
                partial_json: r#"San Francisco", "unit": "fahrenheit"}"#.to_string(),
            },
        };

        // Test that each delta converts properly to OpenAI format
        let openai_start: ChatCompletionsStreamResponse = tool_start.try_into().unwrap();
        let openai_delta1: ChatCompletionsStreamResponse = arg_delta1.try_into().unwrap();
        let openai_delta2: ChatCompletionsStreamResponse = arg_delta2.try_into().unwrap();

        // Verify tool start
        let tool_calls = &openai_start.choices[0].delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls[0].id, Some("call_weather".to_string()));
        assert_eq!(tool_calls[0].function.as_ref().unwrap().name, Some("get_weather".to_string()));

        // Verify argument deltas
        let args1 = &openai_delta1.choices[0].delta.tool_calls.as_ref().unwrap()[0]
            .function.as_ref().unwrap().arguments;
        assert_eq!(args1, &Some(r#"{"location": "#.to_string()));

        let args2 = &openai_delta2.choices[0].delta.tool_calls.as_ref().unwrap()[0]
            .function.as_ref().unwrap().arguments;
        assert_eq!(args2, &Some(r#"San Francisco", "unit": "fahrenheit"}"#.to_string()));
    }

    #[test]
    fn test_streaming_multiple_finish_reasons() {
        // Test different finish reasons in streaming
        let test_cases = vec![
            (MessagesStopReason::EndTurn, FinishReason::Stop),
            (MessagesStopReason::MaxTokens, FinishReason::Length),
            (MessagesStopReason::ToolUse, FinishReason::ToolCalls),
            (MessagesStopReason::StopSequence, FinishReason::Stop),
        ];

        for (anthropic_reason, expected_openai_reason) in test_cases {
            let event = MessagesStreamEvent::MessageDelta {
                delta: MessagesMessageDelta {
                    stop_reason: anthropic_reason.clone(),
                    stop_sequence: None,
                },
                usage: MessagesUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                },
            };

            let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();
            assert_eq!(openai_resp.choices[0].finish_reason, Some(expected_openai_reason));

            // Test reverse conversion
            let roundtrip_event: MessagesStreamEvent = openai_resp.try_into().unwrap();
            match roundtrip_event {
                MessagesStreamEvent::MessageDelta { delta, .. } => {
                    // Note: Some precision may be lost in roundtrip due to mapping differences
                    assert!(matches!(delta.stop_reason, MessagesStopReason::EndTurn | MessagesStopReason::MaxTokens | MessagesStopReason::ToolUse | MessagesStopReason::StopSequence));
                }
                _ => panic!("Expected MessageDelta after roundtrip"),
            }
        }
    }

    #[test]
    fn test_streaming_error_handling() {
        // Test that malformed streaming events are handled gracefully
        let openai_resp_with_missing_data = ChatCompletionsStreamResponse {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "test".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: MessageDelta {
                    role: None,
                    content: None,
                    refusal: None,
                    function_call: None,
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
            system_fingerprint: None,
            service_tier: None,
        };

        // Should convert to Ping when no meaningful content
        let anthropic_event: MessagesStreamEvent = openai_resp_with_missing_data.try_into().unwrap();
        assert!(matches!(anthropic_event, MessagesStreamEvent::Ping));
    }

    #[test]
    fn test_streaming_content_block_stop() {
        let event = MessagesStreamEvent::ContentBlockStop { index: 0 };

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        // ContentBlockStop should produce an empty chunk
        assert_eq!(openai_resp.object, "chat.completion.chunk");
        assert_eq!(openai_resp.choices.len(), 1);

        let choice = &openai_resp.choices[0];
        assert_eq!(choice.delta.role, None);
        assert_eq!(choice.delta.content, None);
        assert_eq!(choice.delta.tool_calls, None);
        assert_eq!(choice.finish_reason, None);
    }
}
