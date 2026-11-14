//! Runs one assignment task, stopping on commit or a bounded terminal condition.

use serde_json::json;

use crate::assignment::store::{AssignmentStore, TaskResult};
use crate::client::openai_client::OpenAiClient;
use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::message::{Message, MessagePart, Role};
use crate::domain::session::Session;
use crate::domain::tool_catalogue::{ToolCall, ToolCallErrorKind, ToolError, ToolResult};
use crate::domain::tool_registry::ToolRegistry;
use crate::tools::assignment_tools::register_tools;

pub struct Runner {
    client: OpenAiClient,
    session: Session,
    tool_registry: ToolRegistry,
    max_model_requests: u16,
}

impl Runner {
    pub fn new(
        api_key: String,
        client_config: OpenAiClientConfig,
        mut session: Session,
        max_model_requests: u16,
    ) -> Self {
        let mut tool_registry = ToolRegistry::new();
        register_tools(&mut tool_registry);
        let client = OpenAiClient::new(api_key, client_config, tool_registry.get_catalogue());
        session.append_text(Role::System,
            "Complete the assignment task using the available tools. Everyone means all users in this session. \
             First list assignments to discover the users, current assignments, and supported jobs. \
             Use their IDs to update only assignments that need changing. \
             You may issue multiple tool calls per response; they execute in the order given. \
             When finished, call commit(), placing it after any updates in the same response. \
             A commit ends the session immediately, even if an earlier call failed. \
             Calls after a successful commit are skipped. Text alone does not complete the task.");

        Self {
            client,
            session,
            tool_registry,
            max_model_requests,
        }
    }

    pub fn run(mut self, store: &mut AssignmentStore) -> TaskResult<()> {
        if store.result()?.is_some() {
            return Err("This session has already been finalised".into());
        }
        let result = self.run_task(store);
        if let Err(error) = &result {
            let _ = store.record_event("run_error", json!({ "error": error.to_string() }));
            if let Err(storage_error) = store.finalise("infrastructure_error") {
                eprintln!("Could not finalise the failed run: {storage_error}");
            }
        }
        result
    }

    fn run_task(&mut self, store: &mut AssignmentStore) -> TaskResult<()> {
        self.session.append_text(Role::User, store.instruction());
        store.record_event(
            "tool_definitions",
            json!(
                self.tool_registry
                    .get_catalogue()
                    .iter()
                    .collect::<Vec<_>>()
            ),
        )?;
        for message in &self.session.messages {
            store.record_message(message)?;
        }

        for _ in 0..self.max_model_requests {
            store.record_model_request()?;
            let response = self.client.call(self.session.get_message_history())?;
            let tool_calls = collect_tool_calls(&response);

            for message in response {
                store.record_message(&message)?;
                self.session.append_message(message);
            }

            if tool_calls.is_empty() {
                store.finalise("missing_commit")?;
                return Ok(());
            }
            // Persist every emitted call first. Finalisation rejects any still pending,
            // so calls after commit are included in its frozen accounting but never run.
            let records = tool_calls
                .iter()
                .map(|call| store.record_tool_call(call))
                .collect::<TaskResult<Vec<_>>>()?;
            for (index, (call, record_id)) in tool_calls.iter().zip(&records).enumerate() {
                store.dispatch_tool_call(*record_id)?;
                let (result, error) = match self.tool_registry.execute(call, store) {
                    Ok(value) => (Some(value), None),
                    Err(error) => (None, Some(error)),
                };
                let committed = call.name == "commit" && error.is_none();
                self.record_tool_result(
                    store,
                    *record_id,
                    ToolResult {
                        call_id: call.call_id.clone(),
                        result,
                        error,
                    },
                )?;

                if committed {
                    for (skipped, id) in tool_calls[index + 1..].iter().zip(&records[index + 1..]) {
                        self.record_tool_result(
                            store,
                            *id,
                            ToolResult {
                                call_id: skipped.call_id.clone(),
                                result: None,
                                error: Some(ToolError {
                                    kind: ToolCallErrorKind::Cancelled,
                                    message: "Not executed: an earlier commit ended the session"
                                        .into(),
                                }),
                            },
                        )?;
                    }
                    return Ok(());
                }
            }
        }

        store.finalise("network_limit")?;
        Ok(())
    }

    fn record_tool_result(
        &mut self,
        store: &AssignmentStore,
        id: i64,
        result: ToolResult,
    ) -> TaskResult<()> {
        store.record_tool_result(id, &result)?;
        let message = Message::from_parts(Role::Tool, vec![MessagePart::ToolResult(result)]);
        store.record_message(&message)?;
        self.session.append_message(message);
        Ok(())
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
