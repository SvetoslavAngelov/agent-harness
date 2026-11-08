use serde::Deserialize;
use serde_json::{Value, json};

use super::tool_interface::Tool;
use crate::{
    assignment::store::AssignmentStore,
    domain::{
        tool_catalogue::{ToolCallErrorKind, ToolDefinition, ToolError},
        tool_registry::ToolRegistry,
    },
};

pub fn register_tools(registry: &mut ToolRegistry) {
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

impl Tool for AssignmentTool {
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

    fn execute(&self, arguments: &str, store: &mut AssignmentStore) -> Result<Value, ToolError> {
        let result = match self {
            Self::List => {
                parse_arguments::<NoArguments>(arguments)?;
                store.list_assignments()
            }
            Self::Update => {
                let arguments: UpdateArguments = parse_arguments(arguments)?;
                store.update_assignment(arguments.user_id, arguments.job_id)
            }
            Self::Commit => {
                parse_arguments::<NoArguments>(arguments)?;
                store
                    .finalise("model_commit")
                    .map(|_| json!({ "finalised": true }))
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
