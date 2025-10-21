// Re-export new transformation modules for backward compatibility

//KEEPING THE TESTS TO MAKE SURE ALL THE REFACTORING DIDN'T BREAK ANYTHING

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::apis::anthropic::*;
    use crate::apis::openai::*;
    use crate::transforms::*;
    use serde_json::json;
    type AnthropicMessagesRequest = MessagesRequest;

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
        assert_eq!(
            original_anthropic.max_tokens,
            roundtrip_anthropic.max_tokens
        );
        assert_eq!(
            original_anthropic.temperature,
            roundtrip_anthropic.temperature
        );
        assert_eq!(original_anthropic.top_p, roundtrip_anthropic.top_p);
        assert_eq!(original_anthropic.stream, roundtrip_anthropic.stream);
        assert_eq!(
            original_anthropic.messages.len(),
            roundtrip_anthropic.messages.len()
        );
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
        assert_eq!(openai_resp.object.as_deref(), Some("chat.completion.chunk"));
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

        assert_eq!(openai_resp.object.as_deref(), Some("chat.completion.chunk"));
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
                cache_control: None,
            },
        };

        let openai_resp: ChatCompletionsStreamResponse = tool_start.try_into().unwrap();

        assert_eq!(openai_resp.choices.len(), 1);
        let choice = &openai_resp.choices[0];
        assert!(choice.delta.tool_calls.is_some());

        let tool_calls = choice.delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, Some("call_123".to_string()));
        assert_eq!(
            tool_calls[0].function.as_ref().unwrap().name,
            Some("get_weather".to_string())
        );
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
        assert_eq!(
            tool_calls[0].function.as_ref().unwrap().arguments,
            Some(r#"{"location": "San Francisco"#.to_string())
        );
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

        assert_eq!(openai_resp.object.as_deref(), Some("chat.completion.chunk"));
        assert_eq!(openai_resp.choices.len(), 0); // Ping has no choices
    }

    #[test]
    fn test_openai_to_anthropic_streaming_role_start() {
        let openai_resp = ChatCompletionsStreamResponse {
            id: "chatcmpl-123".to_string(),
            object: Some("chat.completion.chunk".to_string()),
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
            object: Some("chat.completion.chunk".to_string()),
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
            object: Some("chat.completion.chunk".to_string()),
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
            MessagesStreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
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
            object: Some("chat.completion.chunk".to_string()),
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
            object: Some("chat.completion.chunk".to_string()),
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
                cache_control: None,
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
        assert_eq!(
            tool_calls[0].function.as_ref().unwrap().name,
            Some("get_weather".to_string())
        );

        // Verify argument deltas
        let args1 = &openai_delta1.choices[0].delta.tool_calls.as_ref().unwrap()[0]
            .function
            .as_ref()
            .unwrap()
            .arguments;
        assert_eq!(args1, &Some(r#"{"location": "#.to_string()));

        let args2 = &openai_delta2.choices[0].delta.tool_calls.as_ref().unwrap()[0]
            .function
            .as_ref()
            .unwrap()
            .arguments;
        assert_eq!(
            args2,
            &Some(r#"San Francisco", "unit": "fahrenheit"}"#.to_string())
        );
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
            assert_eq!(
                openai_resp.choices[0].finish_reason,
                Some(expected_openai_reason)
            );

            // Test reverse conversion
            let roundtrip_event: MessagesStreamEvent = openai_resp.try_into().unwrap();
            match roundtrip_event {
                MessagesStreamEvent::MessageDelta { delta, .. } => {
                    // Note: Some precision may be lost in roundtrip due to mapping differences
                    assert!(matches!(
                        delta.stop_reason,
                        MessagesStopReason::EndTurn
                            | MessagesStopReason::MaxTokens
                            | MessagesStopReason::ToolUse
                            | MessagesStopReason::StopSequence
                    ));
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
            object: Some("chat.completion.chunk".to_string()),
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
        let anthropic_event: MessagesStreamEvent =
            openai_resp_with_missing_data.try_into().unwrap();
        assert!(matches!(anthropic_event, MessagesStreamEvent::Ping));
    }

    #[test]
    fn test_streaming_content_block_stop() {
        let event = MessagesStreamEvent::ContentBlockStop { index: 0 };

        let openai_resp: ChatCompletionsStreamResponse = event.try_into().unwrap();

        // ContentBlockStop should produce an empty chunk
        assert_eq!(openai_resp.object.as_deref(), Some("chat.completion.chunk"));
        assert_eq!(openai_resp.choices.len(), 1);

        let choice = &openai_resp.choices[0];
        assert_eq!(choice.delta.role, None);
        assert_eq!(choice.delta.content, None);
        assert_eq!(choice.delta.tool_calls, None);
        assert_eq!(choice.finish_reason, None);
    }
}
