//! Holds the provider settings used for each OpenAI request.

use std::env;

#[derive(Debug, Clone)]
pub struct OpenAiClientConfig {
    pub model: String,
    pub uri: String,
    pub max_output_tokens: u32,
    pub reasoning_effort: &'static str,
    pub parallel_tool_calls: bool,
}

impl OpenAiClientConfig {
    pub fn from_env() -> Self {
        Self {
            model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.6-luna".into()),
            uri: env::var("OPENAI_API_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1/responses".into()),
            max_output_tokens: 4096,
            reasoning_effort: "medium",
            parallel_tool_calls: true,
        }
    }
}
