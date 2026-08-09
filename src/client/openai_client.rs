use crate::domain::tool::ToolCatalogue;

use super::openai_tool_definition::OpenAiToolDefinition;

#[derive(Debug, Clone)]
pub struct OpenAiClient {
    api_key: String,
    model: String,
    tool_catalogue: ToolCatalogue,
    max_output_tokens: u32,
    reasoning_effort: ReasoningEffort,
}

impl OpenAiClient {
    pub fn new(
        api_key: String,
        model: String,
        tool_catalogue: ToolCatalogue,
        max_output_tokens: u32,
        reasoning_effort: ReasoningEffort,
    ) -> Self {
        Self {
            api_key,
            model,
            tool_catalogue,
            max_output_tokens,
            reasoning_effort,
        }
    }

    pub fn build_tool_definitions(&self) -> Vec<OpenAiToolDefinition> {
        self.tool_catalogue
            .iter()
            .map(OpenAiToolDefinition::from)
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReasoningEffort {
    None,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}
