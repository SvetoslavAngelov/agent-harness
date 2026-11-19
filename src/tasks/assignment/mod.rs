//! Provides the setup and tools for a roster assignment task backed by SQLite.

mod store;
mod task;
mod tools;

use std::env;

use rusqlite::Connection;
use serde_json::json;

use self::store::AssignmentStore;
use crate::domain::{
    message::Role,
    session::{RunStats, Session},
    tool_registry::ToolRegistry,
};
use crate::session::{
    HarnessResult,
    runner::{RunOutcome, Runner},
};

pub fn run(runner: &Runner, session: &mut Session) -> HarnessResult<()> {
    let database_path =
        env::var_os("ASSIGNMENTS_DB").unwrap_or_else(|| "assignments.sqlite3".into());
    let mut connection = Connection::open(database_path)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    let seed = seed_from_env()?;
    let transaction = connection.transaction()?;
    let task = store::generate(&transaction, seed)?;
    session.append_text(Role::System, system_prompt());
    session.append_text(Role::User, &task.instruction);
    session.set(
        "task",
        json!({
            "instruction": task.instruction,
            "type": task.task_type,
            "seed": seed,
        }),
    )?;
    session.save(&transaction)?;
    store::seed(&transaction, &session.session_id.to_string(), &task)?;
    transaction.commit()?;

    let mut assignments = AssignmentStore::new(&connection, &session.session_id.to_string());
    let mut tools = ToolRegistry::new();
    tools::register_tools(&mut tools);
    let outcome = runner.run(session, &tools, &mut assignments);
    let result = grade(&assignments, &outcome, &session.stats)?;
    session.set("result", &result)?;
    if let Err(error) = &outcome {
        session.set("error", error.to_string())?;
    }
    session.save(&connection)?;
    println!("Session {}", session.session_id);
    println!("{}", serde_json::to_string_pretty(&result)?);
    outcome.map(|_| ())
}

fn system_prompt() -> &'static str {
    "Complete the assignment task using the available tools. Everyone means all users in this session. \
        First list assignments to discover the users, current assignments, and supported jobs. \
        Use their IDs to update only assignments that need changing. \
        You may issue multiple tool calls per response; they execute in the order given. \
        When finished, call commit(), placing it after any updates in the same response. \
        A commit ends the session immediately, even if an earlier call failed. \
        Calls after a successful commit are skipped. Text alone does not complete the task."
}

fn seed_from_env() -> HarnessResult<u64> {
    match env::var("TASK_SEED") {
        Ok(seed) => Ok(seed.parse()?),
        Err(env::VarError::NotPresent) => Ok(rand::random()),
        Err(error) => Err(error.into()),
    }
}

fn grade(
    assignments: &AssignmentStore<'_>,
    outcome: &HarnessResult<RunOutcome>,
    stats: &RunStats,
) -> HarnessResult<serde_json::Value> {
    let reason = match outcome {
        Ok(RunOutcome::Finished) => "model_commit",
        Ok(RunOutcome::NoToolCalls) => "missing_commit",
        Ok(RunOutcome::RequestLimit) => "network_limit",
        Err(_) => "infrastructure_error",
    };
    Ok(serde_json::to_value(assignments.grade(reason, stats)?)?)
}
