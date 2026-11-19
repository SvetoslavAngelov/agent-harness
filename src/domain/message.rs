//! Models conversation messages as ordered text, refusal, and tool-related parts.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use super::ids::{MessageId, serialize_display};
use super::tool_catalogue::{ToolCall, ToolResult};

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    #[serde(serialize_with = "serialize_display")]
    pub message_id: MessageId,
    pub role: Role,
    pub parts: Vec<MessagePart>,
    #[serde(serialize_with = "serialize_display")]
    pub timestamp: DateTime<Utc>,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            message_id: MessageId::generate(),
            role,
            parts: vec![MessagePart::Text(content.into())],
            timestamp: Utc::now(),
        }
    }

    pub fn from_parts(role: Role, parts: Vec<MessagePart>) -> Self {
        Self {
            message_id: MessageId::generate(),
            role,
            parts,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum MessagePart {
    Text(String),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    Refusal(String),
    /// Provider state that must be replayed unchanged on the next model call.
    Opaque(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}
