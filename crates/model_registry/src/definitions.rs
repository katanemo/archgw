/// Default model definitions following CLIProxyAPI patterns
use crate::model_info::{ModelInfo, ModelPricing, ThinkingSupport};
use common::configuration::LlmProviderType;

/// Get default Claude models
pub fn get_claude_models() -> Vec<ModelInfo> {
    vec![
        {
            let mut model = ModelInfo::new(
                "claude-opus-4-5-20251101".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            );
            model.display_name = "Claude Opus 4.5".to_string();
            model.context_window = Some(200_000);
            model.max_completion_tokens = Some(64_000);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.thinking = Some(ThinkingSupport {
                min: 1024,
                max: 100_000,
                zero_allowed: false,
                dynamic_allowed: true,
                levels: Some(vec!["low".to_string(), "medium".to_string(), "high".to_string()]),
            });
            model.pricing = Some(ModelPricing {
                currency: "USD".to_string(),
                input_cost_per_1m_tokens: 3.0,
                output_cost_per_1m_tokens: 15.0,
            });
            model
        },
        {
            let mut model = ModelInfo::new(
                "claude-sonnet-4-20250514".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            );
            model.display_name = "Claude Sonnet 4".to_string();
            model.context_window = Some(200_000);
            model.max_completion_tokens = Some(64_000);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.pricing = Some(ModelPricing {
                currency: "USD".to_string(),
                input_cost_per_1m_tokens: 3.0,
                output_cost_per_1m_tokens: 15.0,
            });
            model
        },
        {
            let mut model = ModelInfo::new(
                "claude-haiku-3-5-20241022".to_string(),
                LlmProviderType::Anthropic,
                "anthropic".to_string(),
            );
            model.display_name = "Claude Haiku 3.5".to_string();
            model.context_window = Some(200_000);
            model.max_completion_tokens = Some(64_000);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.pricing = Some(ModelPricing {
                currency: "USD".to_string(),
                input_cost_per_1m_tokens: 0.8,
                output_cost_per_1m_tokens: 4.0,
            });
            model
        },
    ]
}

/// Get default Gemini models
pub fn get_gemini_models() -> Vec<ModelInfo> {
    vec![
        {
            let mut model = ModelInfo::new(
                "gemini-2.5-pro".to_string(),
                LlmProviderType::Gemini,
                "google".to_string(),
            );
            model.display_name = "Gemini 2.5 Pro".to_string();
            model.name = Some("models/gemini-2.5-pro".to_string());
            model.input_token_limit = Some(1_048_576);
            model.output_token_limit = Some(65_536);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.thinking = Some(ThinkingSupport {
                min: 128,
                max: 32_768,
                zero_allowed: false,
                dynamic_allowed: true,
                levels: None,
            });
            model.supported_generation_methods = Some(vec![
                "generateContent".to_string(),
                "countTokens".to_string(),
                "batchGenerateContent".to_string(),
            ]);
            model
        },
        {
            let mut model = ModelInfo::new(
                "gemini-2.5-flash".to_string(),
                LlmProviderType::Gemini,
                "google".to_string(),
            );
            model.display_name = "Gemini 2.5 Flash".to_string();
            model.name = Some("models/gemini-2.5-flash".to_string());
            model.input_token_limit = Some(1_000_000);
            model.output_token_limit = Some(8_000);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.supported_generation_methods = Some(vec![
                "generateContent".to_string(),
                "countTokens".to_string(),
            ]);
            model
        },
        {
            let mut model = ModelInfo::new(
                "gemini-1.5-pro".to_string(),
                LlmProviderType::Gemini,
                "google".to_string(),
            );
            model.display_name = "Gemini 1.5 Pro".to_string();
            model.name = Some("models/gemini-1.5-pro".to_string());
            model.input_token_limit = Some(2_000_000);
            model.output_token_limit = Some(8_000);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model
        },
    ]
}

/// Get default OpenAI models
pub fn get_openai_models() -> Vec<ModelInfo> {
    vec![
        {
            let mut model = ModelInfo::new(
                "gpt-4-turbo".to_string(),
                LlmProviderType::OpenAI,
                "openai".to_string(),
            );
            model.display_name = "GPT-4 Turbo".to_string();
            model.context_window = Some(128_000);
            model.max_completion_tokens = Some(4_096);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.pricing = Some(ModelPricing {
                currency: "USD".to_string(),
                input_cost_per_1m_tokens: 10.0,
                output_cost_per_1m_tokens: 30.0,
            });
            model
        },
        {
            let mut model = ModelInfo::new(
                "gpt-4o".to_string(),
                LlmProviderType::OpenAI,
                "openai".to_string(),
            );
            model.display_name = "GPT-4o".to_string();
            model.context_window = Some(128_000);
            model.max_completion_tokens = Some(4_096);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.pricing = Some(ModelPricing {
                currency: "USD".to_string(),
                input_cost_per_1m_tokens: 5.0,
                output_cost_per_1m_tokens: 15.0,
            });
            model
        },
        {
            let mut model = ModelInfo::new(
                "gpt-4o-mini".to_string(),
                LlmProviderType::OpenAI,
                "openai".to_string(),
            );
            model.display_name = "GPT-4o Mini".to_string();
            model.context_window = Some(128_000);
            model.max_completion_tokens = Some(4_096);
            model.supports_vision = true;
            model.supports_function_calling = true;
            model.pricing = Some(ModelPricing {
                currency: "USD".to_string(),
                input_cost_per_1m_tokens: 0.15,
                output_cost_per_1m_tokens: 0.6,
            });
            model
        },
    ]
}

/// Get default Groq models
pub fn get_groq_models() -> Vec<ModelInfo> {
    vec![
        {
            let mut model = ModelInfo::new(
                "mixtral-8x7b-32768".to_string(),
                LlmProviderType::Groq,
                "groq".to_string(),
            );
            model.display_name = "Mixtral 8x7B".to_string();
            model.context_window = Some(32_768);
            model.supports_function_calling = true;
            model
        },
        {
            let mut model = ModelInfo::new(
                "llama2-70b-4096".to_string(),
                LlmProviderType::Groq,
                "groq".to_string(),
            );
            model.display_name = "Llama 2 70B".to_string();
            model.context_window = Some(4_096);
            model
        },
    ]
}

/// Get default Mistral models
pub fn get_mistral_models() -> Vec<ModelInfo> {
    vec![
        {
            let mut model = ModelInfo::new(
                "mistral-large-latest".to_string(),
                LlmProviderType::Mistral,
                "mistral".to_string(),
            );
            model.display_name = "Mistral Large".to_string();
            model.context_window = Some(32_000);
            model.supports_function_calling = true;
            model
        },
        {
            let mut model = ModelInfo::new(
                "mistral-medium-latest".to_string(),
                LlmProviderType::Mistral,
                "mistral".to_string(),
            );
            model.display_name = "Mistral Medium".to_string();
            model.context_window = Some(32_000);
            model
        },
    ]
}

/// Get all default models
pub fn get_all_default_models() -> Vec<ModelInfo> {
    let mut models = Vec::new();
    models.extend(get_claude_models());
    models.extend(get_gemini_models());
    models.extend(get_openai_models());
    models.extend(get_groq_models());
    models.extend(get_mistral_models());
    models
}
