use chrono::prelude::*;

use super::ids::{MessageId, SessionId};

#[derive(Debug, Clone)]
pub struct Message {
    pub message_id: MessageId,
    pub session_id: SessionId,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl Message {
    pub fn new(session_id: SessionId, role: Role, content: impl Into<String>) -> Self {
        Self {
            message_id: MessageId::generate(),
            session_id,
            role,
            content: content.into(),
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
