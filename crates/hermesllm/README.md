# hermesllm

A Rust library for translating LLM (Large Language Model) API requests and responses between Mistral, Groq, Gemini, Deepseek, OpenAI, and other provider-compliant formats.

## Features

- Unified types for chat completions and model metadata across multiple LLM providers
- Builder-pattern API for constructing requests in an idiomatic Rust style
- Easy conversion between provider formats
- Streaming and non-streaming response support

## Supported Providers

- Mistral
- Deepseek
- Groq
- Gemini
- OpenAI
- Claude
- Github

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
hermesllm = { git = "https://github.com/katanemo/archgw", subdir = "crates/hermesllm" }
```

_Replace the path with the appropriate location if using as a workspace member or published crate._

## Usage

Construct a chat completion request using the builder pattern:

```rust
use hermesllm::{create_provider, ProviderId};
use hermesllm::providers::openai::types::ChatCompletionsRequest;

let request = ChatCompletionsRequest::builder("gpt-3.5-turbo", vec![Message::new("Hi".to_string())])
    .build()
    .expect("Failed to build OpenAIRequest");

// Create a provider and convert request to bytes
let provider = create_provider(ProviderId::OpenAI);
let bytes = serde_json::to_vec(&request)?;
let parsed_request = provider.try_request_from_bytes(&bytes)?;
```

## API Overview

- `Provider`: Enum listing all supported LLM providers.
- `ChatCompletionsRequest`: Builder-pattern struct for creating chat completion requests.
- `ChatCompletionsResponse`: Struct for parsing responses.
- Streaming support via `SseChatCompletionIter`.
- Error handling via `OpenAIError`.

## Contributing

Contributions are welcome! Please open issues or pull requests for bug fixes, new features, or provider integrations.

## License

This project is licensed under the terms of the [MIT License](../LICENSE).
