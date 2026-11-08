use serde_json::Value;

use crate::assignment::store::AssignmentStore;
use crate::domain::tool_catalogue::{ToolDefinition, ToolError};

pub trait Tool {
    fn get_definition(&self) -> ToolDefinition;
    fn execute(&self, arguments: &str, store: &mut AssignmentStore) -> Result<Value, ToolError>;
}
