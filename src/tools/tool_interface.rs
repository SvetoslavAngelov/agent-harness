use serde_json::Value;

use crate::domain::tool_catalogue::{ToolDefinition, ToolError};

pub trait Tool {
    fn get_definition(&self) -> ToolDefinition;
    fn execute(&self, arguments: &str) -> Result<Value, ToolError>;
}
