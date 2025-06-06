use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::convert::TryFrom;
use std::str;
use thiserror::Error;

use crate::Provider;

#[derive(Debug, Error)]
pub enum OpenAIError {
    #[error("json error: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("utf8 parsing error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("invalid streaming data err {source}, data: {data}")]
    InvalidStreamingData {
        source: serde_json::Error,
        data: String,
    },
    #[error("unsupported provider: {provider}")]
    UnsupportedProvider { provider: String },
}

type Result<T> = std::result::Result<T, OpenAIError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MultiPartContentType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "image_url")]
    ImageUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiPartContent {
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub content_type: MultiPartContentType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentType {
    Text(String),
    MultiPart(Vec<MultiPartContent>),
}

impl Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Text(text) => write!(f, "{}", text),
            ContentType::MultiPart(multi_part) => {
                let text_parts: Vec<String> = multi_part
                    .iter()
                    .filter_map(|part| {
                        if part.content_type == MultiPartContentType::Text {
                            part.text.clone()
                        } else if part.content_type == MultiPartContentType::ImageUrl {
                            // skip image URLs or their data in text representation
                            None
                        } else {
                            panic!("Unsupported content type: {:?}", part.content_type);
                        }
                    })
                    .collect();
                let combined_text = text_parts.join("\n");
                write!(f, "{}", combined_text)
            }
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Option<ContentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionsRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub n: Option<u32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub stop: Option<Vec<String>>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub stream_options: Option<StreamOptions>,
    pub tools: Option<Vec<Value>>,
}

impl TryFrom<&[u8]> for ChatCompletionsRequest {
    type Error = OpenAIError;
    fn try_from(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(OpenAIError::from)
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionsResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

impl TryFrom<&[u8]> for ChatCompletionsResponse {
    type Error = OpenAIError;
    fn try_from(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(OpenAIError::from)
    }
}

impl<'a> TryFrom<(&'a [u8], &'a Provider)> for ChatCompletionsResponse {
    type Error = OpenAIError;

    fn try_from(input: (&'a [u8], &'a Provider)) -> Result<Self> {
        // Use input.provider as needed, if necessary
        serde_json::from_slice(input.0).map_err(OpenAIError::from)
    }
}

impl ChatCompletionsRequest {
    pub fn to_bytes(&self, provider: Provider) -> Result<Vec<u8>> {
        match provider {
            Provider::OpenAI
            | Provider::Arch
            | Provider::Deepseek
            | Provider::Mistral
            | Provider::Groq
            | Provider::Gemini
            | Provider::Claude => serde_json::to_vec(self).map_err(OpenAIError::from),
            _ => Err(OpenAIError::UnsupportedProvider {
                provider: provider.to_string(),
            }),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaMessage {
    pub role: Option<String>,
    pub content: Option<ContentType>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: DeltaMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionStreamResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
    pub usage: Option<Usage>,
}

pub struct SseChatCompletionIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    lines: I,
}

impl<I> SseChatCompletionIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    pub fn new(lines: I) -> Self {
        Self { lines }
    }
}

impl<I> Iterator for SseChatCompletionIter<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    type Item = Result<ChatCompletionStreamResponse>;

    fn next(&mut self) -> Option<Self::Item> {
        for line in &mut self.lines {
            let line = line.as_ref();
            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data == "[DONE]" {
                    return None;
                }

                if data == r#"{"type": "ping"}"# {
                    continue; // Skip ping messages - that is usually from anthropic
                }

                return Some(
                    serde_json::from_str::<ChatCompletionStreamResponse>(data).map_err(|e| {
                        OpenAIError::InvalidStreamingData {
                            source: e,
                            data: data.to_string(),
                        }
                    }),
                );
            }
        }
        None
    }
}

impl<'a> TryFrom<(&'a [u8], &'a Provider)> for SseChatCompletionIter<str::Lines<'a>> {
    type Error = OpenAIError;

    fn try_from(input: (&'a [u8], &'a Provider)) -> Result<Self> {
        let s = std::str::from_utf8(input.0)?;
        // Use input.provider as needed
        Ok(SseChatCompletionIter::new(s.lines()))
    }
}

impl<'a> TryFrom<&'a [u8]> for SseChatCompletionIter<str::Lines<'a>> {
    type Error = OpenAIError;

    fn try_from(bytes: &'a [u8]) -> Result<Self> {
        let s = std::str::from_utf8(bytes)?;
        Ok(SseChatCompletionIter::new(s.lines()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetail {
    pub id: String,
    pub object: String,
    pub created: usize,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelObject {
    #[serde(rename = "list")]
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Models {
    pub object: ModelObject,
    pub data: Vec<ModelDetail>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_display() {
        let text_content = ContentType::Text("Hello, world!".to_string());
        assert_eq!(text_content.to_string(), "Hello, world!");

        let multi_part_content = ContentType::MultiPart(vec![
            MultiPartContent {
                text: Some("This is a text part.".to_string()),
                content_type: MultiPartContentType::Text,
            },
            MultiPartContent {
                text: Some("https://example.com/image.png".to_string()),
                content_type: MultiPartContentType::ImageUrl,
            },
        ]);
        assert_eq!(multi_part_content.to_string(), "This is a text part.");
    }

    #[test]
    fn test_chat_completions_request_text_type_array() {
        const CHAT_COMPLETIONS_REQUEST: &str = r#"
        {
          "model": "gpt-3.5-turbo",
          "messages": [
            {
              "role": "user",
              "content": [
                {
                  "type": "text",
                  "text": "What city do you want to know the weather for?"
                },
                {
                  "type": "text",
                  "text": "hello world"
                }
              ]
            }
          ]
        }
        "#;

        let chat_completions_request: ChatCompletionsRequest =
            serde_json::from_str(CHAT_COMPLETIONS_REQUEST).unwrap();
        assert_eq!(chat_completions_request.model, "gpt-3.5-turbo");
        if let Some(ContentType::MultiPart(multi_part_content)) =
            chat_completions_request.messages[0].content.as_ref()
        {
            assert_eq!(multi_part_content.len(), 2);
            assert_eq!(
                multi_part_content[0].content_type,
                MultiPartContentType::Text
            );
            assert_eq!(
                multi_part_content[0].text,
                Some("What city do you want to know the weather for?".to_string())
            );
            assert_eq!(
                multi_part_content[1].content_type,
                MultiPartContentType::Text
            );
            assert_eq!(multi_part_content[1].text, Some("hello world".to_string()));
        } else {
            panic!("Expected MultiPartContent");
        }
    }

    #[test]
    fn test_sse_streaming() {
        let json_data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1700000000,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1700000000,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"content":"Hello, how can I help you today?"},"finish_reason":null}]}
data: [DONE]"#;

        let iter = SseChatCompletionIter::new(json_data.lines());

        println!("Testing SSE Streaming");
        for item in iter {
            match item {
                Ok(response) => {
                    println!("Received response: {:?}", response);
                    if response.choices.is_empty() {
                        continue;
                    }
                    for choice in response.choices {
                        if let Some(content) = choice.delta.content {
                            println!("Content: {}", content);
                        }
                    }
                }
                Err(e) => {
                    println!("Error parsing JSON: {}", e);
                    return;
                }
            }
        }
    }

    #[test]
    fn test_sse_streaming_try_from_bytes() {
        let json_data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1700000000,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1700000000,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"content":"Hello, how can I help you today?"},"finish_reason":null}]}
data: [DONE]"#;

        let iter = SseChatCompletionIter::try_from(json_data.as_bytes())
            .expect("Failed to create SSE iterator");

        println!("Testing SSE Streaming");
        for item in iter {
            match item {
                Ok(response) => {
                    println!("Received response: {:?}", response);
                    if response.choices.is_empty() {
                        continue;
                    }
                    for choice in response.choices {
                        if let Some(content) = choice.delta.content {
                            println!("Content: {}", content);
                        }
                    }
                }
                Err(e) => {
                    println!("Error parsing JSON: {}", e);
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_chat_completions_request() {
        const CHAT_COMPLETIONS_REQUEST: &str = r#"
{
  "model": "None",
  "messages": [
    {
      "role": "user",
      "content": "how is the weather in seattle"
    }
  ],
  "stream": true
}        "#;

        let _chat_completions_request: ChatCompletionsRequest =
            ChatCompletionsRequest::try_from(CHAT_COMPLETIONS_REQUEST.as_bytes())
                .expect("Failed to parse ChatCompletionsRequest");
    }

    #[test]
    fn stream_chunk_parse_claude() {
        const CHUNK_RESPONSE: &str = r#"data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{"role":"assistant"}}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: {"type": "ping"}

data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{"content":"Hello!"}}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{"content":" How can I assist you today? Whether"}}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{"content":" you have a question, need information"}}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{"content":", or just want to chat about"}}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{"content":" something, I'm here to help. What woul"}}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{"content":"d you like to talk about?"}}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: {"id":"msg_01DZDMxYSgq8aPQxMQoBv6Kb","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"created":1747685264,"model":"claude-3-7-sonnet-latest","object":"chat.completion.chunk"}

data: [DONE]
"#;

        let iter = SseChatCompletionIter::try_from(CHUNK_RESPONSE.as_bytes());

        assert!(iter.is_ok(), "Failed to create SSE iterator");
        let iter: SseChatCompletionIter<str::Lines<'_>> = iter.unwrap();

        let all_text: Vec<String> = iter
            .map(|item| {
                let response = item.expect("Failed to parse response");
                response
                    .choices
                    .into_iter()
                    .filter_map(|choice| choice.delta.content)
                    .map(|content| content.to_string())
                    .collect::<String>()
            })
            .collect();

        assert_eq!(
            all_text.len(),
            8,
            "Expected 8 chunks of text, but got {}",
            all_text.len()
        );

        assert_eq!(
            all_text.join(""),
            "Hello! How can I assist you today? Whether you have a question, need information, or just want to chat about something, I'm here to help. What would you like to talk about?"
        );
    }
}
