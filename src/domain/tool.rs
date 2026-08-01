use super::ids::ToolCallId;

struct ToolCall {
    call_id: ToolCallId,
    name: String,
    arguments: json::JsonValue,
}
