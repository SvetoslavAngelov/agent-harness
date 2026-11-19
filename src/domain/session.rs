//! Holds a conversation and any experiment data collected during a run.

use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::{Map, Value};

use super::ids::{SessionId, serialize_display};
use super::message::{Message, Role};
use crate::session::HarnessResult;

#[derive(Debug, Serialize)]
pub struct Session {
    #[serde(serialize_with = "serialize_display")]
    pub session_id: SessionId,
    pub messages: Vec<Message>,
    pub data: Map<String, Value>,
    pub stats: RunStats,
}

#[derive(Debug, Default, Serialize)]
pub struct RunStats {
    pub model_calls: usize,
    pub tool_calls: Vec<(String, String)>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            session_id: SessionId::generate(),
            messages: Vec::new(),
            data: Map::new(),
            stats: RunStats::default(),
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

    pub fn set(&mut self, key: &str, value: impl Serialize) -> HarnessResult<()> {
        self.data.insert(key.into(), serde_json::to_value(value)?);
        Ok(())
    }

    /// Saves the current conversation, counters, and experiment data as one snapshot.
    pub fn save(&self, connection: &Connection) -> HarnessResult<()> {
        connection.execute_batch(include_str!("../session/schema.sql"))?;
        connection.execute(
            "INSERT OR IGNORE INTO sessions (session_id) VALUES (?1)",
            [self.session_id.to_string()],
        )?;
        connection.execute(
            "INSERT INTO session_snapshots (session_id, data) VALUES (?1, ?2)
             ON CONFLICT(session_id) DO UPDATE SET data = excluded.data",
            params![self.session_id.to_string(), serde_json::to_string(self)?],
        )?;
        Ok(())
    }
}
