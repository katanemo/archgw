use http::StatusCode;
use proxy_wasm_test_framework::tester::{self, Tester};
use proxy_wasm_test_framework::types::{
    Action, BufferType, LogLevel, MapType, MetricType, ReturnType,
};
use serial_test::serial;
use std::path::Path;

fn wasm_module() -> String {
    let wasm_file = Path::new("../target/wasm32-wasip1/release/llm_gateway.wasm");
    assert!(
        wasm_file.exists(),
        "Run `cargo build --release --target=wasm32-wasip1` first"
    );
    wasm_file.to_string_lossy().to_string()
}

fn request_headers_expectations(module: &mut Tester, http_context: i32) {
    module
        .call_proxy_on_request_headers(http_context, 0, false)
        .expect_get_header_map_value(Some(MapType::HttpRequestHeaders), Some(":path"))
        .returning(Some("/v1/chat/completions"))
        .expect_get_header_map_value(
            Some(MapType::HttpRequestHeaders),
            Some("x-arch-llm-provider"),
        )
        .returning(None)
        .expect_get_header_map_value(
            Some(MapType::HttpRequestHeaders),
            Some("x-arch-llm-provider-hint"),
        )
        .returning(None)
        .expect_log(
            Some(LogLevel::Info),
            None, // Dynamic request ID - could be context_id or x-request-id
        )
        .expect_add_header_map_value(
            Some(MapType::HttpRequestHeaders),
            Some("x-arch-llm-provider"),
            Some("openai"),
        )
        .expect_remove_header_map_value(Some(MapType::HttpRequestHeaders), Some("x-api-key"))
        .expect_replace_header_map_value(
            Some(MapType::HttpRequestHeaders),
            Some("Authorization"),
            Some("Bearer secret_key"),
        )
        .expect_remove_header_map_value(Some(MapType::HttpRequestHeaders), Some("content-length"))
        .expect_get_header_map_value(
            Some(MapType::HttpRequestHeaders),
            Some("x-arch-llm-provider-hint"),
        )
        .returning(Some("default"))
        .expect_get_header_map_value(
            Some(MapType::HttpRequestHeaders),
            Some("x-arch-ratelimit-selector"),
        )
        .returning(Some("selector-key"))
        .expect_get_header_map_value(Some(MapType::HttpRequestHeaders), Some("selector-key"))
        .returning(Some("selector-value"))
        .expect_get_header_map_value(Some(MapType::HttpRequestHeaders), Some("x-request-id"))
        .returning(None)
        .expect_get_header_map_value(Some(MapType::HttpRequestHeaders), Some("traceparent"))
        .returning(None)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}

fn normal_flow(module: &mut Tester, filter_context: i32, http_context: i32) {
    module
        .call_proxy_on_context_create(http_context, filter_context)
        .expect_log(Some(LogLevel::Trace), None)
        .execute_and_expect(ReturnType::None)
        .unwrap();

    request_headers_expectations(module, http_context);
}

fn setup_filter(module: &mut Tester, config: &str) -> i32 {
    let filter_context = 1;

    module
        .call_proxy_on_context_create(filter_context, 0)
        .expect_metric_creation(MetricType::Gauge, "active_http_calls")
        .expect_metric_creation(MetricType::Counter, "ratelimited_rq")
        .expect_metric_creation(MetricType::Histogram, "time_to_first_token")
        .expect_metric_creation(MetricType::Histogram, "time_per_output_token")
        .expect_metric_creation(MetricType::Histogram, "tokens_per_second")
        .expect_metric_creation(MetricType::Histogram, "request_latency")
        .expect_metric_creation(MetricType::Histogram, "output_sequence_length")
        .expect_metric_creation(MetricType::Histogram, "input_sequence_length")
        .execute_and_expect(ReturnType::None)
        .unwrap();

    module
        .call_proxy_on_configure(filter_context, config.len() as i32)
        .expect_get_buffer_bytes(Some(BufferType::PluginConfiguration))
        .returning(Some(config))
        .execute_and_expect(ReturnType::Bool(true))
        .unwrap();

    filter_context
}

fn default_config() -> &'static str {
    r#"
version: "0.1-beta"

listener:
  address: 0.0.0.0
  port: 10000
  message_format: huggingface
  connect_timeout: 0.005s

endpoints:
  api_server:
    endpoint: api_server:80
    connect_timeout: 0.005s

llm_providers:
  - name: open-ai-gpt-4
    provider_interface: openai
    access_key: secret_key
    model: gpt-4
    default: true
  - name: open-ai-gpt-4o
    provider_interface: openai
    access_key: secret_key
    model: gpt-4o

overrides:
  # confidence threshold for prompt target intent matching
  prompt_target_intent_matching_threshold: 0.6

system_prompt: |
  You are a helpful assistant.

prompt_guards:
  input_guards:
    jailbreak:
      on_exception:
        message: "Looks like you're curious about my abilities, but I can only provide assistance within my programmed parameters."

prompt_targets:
  - name: weather_forecast
    description: This function provides realtime weather forecast information for a given city.
    parameters:
      - name: city
        required: true
        description: The city for which the weather forecast is requested.
      - name: days
        description: The number of days for which the weather forecast is requested.
      - name: units
        description: The units in which the weather forecast is requested.
    endpoint:
      name: api_server
      path: /weather
    system_prompt: |
      You are a helpful weather forecaster. Use weater data that is provided to you. Please following following guidelines when responding to user queries:
      - Use farenheight for temperature
      - Use miles per hour for wind speed

ratelimits:
  - model: gpt-4
    selector:
      key: selector-key
      value: selector-value
    limit:
      tokens: 100
      unit: minute
"#
}

#[test]
#[serial]
fn llm_gateway_successful_request_to_open_ai_chat_completions() {
    let args = tester::MockSettings {
        wasm_path: wasm_module(),
        quiet: false,
        allow_unexpected: false,
    };
    let mut module = tester::mock(args).unwrap();

    module
        .call_start()
        .execute_and_expect(ReturnType::None)
        .unwrap();

    // Setup Filter
    let filter_context = setup_filter(&mut module, default_config());

    // Setup HTTP Stream
    let http_context = 2;

    module
        .call_proxy_on_context_create(http_context, filter_context)
        .expect_log(Some(LogLevel::Trace), None)
        .execute_and_expect(ReturnType::None)
        .unwrap();

    request_headers_expectations(&mut module, http_context);

    // Request Body
    let chat_completions_request_body = r#"{"model":"gpt-4","messages":[{"role":"system","content":"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair."},{"role":"user","content":"Compose a poem."}]}"#;

    module
        .call_proxy_on_request_body(
            http_context,
            chat_completions_request_body.len() as i32,
            true,
        )
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID - REQUEST_BODY_CHUNK
        .expect_get_buffer_bytes(Some(BufferType::HttpRequestBody))
        .returning(Some(chat_completions_request_body))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - CLIENT_REQUEST_RECEIVED
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID - CLIENT_REQUEST_PAYLOAD
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - MODEL_RESOLUTION
        .expect_log(Some(LogLevel::Debug), Some("TOKENIZER: computing token count for model=gpt-4"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - TOKEN_COUNT
        .expect_metric_record("input_sequence_length", 21)
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - RATELIMIT_CHECK
        .expect_log(Some(LogLevel::Debug), Some("Checking limit for provider=gpt-4, with selector=Header { key: \"selector-key\", value: \"selector-value\" }, consuming tokens=21"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - UPSTREAM_TRANSFORM
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID - UPSTREAM_REQUEST_PAYLOAD
        .expect_set_buffer_bytes(Some(BufferType::HttpRequestBody), None)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}

#[test]
#[serial]
fn llm_gateway_bad_request_to_open_ai_chat_completions() {
    let args = tester::MockSettings {
        wasm_path: wasm_module(),
        quiet: false,
        allow_unexpected: false,
    };
    let mut module = tester::mock(args).unwrap();

    module
        .call_start()
        .execute_and_expect(ReturnType::None)
        .unwrap();

    // Setup Filter
    let filter_context = setup_filter(&mut module, default_config());

    // Setup HTTP Stream
    let http_context = 2;

    module
        .call_proxy_on_context_create(http_context, filter_context)
        .expect_log(Some(LogLevel::Trace), None)
        .execute_and_expect(ReturnType::None)
        .unwrap();

    request_headers_expectations(&mut module, http_context);

    // Request Body
    let incomplete_chat_completions_request_body = r#"{"model":"gpt-1","messages":[{"role":"system","content":"Compose a poem that explains the concept of recursion in programming."}]}"#;

    module
        .call_proxy_on_request_body(
            http_context,
            incomplete_chat_completions_request_body.len() as i32,
            true,
        )
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID - REQUEST_BODY_CHUNK
        .expect_get_buffer_bytes(Some(BufferType::HttpRequestBody))
        .returning(Some(incomplete_chat_completions_request_body))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - CLIENT_REQUEST_RECEIVED
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID - CLIENT_REQUEST_PAYLOAD
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - MODEL_RESOLUTION
        .expect_log(Some(LogLevel::Debug), Some("TOKENIZER: computing token count for model=gpt-4"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - TOKEN_COUNT
        .expect_metric_record("input_sequence_length", 13)
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - RATELIMIT_CHECK
        .expect_log(Some(LogLevel::Debug), Some("Checking limit for provider=gpt-4, with selector=Header { key: \"selector-key\", value: \"selector-value\" }, consuming tokens=13"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID - RATELIMIT_CHECK
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] UPSTREAM_REQUEST_PAYLOAD: {\"messages\":[{\"role\":\"system\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}],\"model\":\"gpt-4\"}"))
        .expect_set_buffer_bytes(Some(BufferType::HttpRequestBody), None)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}

#[test]
#[serial]
fn llm_gateway_request_ratelimited() {
    let args = tester::MockSettings {
        wasm_path: wasm_module(),
        quiet: false,
        allow_unexpected: false,
    };
    let mut module = tester::mock(args).unwrap();

    module
        .call_start()
        .execute_and_expect(ReturnType::None)
        .unwrap();

    // Setup Filter
    let filter_context = setup_filter(&mut module, default_config());

    // Setup HTTP Stream
    let http_context = 2;

    normal_flow(&mut module, filter_context, http_context);

    // Request Body
    let chat_completions_request_body = "\
{\
    \"messages\": [\
    {\
        \"role\": \"system\",\
        \"content\": \"You are a helpful poetic assistant!, skilled in explaining complex programming concepts with creative flair. Be sure to be concise and to the point.\"\
    },\
    {\
        \"role\": \"user\",\
        \"content\": \"Compose a poem that explains the concept of recursion in programming. Compose a poem that explains the concept of recursion in programming. Compose a poem that explains the concept of recursion in programming. And also summarize it how a 4th graded would understand it. Compose a poem that explains the concept of recursion in programming. And also summarize it how a 4th graded would understand it.\"\
    }\
    ],\
    \"model\": \"gpt-4\"\
}";

    module
        .call_proxy_on_request_body(
            http_context,
            chat_completions_request_body.len() as i32,
            true,
        )
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID)
        .expect_get_buffer_bytes(Some(BufferType::HttpRequestBody))
        .returning(Some(chat_completions_request_body))
        // The actual call is not important in this test, we just need to grab the token_id
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] CLIENT_REQUEST_PAYLOAD: {\"messages\": [{\"role\": \"system\",\"content\": \"You are a helpful poetic assistant!, skilled in explaining complex programming concepts with creative flair. Be sure to be concise and to the point.\"},{\"role\": \"user\",\"content\": \"Compose a poem that explains the concept of recursion in programming. Compose a poem that explains the concept of recursion in programming. Compose a poem that explains the concept of recursion in programming. And also summarize it how a 4th graded would understand it. Compose a poem that explains the concept of recursion in programming. And also summarize it how a 4th graded would understand it.\"}],\"model\": \"gpt-4\"}"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("TOKENIZER: computing token count for model=gpt-4"))
        .expect_log(Some(LogLevel::Info), None)// Dynamic request ID)
        .expect_metric_record("input_sequence_length", 107)
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("Checking limit for provider=gpt-4, with selector=Header { key: \"selector-key\", value: \"selector-value\" }, consuming tokens=107"))
        .expect_log(Some(LogLevel::Warn), Some(r#"server error occurred: exceeded limit provider=gpt-4, selector=Header { key: "selector-key", value: "selector-value" }, tokens_used=107"#))
        .expect_send_local_response(
            Some(StatusCode::TOO_MANY_REQUESTS.as_u16().into()),
            None,
            None,
            None,
        )
        .expect_metric_increment("ratelimited_rq", 1)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}

#[test]
#[serial]
fn llm_gateway_request_not_ratelimited() {
    let args = tester::MockSettings {
        wasm_path: wasm_module(),
        quiet: false,
        allow_unexpected: false,
    };
    let mut module = tester::mock(args).unwrap();

    module
        .call_start()
        .execute_and_expect(ReturnType::None)
        .unwrap();

    // Setup Filter
    let filter_context = setup_filter(&mut module, default_config());

    // Setup HTTP Stream
    let http_context = 2;

    normal_flow(&mut module, filter_context, http_context);

    // give shorter body to avoid rate limiting
    let chat_completions_request_body = r#"{"model":"gpt-1","messages":[{"role":"system","content":"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair."},{"role":"user","content":"Compose a poem that explains the concept of recursion in programming."}]}"#;

    module
        .call_proxy_on_request_body(
            http_context,
            chat_completions_request_body.len() as i32,
            true,
        )
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID)
        .expect_get_buffer_bytes(Some(BufferType::HttpRequestBody))
        .returning(Some(chat_completions_request_body))
        // The actual call is not important in this test, we just need to grab the token_id
        .expect_log(Some(LogLevel::Info), None)
         // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] CLIENT_REQUEST_PAYLOAD: {\"model\":\"gpt-1\",\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}]}"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("TOKENIZER: computing token count for model=gpt-4"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_metric_record("input_sequence_length", 29)
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("Checking limit for provider=gpt-4, with selector=Header { key: \"selector-key\", value: \"selector-value\" }, consuming tokens=29"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] UPSTREAM_REQUEST_PAYLOAD: {\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}],\"model\":\"gpt-4\"}"))
        .expect_set_buffer_bytes(Some(BufferType::HttpRequestBody), None)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}

#[test]
#[serial]
fn llm_gateway_override_model_name() {
    let args = tester::MockSettings {
        wasm_path: wasm_module(),
        quiet: false,
        allow_unexpected: false,
    };
    let mut module = tester::mock(args).unwrap();

    module
        .call_start()
        .execute_and_expect(ReturnType::None)
        .unwrap();

    // Setup Filter
    let filter_context = setup_filter(&mut module, default_config());

    // Setup HTTP Stream
    let http_context = 2;

    normal_flow(&mut module, filter_context, http_context);

    // give shorter body to avoid rate limiting
    let chat_completions_request_body = r#"{"model":"gpt-1","messages":[{"role":"system","content":"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair."},{"role":"user","content":"Compose a poem that explains the concept of recursion in programming."}]}"#;

    module
        .call_proxy_on_request_body(
            http_context,
            chat_completions_request_body.len() as i32,
            true,
        )
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID)
        .expect_get_buffer_bytes(Some(BufferType::HttpRequestBody))
        .returning(Some(chat_completions_request_body))
        // The actual call is not important in this test, we just need to grab the token_id
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] CLIENT_REQUEST_PAYLOAD: {\"model\":\"gpt-1\",\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}]}"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("TOKENIZER: computing token count for model=gpt-4"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_metric_record("input_sequence_length", 29)
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("Checking limit for provider=gpt-4, with selector=Header { key: \"selector-key\", value: \"selector-value\" }, consuming tokens=29"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] UPSTREAM_REQUEST_PAYLOAD: {\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}],\"model\":\"gpt-4\"}"))
        .expect_set_buffer_bytes(Some(BufferType::HttpRequestBody), None)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}

#[test]
#[serial]
fn llm_gateway_override_use_default_model() {
    let args = tester::MockSettings {
        wasm_path: wasm_module(),
        quiet: false,
        allow_unexpected: false,
    };
    let mut module = tester::mock(args).unwrap();

    module
        .call_start()
        .execute_and_expect(ReturnType::None)
        .unwrap();

    // Setup Filter
    let filter_context = setup_filter(&mut module, default_config());

    // Setup HTTP Stream
    let http_context = 2;

    normal_flow(&mut module, filter_context, http_context);

    // give shorter body to avoid rate limiting
    let chat_completions_request_body = r#"{"model":"gpt-1","messages":[{"role":"system","content":"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair."},{"role":"user","content":"Compose a poem that explains the concept of recursion in programming."}]}"#;

    module
        .call_proxy_on_request_body(
            http_context,
            chat_completions_request_body.len() as i32,
            true,
        )
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID)
        .expect_get_buffer_bytes(Some(BufferType::HttpRequestBody))
        .returning(Some(chat_completions_request_body))
        // The actual call is not important in this test, we just need to grab the token_id
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] CLIENT_REQUEST_PAYLOAD: {\"model\":\"gpt-1\",\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}]}"))
        .expect_log(
            Some(LogLevel::Info),
            None // Dynamic request ID,
        )
        .expect_log(Some(LogLevel::Debug), Some("TOKENIZER: computing token count for model=gpt-4"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_metric_record("input_sequence_length", 29)
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("Checking limit for provider=gpt-4, with selector=Header { key: \"selector-key\", value: \"selector-value\" }, consuming tokens=29"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] UPSTREAM_REQUEST_PAYLOAD: {\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}],\"model\":\"gpt-4\"}"))
        .expect_set_buffer_bytes(Some(BufferType::HttpRequestBody), None)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}

#[test]
#[serial]
fn llm_gateway_override_use_model_name_none() {
    let args = tester::MockSettings {
        wasm_path: wasm_module(),
        quiet: false,
        allow_unexpected: false,
    };
    let mut module = tester::mock(args).unwrap();

    module
        .call_start()
        .execute_and_expect(ReturnType::None)
        .unwrap();

    // Setup Filter
    let filter_context = setup_filter(&mut module, default_config());

    // Setup HTTP Stream
    let http_context = 2;

    normal_flow(&mut module, filter_context, http_context);

    // give shorter body to avoid rate limiting
    let chat_completions_request_body = r#"{"model":"none","messages":[{"role":"system","content":"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair."},{"role":"user","content":"Compose a poem that explains the concept of recursion in programming."}]}"#;

    module
        .call_proxy_on_request_body(
            http_context,
            chat_completions_request_body.len() as i32,
            true,
        )
        .expect_log(Some(LogLevel::Debug), None) // Dynamic request ID)
        .expect_get_buffer_bytes(Some(BufferType::HttpRequestBody))
        .returning(Some(chat_completions_request_body))
        // The actual call is not important in this test, we just need to grab the token_id
        .expect_log(Some(LogLevel::Info), None)
         // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] CLIENT_REQUEST_PAYLOAD: {\"model\":\"none\",\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}]}"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("TOKENIZER: computing token count for model=gpt-4"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_metric_record("input_sequence_length", 29)
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("Checking limit for provider=gpt-4, with selector=Header { key: \"selector-key\", value: \"selector-value\" }, consuming tokens=29"))
        .expect_log(Some(LogLevel::Info), None) // Dynamic request ID)
        .expect_log(Some(LogLevel::Debug), Some("[ARCHGW_REQ_ID:NO_REQUEST_ID] UPSTREAM_REQUEST_PAYLOAD: {\"messages\":[{\"role\":\"system\",\"content\":\"You are a poetic assistant, skilled in explaining complex programming concepts with creative flair.\"},{\"role\":\"user\",\"content\":\"Compose a poem that explains the concept of recursion in programming.\"}],\"model\":\"gpt-4\"}"))
        .expect_set_buffer_bytes(Some(BufferType::HttpRequestBody), None)
        .execute_and_expect(ReturnType::Action(Action::Continue))
        .unwrap();
}
