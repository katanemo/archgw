pub const DEFAULT_EMBEDDING_MODEL: &str = "BAAI/bge-large-en-v1.5";
pub const DEFAULT_INTENT_MODEL: &str = "tasksource/deberta-base-long-nli";
pub const DEFAULT_PROMPT_TARGET_THRESHOLD: f64 = 0.8;
pub const RATELIMIT_SELECTOR_HEADER_KEY: &str = "x-bolt-ratelimit-selector";
pub const SYSTEM_ROLE: &str = "system";
pub const USER_ROLE: &str = "user";
pub const GPT_35_TURBO: &str = "gpt-3.5-turbo";
pub const BOLT_FC_CLUSTER: &str = "bolt_fc_1b";
pub const BOLT_FC_REQUEST_TIMEOUT_MS: u64 = 120000; // 2 minutes
pub const MODEL_SERVER_NAME: &str = "model_server";
pub const BOLT_ROUTING_HEADER: &str = "x-bolt-llm-provider";
