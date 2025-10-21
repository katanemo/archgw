use crate::apis::amazon_bedrock::{
    ConverseOutput, ConverseResponse, ConverseStreamEvent, StopReason,
};
use crate::apis::anthropic::{
    MessagesContentBlock, MessagesContentDelta, MessagesResponse, MessagesStopReason,
    MessagesStreamEvent, MessagesUsage,
};
use crate::apis::openai::{
    ChatCompletionsResponse, ChatCompletionsStreamResponse, Choice, FinishReason,
    FunctionCallDelta, MessageContent, MessageDelta, ResponseMessage, Role, StreamChoice,
    ToolCallDelta, Usage,
};
use crate::clients::TransformError;
use crate::transforms::lib::*;

// ============================================================================
// MAIN RESPONSE TRANSFORMATIONS
// ============================================================================

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
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
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
            object: Some("chat.completion".to_string()),
            created: current_timestamp(),
            model: resp.model,
            choices: vec![choice],
            usage,
            system_fingerprint: None,
            service_tier: None,
        })
    }
}

impl TryFrom<ConverseResponse> for ChatCompletionsResponse {
    type Error = TransformError;

    fn try_from(resp: ConverseResponse) -> Result<Self, Self::Error> {
        // Extract the message from the ConverseOutput
        let message = match resp.output {
            ConverseOutput::Message { message } => message,
        };

        // Convert Bedrock ConversationRole to OpenAI Role
        let role = match message.role {
            crate::apis::amazon_bedrock::ConversationRole::User => Role::User,
            crate::apis::amazon_bedrock::ConversationRole::Assistant => Role::Assistant,
        };

        // Convert Bedrock message content to OpenAI format
        let (content, tool_calls) = convert_bedrock_message_to_openai(&message)?;

        // Convert Bedrock stop reason to OpenAI finish reason
        let finish_reason = match resp.stop_reason {
            StopReason::EndTurn => FinishReason::Stop,
            StopReason::ToolUse => FinishReason::ToolCalls,
            StopReason::MaxTokens => FinishReason::Length,
            StopReason::StopSequence => FinishReason::Stop,
            StopReason::GuardrailIntervened => FinishReason::ContentFilter,
            StopReason::ContentFiltered => FinishReason::ContentFilter,
        };

        // Create response message
        let response_message = ResponseMessage {
            role,
            content,
            refusal: None,
            annotations: None,
            audio: None,
            function_call: None,
            tool_calls,
        };

        // Create choice
        let choice = Choice {
            index: 0,
            message: response_message,
            finish_reason: Some(finish_reason),
            logprobs: None,
        };

        // Convert token usage
        let usage = Usage {
            prompt_tokens: resp.usage.input_tokens,
            completion_tokens: resp.usage.output_tokens,
            total_tokens: resp.usage.total_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        // Generate a response ID (using timestamp since Bedrock doesn't provide one)
        let id = format!(
            "bedrock-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        // Extract model ID from trace information if available, otherwise use fallback
        let model = resp
            .trace
            .as_ref()
            .and_then(|trace| trace.prompt_router.as_ref())
            .map(|router| router.invoked_model_id.clone())
            .unwrap_or_else(|| "bedrock-model".to_string());

        Ok(ChatCompletionsResponse {
            id,
            object: Some("chat.completion".to_string()),
            created: current_timestamp(),
            model,
            choices: vec![choice],
            usage,
            system_fingerprint: None,
            service_tier: None,
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
            MessagesStreamEvent::MessageStart { message } => Ok(create_openai_chunk(
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
            )),

            MessagesStreamEvent::ContentBlockStart { content_block, .. } => {
                convert_content_block_start(content_block)
            }

            MessagesStreamEvent::ContentBlockDelta { delta, .. } => convert_content_delta(delta),

            MessagesStreamEvent::ContentBlockStop { .. } => Ok(create_empty_openai_chunk()),

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

            MessagesStreamEvent::MessageStop => Ok(create_openai_chunk(
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
            )),

            MessagesStreamEvent::Ping => Ok(ChatCompletionsStreamResponse {
                id: "stream".to_string(),
                object: Some("chat.completion.chunk".to_string()),
                created: current_timestamp(),
                model: "unknown".to_string(),
                choices: vec![],
                usage: None,
                system_fingerprint: None,
                service_tier: None,
            }),
        }
    }
}

impl TryFrom<ConverseStreamEvent> for ChatCompletionsStreamResponse {
    type Error = TransformError;

    fn try_from(event: ConverseStreamEvent) -> Result<Self, Self::Error> {
        match event {
            ConverseStreamEvent::MessageStart(start_event) => {
                let role = match start_event.role {
                    crate::apis::amazon_bedrock::ConversationRole::User => Role::User,
                    crate::apis::amazon_bedrock::ConversationRole::Assistant => Role::Assistant,
                };

                Ok(create_openai_chunk(
                    "stream",
                    "unknown",
                    MessageDelta {
                        role: Some(role),
                        content: None,
                        refusal: None,
                        function_call: None,
                        tool_calls: None,
                    },
                    None,
                    None,
                ))
            }

            ConverseStreamEvent::ContentBlockStart(start_event) => {
                use crate::apis::amazon_bedrock::ContentBlockStart;

                match start_event.start {
                    ContentBlockStart::ToolUse { tool_use } => Ok(create_openai_chunk(
                        "stream",
                        "unknown",
                        MessageDelta {
                            role: None,
                            content: None,
                            refusal: None,
                            function_call: None,
                            tool_calls: Some(vec![ToolCallDelta {
                                index: start_event.content_block_index as u32,
                                id: Some(tool_use.tool_use_id),
                                call_type: Some("function".to_string()),
                                function: Some(FunctionCallDelta {
                                    name: Some(tool_use.name),
                                    arguments: Some("".to_string()),
                                }),
                            }]),
                        },
                        None,
                        None,
                    )),
                }
            }

            ConverseStreamEvent::ContentBlockDelta(delta_event) => {
                use crate::apis::amazon_bedrock::ContentBlockDelta;

                match delta_event.delta {
                    ContentBlockDelta::Text { text } => Ok(create_openai_chunk(
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
                    )),
                    ContentBlockDelta::ToolUse { tool_use } => Ok(create_openai_chunk(
                        "stream",
                        "unknown",
                        MessageDelta {
                            role: None,
                            content: None,
                            refusal: None,
                            function_call: None,
                            tool_calls: Some(vec![ToolCallDelta {
                                index: delta_event.content_block_index as u32,
                                id: None,
                                call_type: None,
                                function: Some(FunctionCallDelta {
                                    name: None,
                                    arguments: Some(tool_use.input),
                                }),
                            }]),
                        },
                        None,
                        None,
                    )),
                }
            }

            ConverseStreamEvent::ContentBlockStop(_) => Ok(create_empty_openai_chunk()),

            ConverseStreamEvent::MessageStop(stop_event) => {
                let finish_reason = match stop_event.stop_reason {
                    StopReason::EndTurn => FinishReason::Stop,
                    StopReason::ToolUse => FinishReason::ToolCalls,
                    StopReason::MaxTokens => FinishReason::Length,
                    StopReason::StopSequence => FinishReason::Stop,
                    StopReason::GuardrailIntervened => FinishReason::ContentFilter,
                    StopReason::ContentFiltered => FinishReason::ContentFilter,
                };

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
                    Some(finish_reason),
                    None,
                ))
            }

            ConverseStreamEvent::Metadata(metadata_event) => {
                let usage = Usage {
                    prompt_tokens: metadata_event.usage.input_tokens,
                    completion_tokens: metadata_event.usage.output_tokens,
                    total_tokens: metadata_event.usage.total_tokens,
                    prompt_tokens_details: None,
                    completion_tokens_details: None,
                };

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
                    None,
                    Some(usage),
                ))
            }

            // Error events - convert to empty chunks (errors should be handled elsewhere)
            ConverseStreamEvent::InternalServerException(_)
            | ConverseStreamEvent::ModelStreamErrorException(_)
            | ConverseStreamEvent::ServiceUnavailableException(_)
            | ConverseStreamEvent::ThrottlingException(_)
            | ConverseStreamEvent::ValidationException(_) => Ok(create_empty_openai_chunk()),
        }
    }
}

/// Convert content block start to OpenAI chunk
fn convert_content_block_start(
    content_block: MessagesContentBlock,
) -> Result<ChatCompletionsStreamResponse, TransformError> {
    match content_block {
        MessagesContentBlock::Text { .. } => {
            // No immediate output for text block start
            Ok(create_empty_openai_chunk())
        }
        MessagesContentBlock::ToolUse { id, name, .. }
        | MessagesContentBlock::ServerToolUse { id, name, .. }
        | MessagesContentBlock::McpToolUse { id, name, .. } => {
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
        _ => Err(TransformError::UnsupportedContent(
            "Unsupported content block type in stream start".to_string(),
        )),
    }
}

/// Convert content delta to OpenAI chunk
fn convert_content_delta(
    delta: MessagesContentDelta,
) -> Result<ChatCompletionsStreamResponse, TransformError> {
    match delta {
        MessagesContentDelta::TextDelta { text } => Ok(create_openai_chunk(
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
        )),
        MessagesContentDelta::ThinkingDelta { thinking } => Ok(create_openai_chunk(
            "stream",
            "unknown",
            MessageDelta {
                role: None,
                content: Some(format!("thinking: {}", thinking)),
                refusal: None,
                function_call: None,
                tool_calls: None,
            },
            None,
            None,
        )),
        MessagesContentDelta::InputJsonDelta { partial_json } => Ok(create_openai_chunk(
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
        )),
    }
}

/// Helper to create OpenAI streaming chunk
fn create_openai_chunk(
    id: &str,
    model: &str,
    delta: MessageDelta,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
) -> ChatCompletionsStreamResponse {
    ChatCompletionsStreamResponse {
        id: id.to_string(),
        object: Some("chat.completion.chunk".to_string()),
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

/// Convert Anthropic content blocks to OpenAI message content
fn convert_anthropic_content_to_openai(
    content: &[MessagesContentBlock],
) -> Result<MessageContent, TransformError> {
    let mut text_parts = Vec::new();

    for block in content {
        match block {
            MessagesContentBlock::Text { text, .. } => {
                text_parts.push(text.clone());
            }
            MessagesContentBlock::Thinking { thinking, .. } => {
                text_parts.push(format!("thinking: {}", thinking));
            }
            _ => {
                // Skip other content types for basic text conversion
                continue;
            }
        }
    }

    Ok(MessageContent::Text(text_parts.join("\n")))
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

/// Convert Bedrock Message to OpenAI content and tool calls
/// This function extracts text content and tool calls from a Bedrock message
fn convert_bedrock_message_to_openai(
    message: &crate::apis::amazon_bedrock::Message,
) -> Result<(Option<String>, Option<Vec<crate::apis::openai::ToolCall>>), TransformError> {
    use crate::apis::amazon_bedrock::ContentBlock;
    use crate::apis::openai::{FunctionCall, ToolCall};

    let mut text_content = String::new();
    let mut tool_calls = Vec::new();

    for content_block in &message.content {
        match content_block {
            ContentBlock::Text { text } => {
                text_content.push_str(text);
            }
            ContentBlock::ToolUse { tool_use } => {
                tool_calls.push(ToolCall {
                    id: tool_use.tool_use_id.clone(),
                    call_type: "function".to_string(),
                    function: FunctionCall {
                        name: tool_use.name.clone(),
                        arguments: serde_json::to_string(&tool_use.input).unwrap_or_default(),
                    },
                });
            }
            _ => continue,
        }
    }

    let content = if text_content.is_empty() {
        None
    } else {
        Some(text_content)
    };
    let tool_calls = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    Ok((content, tool_calls))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::amazon_bedrock::{
        BedrockTokenUsage, ContentBlock, ConversationRole, ConverseOutput, ConverseResponse,
        ConverseTrace, Message as BedrockMessage, PromptRouterTrace, StopReason,
    };
    use crate::apis::openai::{ChatCompletionsResponse, FinishReason, Role};
    use serde_json::json;

    #[test]
    fn test_bedrock_to_openai_basic_response() {
        let bedrock_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![ContentBlock::Text {
                        text: "Hello! How can I help you today?".to_string(),
                    }],
                },
            },
            stop_reason: StopReason::EndTurn,
            usage: BedrockTokenUsage {
                input_tokens: 10,
                output_tokens: 25,
                total_tokens: 35,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();

        assert_eq!(openai_response.object, Some("chat.completion".to_string()));
        assert_eq!(openai_response.model, "bedrock-model");
        assert!(openai_response.id.starts_with("bedrock-"));

        // Check choices
        assert_eq!(openai_response.choices.len(), 1);
        let choice = &openai_response.choices[0];
        assert_eq!(choice.index, 0);
        assert_eq!(choice.message.role, Role::Assistant);
        assert_eq!(
            choice.message.content,
            Some("Hello! How can I help you today?".to_string())
        );
        assert_eq!(choice.finish_reason, Some(FinishReason::Stop));
        assert!(choice.message.tool_calls.is_none());

        // Check usage
        assert_eq!(openai_response.usage.prompt_tokens, 10);
        assert_eq!(openai_response.usage.completion_tokens, 25);
        assert_eq!(openai_response.usage.total_tokens, 35);
    }

    #[test]
    fn test_bedrock_to_openai_with_tool_use() {
        let bedrock_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![
                        ContentBlock::Text {
                            text: "I'll help you check the weather.".to_string(),
                        },
                        ContentBlock::ToolUse {
                            tool_use: crate::apis::amazon_bedrock::ToolUseBlock {
                                tool_use_id: "tool_12345".to_string(),
                                name: "get_weather".to_string(),
                                input: json!({
                                    "location": "San Francisco"
                                }),
                            },
                        },
                    ],
                },
            },
            stop_reason: StopReason::ToolUse,
            usage: BedrockTokenUsage {
                input_tokens: 15,
                output_tokens: 30,
                total_tokens: 45,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();

        assert_eq!(
            openai_response.choices[0].finish_reason,
            Some(FinishReason::ToolCalls)
        );
        assert_eq!(
            openai_response.choices[0].message.content,
            Some("I'll help you check the weather.".to_string())
        );

        // Check tool calls
        let tool_calls = openai_response.choices[0]
            .message
            .tool_calls
            .as_ref()
            .unwrap();
        assert_eq!(tool_calls.len(), 1);

        let tool_call = &tool_calls[0];
        assert_eq!(tool_call.id, "tool_12345");
        assert_eq!(tool_call.call_type, "function");
        assert_eq!(tool_call.function.name, "get_weather");

        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments).unwrap();
        assert_eq!(args["location"], "San Francisco");
    }

    #[test]
    fn test_bedrock_to_openai_stop_reason_conversions() {
        let test_cases = vec![
            (StopReason::EndTurn, FinishReason::Stop),
            (StopReason::ToolUse, FinishReason::ToolCalls),
            (StopReason::MaxTokens, FinishReason::Length),
            (StopReason::StopSequence, FinishReason::Stop),
            (StopReason::GuardrailIntervened, FinishReason::ContentFilter),
            (StopReason::ContentFiltered, FinishReason::ContentFilter),
        ];

        for (bedrock_stop_reason, expected_openai_finish_reason) in test_cases {
            let bedrock_response = ConverseResponse {
                output: ConverseOutput::Message {
                    message: BedrockMessage {
                        role: ConversationRole::Assistant,
                        content: vec![ContentBlock::Text {
                            text: "Test response".to_string(),
                        }],
                    },
                },
                stop_reason: bedrock_stop_reason,
                usage: BedrockTokenUsage {
                    input_tokens: 5,
                    output_tokens: 10,
                    total_tokens: 15,
                    ..Default::default()
                },
                metrics: None,
                trace: None,
                additional_model_response_fields: None,
                performance_config: None,
            };

            let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();
            assert_eq!(
                openai_response.choices[0].finish_reason,
                Some(expected_openai_finish_reason)
            );
        }
    }

    #[test]
    fn test_bedrock_to_openai_multiple_tool_calls() {
        let bedrock_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![
                        ContentBlock::Text {
                            text: "I'll help with multiple tasks.".to_string(),
                        },
                        ContentBlock::ToolUse {
                            tool_use: crate::apis::amazon_bedrock::ToolUseBlock {
                                tool_use_id: "tool_1".to_string(),
                                name: "search".to_string(),
                                input: json!({"query": "weather"}),
                            },
                        },
                        ContentBlock::ToolUse {
                            tool_use: crate::apis::amazon_bedrock::ToolUseBlock {
                                tool_use_id: "tool_2".to_string(),
                                name: "lookup".to_string(),
                                input: json!({"id": "12345"}),
                            },
                        },
                    ],
                },
            },
            stop_reason: StopReason::ToolUse,
            usage: BedrockTokenUsage {
                input_tokens: 25,
                output_tokens: 40,
                total_tokens: 65,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();

        assert_eq!(
            openai_response.choices[0].finish_reason,
            Some(FinishReason::ToolCalls)
        );
        assert_eq!(
            openai_response.choices[0].message.content,
            Some("I'll help with multiple tasks.".to_string())
        );

        // Check multiple tool calls
        let tool_calls = openai_response.choices[0]
            .message
            .tool_calls
            .as_ref()
            .unwrap();
        assert_eq!(tool_calls.len(), 2);

        // First tool call
        assert_eq!(tool_calls[0].id, "tool_1");
        assert_eq!(tool_calls[0].function.name, "search");
        let args1: serde_json::Value =
            serde_json::from_str(&tool_calls[0].function.arguments).unwrap();
        assert_eq!(args1["query"], "weather");

        // Second tool call
        assert_eq!(tool_calls[1].id, "tool_2");
        assert_eq!(tool_calls[1].function.name, "lookup");
        let args2: serde_json::Value =
            serde_json::from_str(&tool_calls[1].function.arguments).unwrap();
        assert_eq!(args2["id"], "12345");
    }

    #[test]
    fn test_bedrock_to_openai_mixed_content() {
        let bedrock_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![
                        ContentBlock::Text {
                            text: "First part. ".to_string(),
                        },
                        ContentBlock::ToolUse {
                            tool_use: crate::apis::amazon_bedrock::ToolUseBlock {
                                tool_use_id: "tool_mid".to_string(),
                                name: "calculate".to_string(),
                                input: json!({"expr": "2+2"}),
                            },
                        },
                        ContentBlock::Text {
                            text: "Second part.".to_string(),
                        },
                    ],
                },
            },
            stop_reason: StopReason::ToolUse,
            usage: BedrockTokenUsage {
                input_tokens: 20,
                output_tokens: 35,
                total_tokens: 55,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();

        // Content should be combined text parts (no separator added)
        assert_eq!(
            openai_response.choices[0].message.content,
            Some("First part. Second part.".to_string())
        );

        // Should have one tool call
        let tool_calls = openai_response.choices[0]
            .message
            .tool_calls
            .as_ref()
            .unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "tool_mid");
        assert_eq!(tool_calls[0].function.name, "calculate");
    }

    #[test]
    fn test_bedrock_to_openai_empty_content() {
        let bedrock_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![ContentBlock::ToolUse {
                        tool_use: crate::apis::amazon_bedrock::ToolUseBlock {
                            tool_use_id: "tool_only".to_string(),
                            name: "action".to_string(),
                            input: json!({}),
                        },
                    }],
                },
            },
            stop_reason: StopReason::ToolUse,
            usage: BedrockTokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();

        // Content should be None when there's no text
        assert_eq!(openai_response.choices[0].message.content, None);

        // Should have tool call
        let tool_calls = openai_response.choices[0]
            .message
            .tool_calls
            .as_ref()
            .unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "tool_only");
    }

    #[test]
    fn test_convert_bedrock_message_to_openai() {
        let bedrock_message = BedrockMessage {
            role: ConversationRole::Assistant,
            content: vec![
                ContentBlock::Text {
                    text: "Hello world!".to_string(),
                },
                ContentBlock::ToolUse {
                    tool_use: crate::apis::amazon_bedrock::ToolUseBlock {
                        tool_use_id: "test_tool".to_string(),
                        name: "test_function".to_string(),
                        input: json!({"param": "value"}),
                    },
                },
            ],
        };

        let (content, tool_calls) = convert_bedrock_message_to_openai(&bedrock_message).unwrap();

        assert_eq!(content, Some("Hello world!".to_string()));

        let tool_calls = tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "test_tool");
        assert_eq!(tool_calls[0].function.name, "test_function");

        let args: serde_json::Value =
            serde_json::from_str(&tool_calls[0].function.arguments).unwrap();
        assert_eq!(args["param"], "value");
    }

    #[test]
    fn test_bedrock_to_openai_role_conversion() {
        // Test Assistant role
        let assistant_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![ContentBlock::Text {
                        text: "I am an assistant".to_string(),
                    }],
                },
            },
            stop_reason: StopReason::EndTurn,
            usage: BedrockTokenUsage {
                input_tokens: 5,
                output_tokens: 10,
                total_tokens: 15,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = assistant_response.try_into().unwrap();
        assert_eq!(openai_response.choices[0].message.role, Role::Assistant);

        // Test User role
        let user_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::User,
                    content: vec![ContentBlock::Text {
                        text: "I am a user".to_string(),
                    }],
                },
            },
            stop_reason: StopReason::EndTurn,
            usage: BedrockTokenUsage {
                input_tokens: 5,
                output_tokens: 10,
                total_tokens: 15,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = user_response.try_into().unwrap();
        assert_eq!(openai_response.choices[0].message.role, Role::User);
    }

    #[test]
    fn test_bedrock_to_openai_model_extraction() {
        // Test model extraction from trace information
        let bedrock_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![ContentBlock::Text {
                        text: "Test response".to_string(),
                    }],
                },
            },
            stop_reason: StopReason::EndTurn,
            usage: BedrockTokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
                ..Default::default()
            },
            metrics: None,
            trace: Some(ConverseTrace {
                guardrail: None,
                prompt_router: Some(PromptRouterTrace {
                    invoked_model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
                }),
            }),
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();

        // Should extract model ID from trace
        assert_eq!(
            openai_response.model,
            "anthropic.claude-3-sonnet-20240229-v1:0"
        );

        // Test fallback when no trace information is available
        let bedrock_response_no_trace = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![ContentBlock::Text {
                        text: "Test response".to_string(),
                    }],
                },
            },
            stop_reason: StopReason::EndTurn,
            usage: BedrockTokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response_fallback: ChatCompletionsResponse =
            bedrock_response_no_trace.try_into().unwrap();

        // Should use fallback model name
        assert_eq!(openai_response_fallback.model, "bedrock-model");
    }

    #[test]
    fn test_bedrock_to_openai_with_multimedia_content() {
        use crate::apis::amazon_bedrock::ImageSource;

        let bedrock_response = ConverseResponse {
            output: ConverseOutput::Message {
                message: BedrockMessage {
                    role: ConversationRole::Assistant,
                    content: vec![
                        ContentBlock::Text {
                            text: "Here's the analysis:".to_string(),
                        },
                        ContentBlock::Image {
                            image: crate::apis::amazon_bedrock::ImageBlock {
                                source: ImageSource::Base64 {
                                    media_type: "image/jpeg".to_string(),
                                    data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==".to_string(),
                                },
                            },
                        }
                    ],
                },
            },
            stop_reason: StopReason::EndTurn,
            usage: BedrockTokenUsage {
                input_tokens: 50,
                output_tokens: 75,
                total_tokens: 125,
                ..Default::default()
            },
            metrics: None,
            trace: None,
            additional_model_response_fields: None,
            performance_config: None,
        };

        let openai_response: ChatCompletionsResponse = bedrock_response.try_into().unwrap();

        assert_eq!(
            openai_response.choices[0].finish_reason,
            Some(FinishReason::Stop)
        );

        let content = openai_response.choices[0].message.content.as_ref().unwrap();

        // Check that text content is preserved (image blocks are currently ignored)
        assert!(content.contains("Here's the analysis:"));
        // Note: Image blocks are not converted to text in the current implementation
    }
}
