/*use std::collections::HashMap;

use crate::domain::ids::SessionId;
use crate::domain::message::{Message, Role};
use crate::domain::session::Session;

//use super::session_runner::SessionRunner;

// TODO add thread safety
#[derive(Debug)]
pub struct SessionManager {
    pub sessions: HashMap<SessionId, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(&mut self) -> SessionId {
        let new_session = Session::new();
        let id = new_session.session_id.clone();
        self.sessions.insert(new_session.session_id, new_session);

        return id;
    }

    // Record a new message to an existing Session.
    // Returns a const reference to the Session if successful,
    // else returns None.
    pub fn record_message(&mut self, session_id: SessionId, message: Message) {}

    pub fn try_get_session_history(self, session_id: SessionId) -> Option<&Vec<Message>> {
        let session_history = self.sessions.get(&session_id).get_message_history();
        return session_history;
    }

    // expose status
}
*/
