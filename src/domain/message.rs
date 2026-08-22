//! Models conversation messages as ordered text, refusal, and tool-related parts.

use chrono::{DateTime, Utc};

use super::ids::MessageId;
use super::tool::{ToolCall, ToolResult};

#[derive(Debug, Clone)]
pub struct Message {
    pub message_id: MessageId,
    pub role: Role,
    pub parts: Vec<MessagePart>,
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

#[derive(Debug, Clone)]
pub enum MessagePart {
    Text(String),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    Refusal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}
