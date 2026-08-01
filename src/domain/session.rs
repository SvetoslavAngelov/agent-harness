use chrono::prelude::*;

use super::ids::SessionId;
use super::message::{Message, Role};

#[derive(Debug)]
pub struct Session {
    pub session_id: SessionId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    pub status: RunStatus,
}

impl Session {
    pub fn new() -> Self {
        let session_id = SessionId::generate();
        let now = Utc::now();

        Self {
            session_id,
            created_at: now,
            updated_at: now,
            messages: Vec::with_capacity(4),
            status: RunStatus::Idle,
        }
    }

    pub fn append_message(&mut self, role: Role, content: impl Into<String>) {
        self.messages
            .push(Message::new(self.session_id, role, content));

        self.updated_at = Utc::now();
    }
}

#[derive(Debug)]
pub enum RunStatus {
    Running,
    Cancelling,
    Failed,
    Idle,
}
