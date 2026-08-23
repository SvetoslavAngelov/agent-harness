//! Serialized loop that alternates model calls and tool execution.

use crate::client::client_error::ClientError;
use crate::client::openai_client::OpenAiClient;
use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::message::{Message, Role};
use crate::domain::session::Session;
use crate::domain::tool::ToolCatalogue;

pub struct Runner {
    client: OpenAiClient,
    session: Session,
}

impl Runner {
    pub fn new(api_key: String, client_config: OpenAiClientConfig, tools: ToolCatalogue) -> Self {
        return Runner {
            client: OpenAiClient::new(api_key, client_config, tools),
            session: Session::new(),
        };
    }

    pub fn run(&mut self, content: impl Into<String>) -> Result<Vec<Message>, ClientError> {
        let system_prompt = "You're a helpful assistant, aiding the user achieve their task and calling only the tools that are declared in this message.";

        self.session.append_text(Role::System, system_prompt);
        self.session.append_text(Role::User, content);

        let response = self.client.call(self.session.get_message_history())?;

        for message in response.iter().cloned() {
            self.session.append_message(message);
        }

        Ok(response)
    }
}
