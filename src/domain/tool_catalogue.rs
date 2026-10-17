//! Defines provider-neutral tool schemas, calls, results, and execution errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ids::ToolCallId;

#[derive(Debug, Clone, Default)]
pub struct ToolCatalogue {
    tools: Vec<ToolDefinition>,
}

impl ToolCatalogue {
    pub fn new(tools: Vec<ToolDefinition>) -> Self {
        Self { tools }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ToolDefinition> {
        self.tools.iter()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub call_id: ToolCallId,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub call_id: ToolCallId,
    pub result: Option<Value>,
    pub error: Option<ToolError>,
}

#[derive(Debug, Clone)]
pub struct ToolError {
    pub kind: ToolCallErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ToolCallErrorKind {
    InvalidArguments,
    ExecutionFailed,
    Cancelled,
}
