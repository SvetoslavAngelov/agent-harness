//! Generates a SQLite assignment task and lets the model solve it through tools.

mod assignment;
mod client;
mod domain;
mod session;
mod tools;

use std::path::Path;

use crate::assignment::store::{AssignmentStore, TaskResult};
use crate::client::openai_client_config::OpenAiClientConfig;
use crate::domain::session::Session;
use crate::session::runner::Runner;

const MAX_MODEL_REQUESTS: u16 = 20;

fn main() -> TaskResult<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let database_path =
        std::env::var("ASSIGNMENTS_DB").unwrap_or_else(|_| "assignments.sqlite3".into());
    let seed = match std::env::var("TASK_SEED") {
        Ok(seed) => seed.parse::<u64>()?,
        Err(std::env::VarError::NotPresent) => rand::random(),
        Err(error) => return Err(error.into()),
    };
    let mut config = OpenAiClientConfig::new(
        std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.6-luna".into()),
    );
    if let Ok(uri) = std::env::var("OPENAI_API_URL") {
        config.uri = uri;
    }
    let session = Session::new();
    let mut store = AssignmentStore::create(Path::new(&database_path), session.session_id, seed)?;
    store.record_event(
        "configuration",
        serde_json::json!({
            "model": config.model,
            "max_output_tokens": config.max_output_tokens,
            "reasoning_effort": config.reasoning_effort,
            "parallel_tool_calls": config.parallel_tool_calls,
            "max_model_requests": MAX_MODEL_REQUESTS,
        }),
    )?;

    println!("Database: {database_path}");
    println!("Session: {}", session.session_id);
    println!("Seed: {seed}");
    println!("Task: {}", store.instruction());

    let outcome = Runner::new(api_key, config, session, MAX_MODEL_REQUESTS).run(&mut store);
    if let Some(result) = store.result()? {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    outcome
}
