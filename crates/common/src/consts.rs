pub const DEFAULT_EMBEDDING_MODEL: &str = "katanemo/bge-large-en-v1.5";
pub const DEFAULT_INTENT_MODEL: &str = "katanemo/bart-large-mnli";
pub const DEFAULT_PROMPT_TARGET_THRESHOLD: f64 = 0.8;
pub const DEFAULT_HALLUCINATED_THRESHOLD: f64 = 0.25;
pub const RATELIMIT_SELECTOR_HEADER_KEY: &str = "x-arch-ratelimit-selector";
pub const SYSTEM_ROLE: &str = "system";
pub const USER_ROLE: &str = "user";
pub const TOOL_ROLE: &str = "tool";
pub const ASSISTANT_ROLE: &str = "assistant";
pub const ARCH_FC_REQUEST_TIMEOUT_MS: u64 = 120000; // 2 minutes
pub const MODEL_SERVER_NAME: &str = "model_server";
pub const ZEROSHOT_INTERNAL_HOST: &str = "zeroshot";
pub const ARCH_FC_INTERNAL_HOST: &str = "arch_fc";
pub const HALLUCINATION_INTERNAL_HOST: &str = "hallucination";
pub const EMBEDDINGS_INTERNAL_HOST: &str = "embeddings";
pub const GUARD_INTERNAL_HOST: &str = "guard";
pub const ARCH_ROUTING_HEADER: &str = "x-arch-llm-provider";
pub const ARCH_MESSAGES_KEY: &str = "arch_messages";
pub const ARCH_PROVIDER_HINT_HEADER: &str = "x-arch-llm-provider-hint";
pub const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";
pub const ARCH_STATE_HEADER: &str = "x-arch-state";
pub const ARCH_FC_MODEL_NAME: &str = "Arch-Function-1.5B";
pub const REQUEST_ID_HEADER: &str = "x-request-id";
pub const ARCH_INTERNAL_CLUSTER_NAME: &str = "arch_internal";
pub const ARCH_UPSTREAM_HOST_HEADER: &str = "x-arch-upstream";
pub const ARCH_LLM_UPSTREAM_LISTENER: &str = "arch_llm_listener";
pub const ARCH_MODEL_PREFIX: &str = "Arch";
pub const HALLUCINATION_TEMPLATE: &str = "It seems I’m missing some information. Could you provide the following details";
