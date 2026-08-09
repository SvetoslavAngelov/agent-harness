use chrono::prelude::*;

use super::ids::{MessageId, SessionId};
use super::tool::{ToolCall, ToolCatalogue, ToolResult};

#[derive(Debug, Clone)]
pub struct Message {
    pub message_id: MessageId,
    pub session_id: SessionId,
    pub role: Role,
    pub content: String,
    pub tool_catalogue: Option<ToolCatalogue>,
    pub tool_call: Option<ToolCall>,
    pub tool_result: Option<ToolResult>,
    pub timestamp: DateTime<Utc>,
}

impl Message {
    pub fn new(session_id: SessionId, role: Role, content: impl Into<String>) -> Self {
        Self {
            message_id: MessageId::generate(),
            session_id,
            role,
            content: content.into(),
            tool_catalogue: None,
            tool_call: None,
            tool_result: None,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}
