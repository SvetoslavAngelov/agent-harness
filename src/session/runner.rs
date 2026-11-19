//! Runs a bounded model conversation and dispatches tool calls in order.

use serde_json::json;

use super::HarnessResult;
use crate::client::openai_client::OpenAiClient;
use crate::domain::message::{Message, MessagePart, Role};
use crate::domain::session::Session;
use crate::domain::tool_catalogue::{ToolCallErrorKind, ToolError, ToolResult};
use crate::domain::tool_registry::ToolRegistry;
use crate::tools::tool_interface::ToolOutcome;

pub enum RunOutcome {
    NoToolCalls,
    Finished,
    RequestLimit,
}

pub struct Runner {
    client: OpenAiClient,
    max_model_requests: u16,
}

impl Runner {
    pub fn new(client: OpenAiClient, max_model_requests: u16) -> Self {
        Self {
            client,
            max_model_requests,
        }
    }

    pub fn run<State>(
        &self,
        session: &mut Session,
        tools: &ToolRegistry<State>,
        state: &mut State,
    ) -> HarnessResult<RunOutcome> {
        let catalogue = tools.get_catalogue();
        let config = self.client.config();
        session.set("configuration", json!({
            "model": config.model,
            "max_output_tokens": config.max_output_tokens,
            "request_timeout_seconds": self.client.request_timeout().map(|timeout| timeout.as_secs()),
            "reasoning_effort": config.reasoning_effort,
            "parallel_tool_calls": config.parallel_tool_calls,
            "max_model_requests": self.max_model_requests,
        }))?;
        session.set("tool_definitions", catalogue.iter().collect::<Vec<_>>())?;

        for _ in 0..self.max_model_requests {
            session.stats.model_calls += 1;
            let response = self
                .client
                .call(session.get_message_history(), &catalogue)?;
            let mut calls = Vec::new();
            for message in response {
                calls.extend(message.parts.iter().filter_map(|part| match part {
                    MessagePart::ToolCall(call) => Some(call.clone()),
                    _ => None,
                }));
                session.append_message(message);
            }
            if calls.is_empty() {
                return Ok(RunOutcome::NoToolCalls);
            }

            let mut finished = false;
            for call in calls {
                let output = if finished {
                    Err(ToolError {
                        kind: ToolCallErrorKind::Cancelled,
                        message: "Not executed: an earlier tool ended the session".into(),
                    })
                } else {
                    match tools.execute(&call, state) {
                        Ok(ToolOutcome::Continue(value)) => Ok(value),
                        Ok(ToolOutcome::Finished(value)) => {
                            finished = true;
                            Ok(value)
                        }
                        Err(error) => Err(error),
                    }
                };
                let status = match &output {
                    Err(error) if matches!(error.kind, ToolCallErrorKind::Cancelled) => "rejected",
                    Err(_) => "failed",
                    Ok(_) => "succeeded",
                };
                session.stats.tool_calls.push((call.name, status.into()));
                session.append_message(Message::from_parts(
                    Role::Tool,
                    vec![MessagePart::ToolResult(ToolResult::new(
                        call.call_id,
                        output,
                    ))],
                ));
            }
            if finished {
                return Ok(RunOutcome::Finished);
            }
        }
        Ok(RunOutcome::RequestLimit)
    }
}
