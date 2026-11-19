//! Runs model conversations.

pub mod runner;

pub type HarnessResult<T> = Result<T, Box<dyn std::error::Error>>;
