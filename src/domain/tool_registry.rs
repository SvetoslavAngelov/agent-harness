use serde_json::Value;
use std::collections::HashMap;

use super::tool_catalogue::{ToolCall, ToolCallErrorKind, ToolCatalogue, ToolError};
use crate::tools::tool_interface::Tool;

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.get_definition().name;
        self.tools.insert(name, Box::new(tool));
    }

    pub fn execute(&self, tool_call: &ToolCall) -> Result<Value, ToolError> {
        let tool = self.tools.get(&tool_call.name).ok_or_else(|| ToolError {
            kind: ToolCallErrorKind::ExecutionFailed,
            message: format!("Unknown tool: {}", tool_call.name),
        })?;

        tool.execute(&tool_call.arguments)
    }

    pub fn get_catalogue(&self) -> ToolCatalogue {
        ToolCatalogue::new(
            self.tools
                .values()
                .map(|tool| tool.get_definition())
                .collect(),
        )
    }
}
