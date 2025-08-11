# hermesllm

A Rust library for translating LLM (Large Language Model) API requests and responses between Mistral, Groq, Gemini, Deepseek, OpenAI, and other provider-compliant formats.

## Features

- Unified traits for chat completions across multiple LLM providers
- Function-based API for runtime provider selection and conversion
- Direct trait implementations on concrete types (no wrapper types needed)
- Streaming and non-streaming response support
- Type-safe provider identification and conversion

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

### Basic Request/Response Handling

```rust
use hermesllm::{ProviderId, try_request_from_bytes, try_response_from_bytes, ConversionMode};

// Parse a request from raw bytes with provider-specific handling
let provider_id = ProviderId::OpenAI;
let request_bytes = r#"{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello!"}]}"#;
let request = try_request_from_bytes(request_bytes.as_bytes(), &provider_id)?;

// Work with the request using trait methods
println!("Model: {}", request.model());
println!("Is streaming: {}", request.is_streaming());
if let Some(user_msg) = request.extract_user_message() {
    println!("User message: {}", user_msg);
}
```

### Building Requests with the Builder Pattern

```rust
use hermesllm::apis::openai::{ChatCompletionsRequest, Message, Role, MessageContent};

// Build a request using the builder pattern
let request = ChatCompletionsRequest {
    model: "gpt-4".to_string(),
    messages: vec![
        Message {
            role: Role::System,
            content: MessageContent::Text("You are a helpful assistant".to_string()),
            ..Default::default()
        },
        Message {
            role: Role::User,
            content: MessageContent::Text("What is the capital of France?".to_string()),
            ..Default::default()
        }
    ],
    temperature: Some(0.7),
    max_tokens: Some(150),
    ..Default::default()
};

// Convert to provider-specific format
let provider_bytes = request.to_provider_bytes(ConversionMode::Compatible)?;
```

### Handling Responses

```rust
// Parse responses from provider
let response_bytes = /* response JSON from LLM provider */;
let response = try_response_from_bytes(&response_bytes, &provider_id, ConversionMode::Compatible)?;

// Extract usage information
if let Some((prompt_tokens, completion_tokens, total_tokens)) = response.extract_usage_counts() {
    println!("Token usage: {}/{}/{}", prompt_tokens, completion_tokens, total_tokens);
}
```

### Streaming Responses

```rust
// Handle streaming responses
let stream_data = /* SSE stream data */;
let streaming_iter = try_streaming_from_bytes(&stream_data, &provider_id, ConversionMode::Compatible)?;

for chunk_result in streaming_iter {
    match chunk_result {
        Ok(chunk) => {
            if let Some(content) = chunk.content_delta() {
                print!("{}", content);
            }
            if chunk.is_final() {
                println!("\nStream completed");
                break;
            }
        }
        Err(e) => {
            eprintln!("Streaming error: {}", e);
            break;
        }
    }
}
```

### Provider Compatibility

```rust
use hermesllm::{ProviderId, has_compatible_api, supported_apis};

// Check if a provider supports a specific API
let provider_id = ProviderId::Groq;
if has_compatible_api(&provider_id, "/v1/chat/completions") {
    println!("Groq supports chat completions API");
}

// Get all supported APIs for a provider
let apis = supported_apis(&provider_id);
println!("Groq supports: {:?}", apis);

// Runtime provider selection
let provider_name = "mistral"; // Could come from config or request header
let provider_id = ProviderId::from(provider_name);
```

## API Overview

### Core Functions
- `try_request_from_bytes()`: Parse requests from bytes with provider-specific handling
- `try_response_from_bytes()`: Parse responses from bytes with provider-specific handling
- `try_streaming_from_bytes()`: Create streaming response iterators
- `has_compatible_api()`: Check API compatibility for providers
- `supported_apis()`: Get supported API endpoints for providers

### Core Types
- `ProviderId`: Enum for identifying providers (OpenAI, Mistral, Groq, etc.)
- `ConversionMode`: Controls conversion behavior (Compatible, Passthrough)

### Traits
- `ProviderRequest`: Common interface for all request types
- `ProviderResponse`: Common interface for all response types
- `ProviderStreamResponse`: Interface for streaming response chunks
- `ProviderStreamResponseIter`: Iterator trait for streaming responses
- `TokenUsage`: Interface for token usage information

### Concrete Types
- `ChatCompletionsRequest`: OpenAI-compatible chat completion requests
- `ChatCompletionsResponse`: OpenAI-compatible chat completion responses
- `SseChatCompletionIter`: Streaming response iterator for SSE format

## Architecture

This library uses a function-based approach instead of traditional trait objects to enable:

- **Dynamic Provider Selection**: Runtime provider selection based on request headers or configuration
- **No Wrapper Types**: Direct trait implementations on concrete types like `ChatCompletionsRequest`
- **Type Erasure**: Functions return `Box<dyn ProviderRequest>` for polymorphic usage
- **Parameterized Conversion**: `TryFrom<(&[u8], &ProviderId)>` pattern for provider-specific parsing

The function-based design solves trait object limitations while maintaining clean abstractions and runtime flexibility.

## Contributing

Contributions are welcome! Please open issues or pull requests for bug fixes, new features, or provider integrations.

## License

This project is licensed under the terms of the [MIT License](../LICENSE).
