use crate::apis::amazon_bedrock::{
    ContentBlock, ConversationRole, Message as BedrockMessage, ToolResultContentBlock,
};
use crate::apis::openai::{FunctionCall, Message, MessageContent, Role, ToolCall};
use crate::clients::TransformError;

/// Flatten Bedrock tool result content blocks into the single text payload that an
/// OpenAI `tool` message carries.
pub(crate) fn flatten_tool_result_content(content: &[ToolResultContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ToolResultContentBlock::Text { text } => Some(text.clone()),
            ToolResultContentBlock::Json { json } => serde_json::to_string(json).ok(),
            ToolResultContentBlock::Image { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Message Conversions
impl TryFrom<BedrockMessage> for Vec<Message> {
    type Error = TransformError;

    fn try_from(message: BedrockMessage) -> Result<Self, Self::Error> {
        let role = match message.role {
            ConversationRole::User => Role::User,
            ConversationRole::Assistant => Role::Assistant,
        };

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut tool_results = Vec::new();

        for block in message.content {
            match block {
                ContentBlock::Text { text } => text_parts.push(text),
                ContentBlock::ToolUse { tool_use } => {
                    tool_calls.push(ToolCall {
                        id: tool_use.tool_use_id,
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: tool_use.name,
                            arguments: serde_json::to_string(&tool_use.input)?,
                        },
                    });
                }
                ContentBlock::ToolResult { tool_result } => {
                    tool_results.push((
                        tool_result.tool_use_id,
                        flatten_tool_result_content(&tool_result.content),
                    ));
                }
                ContentBlock::Image { .. }
                | ContentBlock::Document { .. }
                | ContentBlock::GuardContent { .. } => continue,
            }
        }

        // Tool results come first, mirroring the Anthropic split: a Bedrock user message
        // that only carries tool results must not surface as a trailing user message.
        let mut result: Vec<Message> = tool_results
            .into_iter()
            .map(|(tool_use_id, text)| Message {
                role: Role::Tool,
                content: Some(MessageContent::Text(text)),
                name: None,
                tool_calls: None,
                tool_call_id: Some(tool_use_id),
            })
            .collect();

        let text = text_parts.join("\n");
        if !text.is_empty() || !tool_calls.is_empty() || result.is_empty() {
            result.push(Message {
                role,
                content: Some(MessageContent::Text(text)),
                name: None,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
                tool_call_id: None,
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::amazon_bedrock::{ToolResultBlock, ToolResultStatus, ToolUseBlock};
    use serde_json::json;

    #[test]
    fn test_bedrock_tool_result_becomes_tool_message() {
        let bedrock_message = BedrockMessage {
            role: ConversationRole::User,
            content: vec![ContentBlock::ToolResult {
                tool_result: ToolResultBlock {
                    tool_use_id: "call_1".to_string(),
                    content: vec![ToolResultContentBlock::Text {
                        text: "72F and sunny".to_string(),
                    }],
                    status: Some(ToolResultStatus::Success),
                },
            }],
        };

        let messages: Vec<Message> = bedrock_message.try_into().unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::Tool);
        assert_eq!(messages[0].tool_call_id, Some("call_1".to_string()));
        assert_eq!(
            messages[0].content.as_ref().map(|c| c.to_string()),
            Some("72F and sunny".to_string())
        );
    }

    #[test]
    fn test_bedrock_tool_result_with_trailing_text_keeps_user_message_last() {
        let bedrock_message = BedrockMessage {
            role: ConversationRole::User,
            content: vec![
                ContentBlock::ToolResult {
                    tool_result: ToolResultBlock {
                        tool_use_id: "call_1".to_string(),
                        content: vec![ToolResultContentBlock::Json {
                            json: json!({"temp": 72}),
                        }],
                        status: None,
                    },
                },
                ContentBlock::Text {
                    text: "now write the tests".to_string(),
                },
            ],
        };

        let messages: Vec<Message> = bedrock_message.try_into().unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::Tool);
        assert_eq!(
            messages[0].content.as_ref().map(|c| c.to_string()),
            Some(r#"{"temp":72}"#.to_string())
        );
        assert_eq!(messages[1].role, Role::User);
        assert_eq!(
            messages[1].content.as_ref().map(|c| c.to_string()),
            Some("now write the tests".to_string())
        );
    }

    #[test]
    fn test_bedrock_tool_use_becomes_assistant_tool_calls() {
        let bedrock_message = BedrockMessage {
            role: ConversationRole::Assistant,
            content: vec![
                ContentBlock::Text {
                    text: "Checking the weather".to_string(),
                },
                ContentBlock::ToolUse {
                    tool_use: ToolUseBlock {
                        tool_use_id: "call_1".to_string(),
                        name: "get_weather".to_string(),
                        input: json!({"city": "Seattle"}),
                    },
                },
            ],
        };

        let messages: Vec<Message> = bedrock_message.try_into().unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::Assistant);
        let tool_calls = messages[0].tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[0].function.name, "get_weather");
        assert_eq!(tool_calls[0].function.arguments, r#"{"city":"Seattle"}"#);
    }

    #[test]
    fn test_bedrock_text_only_message_round_trips() {
        let bedrock_message = BedrockMessage {
            role: ConversationRole::User,
            content: vec![ContentBlock::Text {
                text: "hello".to_string(),
            }],
        };

        let messages: Vec<Message> = bedrock_message.try_into().unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::User);
        assert!(messages[0].tool_calls.is_none());
    }
}
