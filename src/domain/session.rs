//! Tracks one conversation's history, timestamps, and execution status.

use chrono::{DateTime, Utc};

use super::ids::SessionId;
use super::message::{Message, Role};

#[derive(Debug)]
pub struct Session {
    pub session_id: SessionId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    pub status: RunStatus,
    pub network_calls: u16,
}

impl Session {
    pub fn new() -> Self {
        let now = Utc::now();

        Self {
            session_id: SessionId::generate(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            status: RunStatus::Idle,
            network_calls: 0,
        }
    }

    pub fn append_text(&mut self, role: Role, content: impl Into<String>) {
        self.messages.push(Message::new(role, content));

        self.updated_at = Utc::now();
    }

    pub fn append_message(&mut self, message: Message) {
        self.messages.push(message);

        self.updated_at = Utc::now();
    }

    pub fn update_network_calls(&mut self) {
        self.network_calls += 1;
    }

    pub fn get_message_history(&self) -> &[Message] {
        &self.messages
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RunStatus {
    Running,
    Cancelling,
    Failed,
    Idle,
}
