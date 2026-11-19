//! Defines the tools for the roster assignment task.

use serde::Deserialize;
use serde_json::{Value, json};

use super::store::AssignmentStore;
use crate::{
    domain::{
        tool_catalogue::{ToolCallErrorKind, ToolDefinition, ToolError},
        tool_registry::ToolRegistry,
    },
    tools::tool_interface::{Tool, ToolOutcome},
};

pub(crate) fn register_tools(registry: &mut ToolRegistry<AssignmentStore<'_>>) {
    for tool in [
        AssignmentTool::List,
        AssignmentTool::Update,
        AssignmentTool::Commit,
    ] {
        registry.register(tool);
    }
}

enum AssignmentTool {
    List,
    Update,
    Commit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArguments {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateArguments {
    user_id: i64,
    job_id: i64,
}

impl Tool<AssignmentStore<'_>> for AssignmentTool {
    fn get_definition(&self) -> ToolDefinition {
        let empty_schema = json!({
            "type": "object", "properties": {}, "required": [], "additionalProperties": false,
        });
        match self {
            Self::List => ToolDefinition::new(
                "list_assignments",
                "List every user and their current job in this session, and all supported jobs with their IDs.",
                empty_schema,
            ),
            Self::Update => ToolDefinition::new(
                "update_assignment",
                "Set one session user's job using user_id and job_id from the listing. Other users are unchanged.",
                json!({
                    "type": "object",
                    "properties": {
                        "user_id": { "type": "integer" },
                        "job_id": { "type": "integer" },
                    },
                    "required": ["user_id", "job_id"],
                    "additionalProperties": false,
                }),
            ),
            Self::Commit => ToolDefinition::new(
                "commit",
                "Finish the task when all requested assignments are complete. This ends the session immediately; no further changes are possible.",
                empty_schema,
            ),
        }
    }

    fn execute(
        &self,
        arguments: &str,
        assignments: &mut AssignmentStore<'_>,
    ) -> Result<ToolOutcome, ToolError> {
        let result = match self {
            Self::List => {
                parse_arguments::<NoArguments>(arguments)?;
                assignments.list_assignments().map(ToolOutcome::Continue)
            }
            Self::Update => {
                let arguments: UpdateArguments = parse_arguments(arguments)?;
                assignments
                    .update_assignment(arguments.user_id, arguments.job_id)
                    .map(ToolOutcome::Continue)
            }
            Self::Commit => {
                parse_arguments::<NoArguments>(arguments)?;
                return Ok(ToolOutcome::Finished(json!({ "finalised": true })));
            }
        };
        result.map_err(|error| ToolError {
            kind: ToolCallErrorKind::ExecutionFailed,
            message: error.to_string(),
        })
    }
}

fn parse_arguments<T: serde::de::DeserializeOwned>(arguments: &str) -> Result<T, ToolError> {
    let value: Value = serde_json::from_str(arguments).map_err(invalid_arguments)?;
    if !value.is_object() {
        return Err(ToolError {
            kind: ToolCallErrorKind::InvalidArguments,
            message: "Tool arguments must be a JSON object".into(),
        });
    }
    serde_json::from_value(value).map_err(invalid_arguments)
}

fn invalid_arguments(error: serde_json::Error) -> ToolError {
    ToolError {
        kind: ToolCallErrorKind::InvalidArguments,
        message: error.to_string(),
    }
}
