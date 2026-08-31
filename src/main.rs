//! Demonstrates the harness's domain model with a single in-memory session.

mod client;
mod domain;
mod session;
mod tools;

use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::tool_registry::ToolRegistry;
use crate::session::runner::Runner;
use crate::tools::weather_tool::WeatherCheck;

fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set");

    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(WeatherCheck);

    let mut runner = Runner::new(
        api_key,
        OpenAiClientConfig::new("gpt-5.6-luna".into()),
        tool_registry.get_catalogue(),
    );

    let response = runner
        .run("What is the weather in Oxford, UK?")
        .expect("OpenAI API request failed.");

    println!("{response:#?}");

    for message in runner.get_message_history() {
        println!("{message:#?}");
    }
}
