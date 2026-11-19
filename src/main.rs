//! Runs an assignment experiment with an OpenAI-compatible model.

mod client;
mod domain;
mod session;
mod tasks;
mod tools;

use std::env;

use crate::client::openai_client::OpenAiClient;
use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::session::Session;
use crate::session::{HarnessResult, runner::Runner};
use crate::tasks::assignment;

fn main() -> HarnessResult<()> {
    let client = OpenAiClient::new(env::var("OPENAI_API_KEY")?, OpenAiClientConfig::from_env());
    let runner = Runner::new(client, 20);
    let mut session = Session::new();
    assignment::run(&runner, &mut session)
}
