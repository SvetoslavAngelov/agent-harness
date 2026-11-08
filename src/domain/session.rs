//! Holds one conversation's identity and message history.

use super::ids::SessionId;
use super::message::{Message, Role};

#[derive(Debug)]
pub struct Session {
    pub session_id: SessionId,
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            session_id: SessionId::generate(),
            messages: Vec::new(),
        }
    }

    pub fn append_text(&mut self, role: Role, content: impl Into<String>) {
        self.append_message(Message::new(role, content));
    }

    pub fn append_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn get_message_history(&self) -> &[Message] {
        &self.messages
    }
}
