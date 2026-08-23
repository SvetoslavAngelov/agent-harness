//! Demonstrates the harness's domain model with a single in-memory session.

mod client;
mod domain;
mod session;

use serde_json::json;

use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::tool::{ToolCatalogue, ToolDefinition};
use crate::session::runner::Runner;

fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set");

    let tool_input_schema = json!({
        "type": "object",
        "properties": {
            "city": {
                "type": "string"
            },
            "country": {
                "type": "string"
            }
        },
        "required": ["city", "country"],
        "additionalProperties": false
    });

    let tool_catalogue = ToolCatalogue::new(vec![ToolDefinition::new(
        "weather_check",
        "Query this function to get the latest weather forecast for any city, or country.",
        tool_input_schema,
    )]);

    let client_config = OpenAiClientConfig::new("gpt-5.6-luna".into());

    let mut runner = Runner::new(api_key, client_config, tool_catalogue);
    let response = runner
        .run("What is the weather in Oxford, UK?")
        .expect("OpenAI API request failed.");

    println!("{response:#?}");
}
