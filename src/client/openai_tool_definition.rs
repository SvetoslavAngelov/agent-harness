use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::tool::ToolDefinition;

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn converts_a_domain_tool_to_the_openai_function_shape() {
        let tool = ToolDefinition::new(
            "get_weather",
            "Get the current weather for a city.",
            json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                },
                "required": ["city"],
                "additionalProperties": false
            }),
        );

        let openai_tool = OpenAiToolDefinition::from(&tool);

        assert_eq!(
            serde_json::to_value(openai_tool).unwrap(),
            json!({
                "type": "function",
                "name": "get_weather",
                "description": "Get the current weather for a city.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    },
                    "required": ["city"],
                    "additionalProperties": false
                },
                "strict": true
            })
        );
    }
}
