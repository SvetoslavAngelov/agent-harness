//! Converts provider-neutral message history into Responses API input items.

use serde_json::{Value, json};

use crate::domain::message::{Message, MessagePart, Role};
use crate::domain::tool_catalogue::ToolResult;

pub(super) fn messages_to_openai_json(messages: &[Message]) -> Result<Vec<Value>, String> {
    let mut input = Vec::new();

    for message in messages {
        input.extend(message_to_openai_items(message)?);
    }

    Ok(input)
}

fn message_to_openai_items(message: &Message) -> Result<Vec<Value>, String> {
    let mut items = Vec::new();

    for part in &message.parts {
        match part {
            MessagePart::Text(text) | MessagePart::Refusal(text) => {
                let role = message_role_name(message.role)?;
                items.push(json!({
                    "role": role,
                    "content": text,
                }));
            }
            MessagePart::ToolCall(call) => {
                items.push(json!({
                    "type": "function_call",
                    "call_id": call.call_id.to_string(),
                    "name": call.name,
                    "arguments": call.arguments,
                }));
            }
            MessagePart::ToolResult(result) => {
                items.push(json!({
                    "type": "function_call_output",
                    "call_id": result.call_id.to_string(),
                    "output": tool_result_to_string(result)?,
                }));
            }
            MessagePart::Opaque(item) => items.push(item.clone()),
        }
    }

    if items.is_empty() {
        return Err(format!(
            "message {} does not contain any parts",
            message.message_id
        ));
    }

    Ok(items)
}

fn message_role_name(role: Role) -> Result<&'static str, String> {
    match role {
        Role::System => Ok("system"),
        Role::User => Ok("user"),
        Role::Assistant => Ok("assistant"),
        Role::Tool => Err("tool messages cannot contain text parts".into()),
    }
}

fn tool_result_to_string(result: &ToolResult) -> Result<String, String> {
    match (&result.result, &result.error) {
        (Some(Value::String(text)), None) => Ok(text.clone()),
        (Some(value), None) => Ok(value.to_string()),
        (None, Some(error)) => Ok(json!({ "error": error }).to_string()),
        (Some(_), Some(_)) => Err(format!(
            "tool result {} contains both a value and an error",
            result.call_id
        )),
        (None, None) => Err(format!(
            "tool result {} contains neither a value nor an error",
            result.call_id
        )),
    }
}
