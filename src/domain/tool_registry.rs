use serde_json::Value;
use std::collections::HashMap;

use super::tool_catalogue::{ToolCall, ToolCallErrorKind, ToolCatalogue, ToolError};
use crate::assignment::store::AssignmentStore;
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

    pub fn execute(
        &self,
        tool_call: &ToolCall,
        store: &mut AssignmentStore,
    ) -> Result<Value, ToolError> {
        let tool = self.tools.get(&tool_call.name).ok_or_else(|| ToolError {
            kind: ToolCallErrorKind::ExecutionFailed,
            message: format!("Unknown tool: {}", tool_call.name),
        })?;

        tool.execute(&tool_call.arguments, store)
    }

    pub fn get_catalogue(&self) -> ToolCatalogue {
        let mut definitions: Vec<_> = self
            .tools
            .values()
            .map(|tool| tool.get_definition())
            .collect();
        definitions.sort_by(|left, right| left.name.cmp(&right.name));
        ToolCatalogue::new(definitions)
    }
}
