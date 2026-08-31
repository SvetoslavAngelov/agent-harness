//! Serialized loop that alternates model calls and tool execution.

use crate::client::client_error::ClientError;
use crate::client::openai_client::OpenAiClient;
use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::message::{Message, Role};
use crate::domain::session::Session;
use crate::domain::tool_catalogue::ToolCatalogue;

pub struct Runner {
    client: OpenAiClient,
    session: Session,
}

impl Runner {
    pub fn new(api_key: String, client_config: OpenAiClientConfig, tools: ToolCatalogue) -> Self {
        let mut session = Session::new();
        session.append_text(Role::System, "You're a helpful assistant, aiding the user achieve their task and calling only the declared tools.");

        return Runner {
            client: OpenAiClient::new(api_key, client_config, tools),
            session,
        };
    }

    pub fn run(&mut self, content: impl Into<String>) -> Result<Vec<Message>, ClientError> {
        self.session.append_text(Role::User, content);

        // Task loop: starts with an initial request and completes when the model deems the task complete.
        let response = self.client.call(self.session.get_message_history())?;

        for message in response.iter().cloned() {
            self.session.append_message(message);
        }

        Ok(response)
    }

    pub fn get_message_history(&self) -> &[Message] {
        return self.session.get_message_history();
    }
}
