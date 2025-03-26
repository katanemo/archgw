use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Mistral,
}

impl FromStr for Provider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Provider::OpenAI),
            "mistral" => Ok(Provider::Mistral),
            _ => Err(anyhow::anyhow!(format!("Invalid provider: {}", s))),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct StreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum Role {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ChatResponse {
    pub messages: Vec<Message>,
    pub usage: Option<Usage>,
}

pub trait LlmProvider {
    fn translate_request(&self, request: &ChatRequest) -> Result<Vec<u8>>;
    fn translate_response(&self, response: &Vec<u8>) -> Result<ChatResponse>;
}

pub struct LlmProviders {
    pub providers: HashMap<Provider, Box<dyn LlmProvider>>,
}

impl LlmProviders {
    pub fn new() -> LlmProviders {
        LlmProviders {
            providers: HashMap::from([
                (Provider::OpenAI, Box::new(OpenAI) as Box<dyn LlmProvider>),
                (Provider::Mistral, Box::new(Mistral) as Box<dyn LlmProvider>),
            ]),
        }
    }
}

pub struct OpenAI;
impl LlmProvider for OpenAI {
    fn translate_request(&self, request: &ChatRequest) -> Result<Vec<u8>> {
        serde_json::to_string(request)
            .map(|s| s.into_bytes())
            .map_err(Into::into)
    }

    fn translate_response(&self, response: &Vec<u8>) -> Result<ChatResponse> {
        serde_json::from_slice(response).map_err(Into::into)
    }
}

pub struct Mistral;
impl LlmProvider for Mistral {
    fn translate_request(&self, request: &ChatRequest) -> Result<Vec<u8>> {
        serde_json::to_string(request)
            .map(|s| s.into_bytes())
            .map_err(Into::into)
    }

    fn translate_response(&self, response: &Vec<u8>) -> Result<ChatResponse> {
        serde_json::from_slice(response).map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_translate_request_openai() {
        use super::{ChatRequest, LlmProvider, Message, OpenAI, Role};
        let request = ChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: Some("You are a helpful assistant.".to_string()),
                    model: None,
                },
                Message {
                    role: Role::User,
                    content: Some("I need help with my computer.".to_string()),
                    model: None,
                },
            ],
            temperature: None,
            max_tokens: None,
            stream: None,
            stream_options: None,
        };
        let openai = OpenAI;
        let result = openai.translate_request(&request).unwrap();
        let expected = r#"{"model":"gpt-3.5-turbo","messages":[{"role":"system","content":"You are a helpful assistant."},{"role":"user","content":"I need help with my computer."}]}"#;
        assert_eq!(String::from_utf8(result).unwrap(), expected);
    }
}
