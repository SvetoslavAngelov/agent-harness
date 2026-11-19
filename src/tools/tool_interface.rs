use serde_json::Value;

use crate::domain::tool_catalogue::{ToolDefinition, ToolError};

pub enum ToolOutcome {
    Continue(Value),
    /// The tool ended the run. The runner records its output and skips later calls.
    Finished(Value),
}

pub trait Tool<State> {
    fn get_definition(&self) -> ToolDefinition;
    fn execute(&self, arguments: &str, state: &mut State) -> Result<ToolOutcome, ToolError>;
}
