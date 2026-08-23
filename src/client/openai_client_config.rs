//! Holds the provider settings used for each OpenAI request.

use serde::Serialize;

#[derive(Debug, Clone)]
pub struct OpenAiClientConfig {
    pub model: String,
    pub uri: String,
    pub max_output_tokens: u32,
    pub reasoning_effort: ReasoningEffort,
}

impl OpenAiClientConfig {
    pub fn new(model: String) -> Self {
        return OpenAiClientConfig {
            model: model,
            uri: "https://api.openai.com/v1/responses".into(),
            max_output_tokens: 4096,
            reasoning_effort: ReasoningEffort::Medium,
        };
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}
