use serde_json::Value;

use crate::providers::openai::types::{ChatCompletionsRequest, Message, StreamOptions};

#[derive(Debug, Clone)]
pub struct OpenAIRequestBuilder {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<u32>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    stream_options: Option<StreamOptions>,
    tools: Option<Vec<Value>>,
}

impl OpenAIRequestBuilder {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            top_p: None,
            n: None,
            max_tokens: None,
            stream: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            stream_options: None,
            tools: None,
        }
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn n(mut self, n: u32) -> Self {
        self.n = Some(n);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    pub fn presence_penalty(mut self, presence_penalty: f32) -> Self {
        self.presence_penalty = Some(presence_penalty);
        self
    }

    pub fn frequency_penalty(mut self, frequency_penalty: f32) -> Self {
        self.frequency_penalty = Some(frequency_penalty);
        self
    }

    pub fn stream_options(mut self, include_usage: bool) -> Self {
        self.stream = Some(true);
        self.stream_options = Some(StreamOptions { include_usage });
        self
    }

    pub fn tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn build(self) -> Result<ChatCompletionsRequest, &'static str> {
        let request = ChatCompletionsRequest {
            model: self.model,
            messages: self.messages,
            temperature: self.temperature,
            top_p: self.top_p,
            n: self.n,
            max_tokens: self.max_tokens,
            stream: self.stream,
            stop: self.stop,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            stream_options: self.stream_options,
            tools: self.tools,
        };
        Ok(request)
    }
}

impl ChatCompletionsRequest {
    pub fn builder(model: impl Into<String>, messages: Vec<Message>) -> OpenAIRequestBuilder {
        OpenAIRequestBuilder::new(model, messages)
    }
}
