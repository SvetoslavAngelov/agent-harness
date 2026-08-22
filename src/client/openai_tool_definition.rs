//! Serializes domain tool definitions in OpenAI's function-tool format.

use serde::Serialize;
use serde_json::Value;

use crate::domain::tool::ToolDefinition;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenAiToolDefinition {
    Function {
        name: String,
        description: String,
        parameters: Value,
        strict: bool,
    },
}

impl From<&ToolDefinition> for OpenAiToolDefinition {
    fn from(tool: &ToolDefinition) -> Self {
        Self::Function {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
            strict: true,
        }
    }
}
