//! Sends domain messages to the OpenAI Responses API and normalizes its output.

use std::time::Duration;

use serde_json::{Value, json};

use super::client_error::ClientError;
use super::openai_client_config::OpenAiClientConfig;
use super::openai_input::messages_to_openai_json;
use super::openai_output::openai_json_to_messages;
use super::openai_tool_definition::OpenAiToolDefinition;
use crate::domain::message::Message;
use crate::domain::tool_catalogue::ToolCatalogue;

#[derive(Clone)]
pub struct OpenAiClient {
    api_key: String,
    config: OpenAiClientConfig,
    http: ureq::Agent,
    tools: Vec<OpenAiToolDefinition>,
}

impl OpenAiClient {
    pub fn new(api_key: String, config: OpenAiClientConfig, tool_catalogue: ToolCatalogue) -> Self {
        let http_config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .build();

        let tools = tool_catalogue
            .iter()
            .map(OpenAiToolDefinition::from)
            .collect();

        Self {
            api_key,
            config,
            http: ureq::Agent::new_with_config(http_config),
            tools,
        }
    }

    pub fn call(&self, messages: &[Message]) -> Result<Vec<Message>, ClientError> {
        let request = self.build_request(messages)?;
        let authorization = format!("Bearer {}", self.api_key);

        let mut response = self
            .http
            .post(self.config.uri.as_str())
            .header("Authorization", &authorization)
            .send_json(&request)
            .map_err(ClientError::Http)?;

        let status = response.status();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(ClientError::Http)?;

        if !status.is_success() {
            return Err(ClientError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let response_json: Value = serde_json::from_str(&body)
            .map_err(|error| ClientError::InvalidResponse(error.to_string()))?;

        openai_json_to_messages(&response_json).map_err(ClientError::InvalidResponse)
    }

    fn build_request(&self, messages: &[Message]) -> Result<Value, ClientError> {
        let input = messages_to_openai_json(messages).map_err(ClientError::InvalidInput)?;

        Ok(json!({
            "model": self.config.model,
            "input": input,
            "max_output_tokens": self.config.max_output_tokens,
            "reasoning": {
                "effort": self.config.reasoning_effort,
            },
            "tools": self.tools,
            "parallel_tool_calls": self.config.parallel_tool_calls,
        }))
    }
}
