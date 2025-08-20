# hermesllm

A Rust library for handling LLM (Large Language Model) API requests and responses with unified abstractions across multiple providers.

## Features

- Unified request/response types with provider-specific parsing
- Support for both streaming and non-streaming responses
- Type-safe provider identification
- OpenAI-compatible API structure with extensible provider support

## Supported Providers

- OpenAI
- Mistral
- Groq
- Deepseek
- Gemini
- Claude
- GitHub

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hermesllm = { path = "../hermesllm" }  # or appropriate path in workspace
```

## Usage

### Basic Request Parsing

```rust
use hermesllm::providers::{ProviderRequestType, ProviderRequest, ProviderId};

// Parse request from JSON bytes
let request_bytes = r#"{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello!"}]}"#;

// Parse with provider context
let request = ProviderRequestType::try_from((request_bytes.as_bytes(), &ProviderId::OpenAI))?;

// Access request properties
println!("Model: {}", request.model());
println!("User message: {:?}", request.get_recent_user_message());
println!("Is streaming: {}", request.is_streaming());
```

### Working with Responses

```rust
use hermesllm::providers::{ProviderResponseType, ProviderResponse};

// Parse response from provider
let response_bytes = /* JSON response from LLM */;
let response = ProviderResponseType::try_from((response_bytes, ProviderId::OpenAI))?;

// Extract token usage
if let Some((prompt, completion, total)) = response.extract_usage_counts() {
    println!("Tokens used: {}/{}/{}", prompt, completion, total);
}
```

### Handling Streaming Responses

```rust
use hermesllm::providers::{ProviderStreamResponseIter, ProviderStreamResponse};

// Create streaming iterator from SSE data
let sse_data = /* Server-Sent Events data */;
let mut stream = ProviderStreamResponseIter::try_from((sse_data, &ProviderId::OpenAI))?;

// Process streaming chunks
for chunk_result in stream {
    match chunk_result {
        Ok(chunk) => {
            if let Some(content) = chunk.content_delta() {
                print!("{}", content);
            }
            if chunk.is_final() {
                break;
            }
        }
        Err(e) => eprintln!("Stream error: {}", e),
    }
}
```

### Provider Compatibility

```rust
use hermesllm::providers::{ProviderId, has_compatible_api, supported_apis};

// Check API compatibility
let provider = ProviderId::Groq;
if has_compatible_api(&provider, "/v1/chat/completions") {
    println!("Provider supports chat completions");
}

// List supported APIs
let apis = supported_apis(&provider);
println!("Supported APIs: {:?}", apis);
```

## Core Types

### Provider Types
- `ProviderId` - Enum identifying supported providers (OpenAI, Mistral, Groq, etc.)
- `ProviderRequestType` - Enum wrapping provider-specific request types
- `ProviderResponseType` - Enum wrapping provider-specific response types
- `ProviderStreamResponseIter` - Iterator for streaming response chunks

### Traits
- `ProviderRequest` - Common interface for all request types
- `ProviderResponse` - Common interface for all response types
- `ProviderStreamResponse` - Interface for streaming response chunks
- `TokenUsage` - Interface for token usage information

### OpenAI API Types
- `ChatCompletionsRequest` - Chat completion request structure
- `ChatCompletionsResponse` - Chat completion response structure
- `Message`, `Role`, `MessageContent` - Message building blocks

## Architecture

The library uses a type-safe enum-based approach that:

- **Provides Type Safety**: All provider operations are checked at compile time
- **Enables Runtime Provider Selection**: Provider can be determined from request headers or config
- **Maintains Clean Abstractions**: Common traits hide provider-specific details
- **Supports Extensibility**: New providers can be added by extending the enums

All requests are parsed into a common `ProviderRequestType` enum which implements the `ProviderRequest` trait, allowing uniform access to request properties regardless of the underlying provider format.

## Examples

See the `src/lib.rs` tests for complete working examples of:
- Parsing requests with provider context
- Handling streaming responses
- Working with token usage information

## License

This project is licensed under the MIT License.
