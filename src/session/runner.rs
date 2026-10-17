//! Serialized loop that alternates model calls and tool execution.

use crate::client::client_error::ClientError;
use crate::client::openai_client::OpenAiClient;
use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::message::{Message, MessagePart, Role};
use crate::domain::session::{RunStatus, Session};
use crate::domain::tool_catalogue::{ToolCall, ToolResult};
use crate::domain::tool_registry::ToolRegistry;

pub struct Runner {
    client: OpenAiClient,
    session: Session,
    tool_registry: ToolRegistry,
    max_network_calls: u16,
}

impl Runner {
    pub fn new(
        api_key: String,
        client_config: OpenAiClientConfig,
        tool_registry: ToolRegistry,
        max_network_calls: u16,
    ) -> Self {
        let client = OpenAiClient::new(api_key, client_config, tool_registry.get_catalogue());
        let mut session = Session::new();
        session.append_text(Role::System, "You're a helpful assistant. Please aid the user solve their task with the tools available at your disposal.");

        Self {
            client,
            session,
            tool_registry,
            max_network_calls,
        }
    }

    pub fn run(&mut self, content: impl Into<String>) -> Result<Vec<Message>, ClientError> {
        self.session.append_text(Role::User, content);
        self.session.status = RunStatus::Running;

        let result = self.run_loop();
        self.session.status = if result.is_ok() {
            RunStatus::Idle
        } else {
            RunStatus::Failed
        };

        result
    }

    fn run_loop(&mut self) -> Result<Vec<Message>, ClientError> {
        loop {
            self.session.update_network_calls();

            if self.session.network_calls > self.max_network_calls {
                return Err(ClientError::InvalidInput(format!(
                    "Max network calls reached {}, terminating run.\n",
                    self.max_network_calls,
                )));
            }

            let response = self.client.call(self.session.get_message_history())?;
            let tool_calls = collect_tool_calls(&response);

            for message in response.iter().cloned() {
                self.session.append_message(message);
            }

            if tool_calls.is_empty() {
                return Ok(response);
            }

            let results = tool_calls
                .into_iter()
                .map(|call| {
                    let (result, error) = match self.tool_registry.execute(&call) {
                        Ok(value) => (Some(value), None),
                        Err(error) => (None, Some(error)),
                    };

                    MessagePart::ToolResult(ToolResult {
                        call_id: call.call_id,
                        result,
                        error,
                    })
                })
                .collect();

            self.session
                .append_message(Message::from_parts(Role::Tool, results));
        }
    }

    pub fn get_message_history(&self) -> &[Message] {
        self.session.get_message_history()
    }
}

fn collect_tool_calls(messages: &[Message]) -> Vec<ToolCall> {
    messages
        .iter()
        .flat_map(|message| &message.parts)
        .filter_map(|part| match part {
            MessagePart::ToolCall(call) => Some(call.clone()),
            _ => None,
        })
        .collect()
}
