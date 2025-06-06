//! hermesllm: A library for translating LLM API requests and responses
//! between Mistral, Grok, Gemini, and OpenAI-compliant formats.

use std::fmt::Display;

pub mod providers;

pub enum Provider {
    Arch,
    Mistral,
    Deepseek,
    Groq,
    Gemini,
    OpenAI,
    Claude,
    Github,
}

impl From<&str> for Provider {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "arch" => Provider::Arch,
            "mistral" => Provider::Mistral,
            "deepseek" => Provider::Deepseek,
            "groq" => Provider::Groq,
            "gemini" => Provider::Gemini,
            "openai" => Provider::OpenAI,
            "claude" => Provider::Claude,
            "github" => Provider::Github,
            _ => panic!("Unknown provider: {}", value),
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Arch => write!(f, "Arch"),
            Provider::Mistral => write!(f, "Mistral"),
            Provider::Deepseek => write!(f, "Deepseek"),
            Provider::Groq => write!(f, "Groq"),
            Provider::Gemini => write!(f, "Gemini"),
            Provider::OpenAI => write!(f, "OpenAI"),
            Provider::Claude => write!(f, "Claude"),
            Provider::Github => write!(f, "Github"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::providers::openai::types::ChatCompletionsRequest;

    #[test]
    fn openai_builder() {
        let request = ChatCompletionsRequest::builder("gpt-3.5-turbo", vec![])
            .temperature(0.7)
            .top_p(0.9)
            .n(1)
            .max_tokens(100)
            .stream(false)
            .stop(vec!["\n".to_string()])
            .presence_penalty(0.0)
            .frequency_penalty(0.0)
            .build()
            .expect("Failed to build OpenAIRequest");

        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.n, Some(1));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.stream, Some(false));
        assert_eq!(request.stop, Some(vec!["\n".to_string()]));
        assert_eq!(request.presence_penalty, Some(0.0));
        assert_eq!(request.frequency_penalty, Some(0.0));
    }
}
