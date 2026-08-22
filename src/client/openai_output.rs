//! Normalizes supported Responses API output items into domain messages.

use serde_json::Value;

use crate::domain::ids::ToolCallId;
use crate::domain::message::{Message, MessagePart, Role};
use crate::domain::tool::ToolCall;

pub(super) fn openai_json_to_messages(response: &Value) -> Result<Vec<Message>, String> {
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .ok_or("response is missing status")?;

    if status != "completed" {
        return Err(format!("response finished with status {status}"));
    }

    let output = response
        .get("output")
        .and_then(Value::as_array)
        .ok_or("response is missing output")?;

    let mut parts = Vec::new();

    for item in output {
        match item.get("type").and_then(Value::as_str) {
            Some("message") => {
                collect_message_parts(item, &mut parts)?;
            }

            Some("function_call") => {
                parts.push(MessagePart::ToolCall(ToolCall {
                    call_id: ToolCallId::new(required_string(item, "call_id")?),
                    name: required_string(item, "name")?,
                    arguments: required_string(item, "arguments")?,
                }));
            }
            Some(_) => {}
            None => return Err("output item is missing type".into()),
        }
    }

    if parts.is_empty() {
        return Err("response did not contain supported output".into());
    }

    Ok(vec![Message::from_parts(Role::Assistant, parts)])
}

fn collect_message_parts(item: &Value, parts: &mut Vec<MessagePart>) -> Result<(), String> {
    let content = item
        .get("content")
        .and_then(Value::as_array)
        .ok_or("message output is missing content")?;

    for part in content {
        match part.get("type").and_then(Value::as_str) {
            Some("output_text") => parts.push(MessagePart::Text(required_string(part, "text")?)),
            Some("refusal") => {
                parts.push(MessagePart::Refusal(required_string(part, "refusal")?));
            }
            Some(_) => {}
            None => return Err("content part is missing type".into()),
        }
    }

    Ok(())
}

fn required_string(object: &Value, field: &str) -> Result<String, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing string field {field}"))
}
