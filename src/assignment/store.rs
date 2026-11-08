use std::{error::Error, path::Path};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::task::{self, Assignment, Job, User};
use crate::domain::{
    ids::SessionId,
    message::Message,
    tool_catalogue::{ToolCall, ToolResult},
};

pub type TaskResult<T> = Result<T, Box<dyn Error>>;

/// Owns one session's database connection. Tools cannot choose a session ID.
pub struct AssignmentStore {
    connection: Connection,
    session_id: String,
    instruction: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub termination_reason: String,
    pub state_match: bool,
    pub model_committed: bool,
    pub success: bool,
    pub required_updates: usize,
    pub reference_tool_calls: usize,
    pub model_calls: usize,
    pub tool_calls: usize,
    pub listing_calls: usize,
    pub update_calls: usize,
    pub commit_calls: usize,
    pub noop_update_calls: usize,
    pub failed_tool_calls: usize,
    pub rejected_tool_calls: usize,
}

impl AssignmentStore {
    pub fn create(path: &Path, session_id: SessionId, seed: u64) -> TaskResult<Self> {
        let mut connection = Connection::open(path)?;
        connection.execute_batch(include_str!("schema.sql"))?;
        // Keep databases created by the earlier prototype usable.
        let has_task_parameters: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('sessions') WHERE name = 'task_parameters')",
            [],
            |row| row.get(0),
        )?;
        if has_task_parameters {
            connection.execute("ALTER TABLE sessions DROP COLUMN task_parameters", [])?;
        }
        let transaction = connection.transaction()?;
        seed_catalogues(&transaction)?;

        let users = transaction
            .prepare("SELECT user_id, name FROM users ORDER BY user_id")?
            .query_map([], |row| {
                Ok(User {
                    user_id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let jobs = load_jobs(&transaction)?;
        if users.len() < 3 || jobs.len() < 2 {
            return Err("Task generation needs at least three users and two jobs".into());
        }
        let task = task::generate(users, &jobs, seed);
        let session_id = session_id.to_string();

        transaction.execute(
            "INSERT INTO sessions (session_id, task, task_type, seed)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                session_id,
                task.instruction,
                task.task_type,
                seed.to_string()
            ],
        )?;
        for (position, row) in task.initial.iter().enumerate() {
            transaction.execute(
                "INSERT INTO initial_assignments (session_id, user_id, job_id, position)
                 VALUES (?1, ?2, ?3, ?4)",
                params![session_id, row.user_id, row.job_id, position as i64],
            )?;
            transaction.execute(
                "INSERT INTO session_assignments (session_id, user_id, job_id)
                 VALUES (?1, ?2, ?3)",
                params![session_id, row.user_id, row.job_id],
            )?;
            // Derive ground truth from structured intent, never from task wording.
            transaction.execute(
                "INSERT INTO ground_truth_assignments (session_id, user_id, expected_job_id)
                 VALUES (?1, ?2, ?3)",
                params![
                    session_id,
                    row.user_id,
                    task.intent.expected_job_id(row.user_id, row.job_id)
                ],
            )?;
        }
        transaction.commit()?;

        Ok(Self {
            connection,
            session_id,
            instruction: task.instruction,
        })
    }

    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    pub fn list_assignments(&self) -> TaskResult<Value> {
        let assignments = self
            .connection
            .prepare(
                "SELECT a.user_id, u.name, a.job_id, j.name
                 FROM session_assignments a
                 JOIN users u ON u.user_id = a.user_id
                 JOIN jobs j ON j.job_id = a.job_id
                 JOIN initial_assignments i
                   ON i.session_id = a.session_id AND i.user_id = a.user_id
                 WHERE a.session_id = ?1 ORDER BY i.position",
            )?
            .query_map([&self.session_id], |row| {
                Ok(Assignment {
                    user_id: row.get(0)?,
                    user_name: row.get(1)?,
                    job_id: row.get(2)?,
                    job_name: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(json!({ "assignments": assignments, "jobs": load_jobs(&self.connection)? }))
    }

    pub fn update_assignment(&mut self, user_id: i64, job_id: i64) -> TaskResult<Value> {
        let transaction = self.connection.transaction()?;
        let active: bool = transaction.query_row(
            "SELECT status = 'running' FROM sessions WHERE session_id = ?1",
            [&self.session_id],
            |row| row.get(0),
        )?;
        if !active {
            return Err("This session has already been finalised".into());
        }
        let current: Option<(i64, String)> = transaction
            .query_row(
                "SELECT a.job_id, u.name FROM session_assignments a
             JOIN users u ON u.user_id = a.user_id
             WHERE a.session_id = ?1 AND a.user_id = ?2",
                params![self.session_id, user_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (previous_job_id, user_name) = current.ok_or("Unknown user in this session")?;
        let job_name: String = transaction
            .query_row("SELECT name FROM jobs WHERE job_id = ?1", [job_id], |row| {
                row.get(0)
            })
            .optional()?
            .ok_or("Unknown job ID")?;

        transaction.execute(
            "UPDATE session_assignments SET job_id = ?3
             WHERE session_id = ?1 AND user_id = ?2",
            params![self.session_id, user_id, job_id],
        )?;
        insert_event(
            &transaction,
            &self.session_id,
            "assignment_updated",
            &json!({
                "user_id": user_id, "previous_job_id": previous_job_id, "job_id": job_id,
                "changed": previous_job_id != job_id,
            }),
        )?;
        transaction.commit()?;

        Ok(json!(Assignment {
            user_id,
            user_name,
            job_id,
            job_name
        }))
    }

    pub fn record_event(&self, kind: &str, data: Value) -> TaskResult<()> {
        insert_event(&self.connection, &self.session_id, kind, &data)?;
        Ok(())
    }

    pub fn record_message(&self, message: &Message) -> TaskResult<()> {
        self.record_event(
            "message",
            json!({
                "message_id": message.message_id.to_string(),
                "role": message.role,
                "parts": message.parts,
                "timestamp": message.timestamp.to_rfc3339(),
            }),
        )
    }

    pub fn record_model_request(&mut self) -> TaskResult<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE sessions SET model_calls = model_calls + 1 WHERE session_id = ?1",
            [&self.session_id],
        )?;
        insert_event(
            &transaction,
            &self.session_id,
            "model_requested",
            &json!({}),
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Record before dispatch so the terminal commit counts in its own result.
    pub fn record_tool_call(&self, call: &ToolCall, rejected: bool) -> TaskResult<i64> {
        self.connection.execute(
            "INSERT INTO tool_calls
             (session_id, model_request, call_id, name, arguments, status)
             VALUES (?1, (SELECT model_calls FROM sessions WHERE session_id = ?1),
                     ?2, ?3, ?4, ?5)",
            params![
                self.session_id,
                call.call_id.to_string(),
                call.name,
                call.arguments,
                if rejected { "rejected" } else { "dispatched" }
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn record_tool_result(&self, id: i64, result: &ToolResult) -> TaskResult<()> {
        self.connection.execute(
            "UPDATE tool_calls SET status = ?2, output = ?3, error = ?4,
             finished_at = CURRENT_TIMESTAMP WHERE id = ?1 AND session_id = ?5",
            params![
                id,
                if result.error.is_some() {
                    "failed"
                } else {
                    "succeeded"
                },
                result.result.as_ref().map(Value::to_string),
                result
                    .error
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?,
                self.session_id
            ],
        )?;
        Ok(())
    }

    pub fn result(&self) -> TaskResult<Option<RunResult>> {
        let result: Option<String> = self
            .connection
            .query_row(
                "SELECT result FROM session_results WHERE session_id = ?1",
                [&self.session_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result
            .map(|result| serde_json::from_str(&result))
            .transpose()?)
    }

    /// Freeze and grade persisted state once. A matching table still needs a model commit.
    pub fn finalise(&mut self, reason: &str) -> TaskResult<RunResult> {
        let transaction = self.connection.transaction()?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT result FROM session_results WHERE session_id = ?1",
                [&self.session_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return Ok(serde_json::from_str(&existing)?);
        }

        // Compare both complete sets, detecting omitted or extra people as well as wrong jobs.
        let actual = transaction
            .prepare(
                "SELECT user_id, job_id FROM session_assignments
             WHERE session_id = ?1 ORDER BY user_id",
            )?
            .query_map([&self.session_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let expected = transaction
            .prepare(
                "SELECT user_id, expected_job_id FROM ground_truth_assignments
             WHERE session_id = ?1 ORDER BY user_id",
            )?
            .query_map([&self.session_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let required_updates: usize = transaction.query_row(
            "SELECT COUNT(*) FROM initial_assignments i
             JOIN ground_truth_assignments g
               ON g.session_id = i.session_id AND g.user_id = i.user_id
             WHERE i.session_id = ?1 AND i.job_id != g.expected_job_id",
            [&self.session_id],
            |row| row.get::<_, i64>(0).map(|count| count as usize),
        )?;
        let calls = transaction
            .prepare("SELECT name, status FROM tool_calls WHERE session_id = ?1 ORDER BY id")?
            .query_map([&self.session_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let count_calls = |name: &str| {
            calls
                .iter()
                .filter(|(tool, status)| tool == name && status != "rejected")
                .count()
        };
        let result = RunResult {
            termination_reason: reason.into(),
            state_match: actual == expected,
            model_committed: reason == "model_commit",
            success: actual == expected && reason == "model_commit",
            required_updates,
            reference_tool_calls: required_updates + 2,
            model_calls: transaction.query_row(
                "SELECT model_calls FROM sessions WHERE session_id = ?1",
                [&self.session_id],
                |row| row.get::<_, i64>(0).map(|count| count as usize),
            )?,
            tool_calls: calls
                .iter()
                .filter(|(_, status)| status != "rejected")
                .count(),
            listing_calls: count_calls("list_assignments"),
            update_calls: count_calls("update_assignment"),
            commit_calls: count_calls("commit"),
            noop_update_calls: transaction.query_row(
                "SELECT COUNT(*) FROM session_events WHERE session_id = ?1
                 AND kind = 'assignment_updated' AND json_extract(data, '$.changed') = 0",
                [&self.session_id],
                |row| row.get::<_, i64>(0).map(|count| count as usize),
            )?,
            failed_tool_calls: calls
                .iter()
                .filter(|(_, status)| status == "failed")
                .count(),
            rejected_tool_calls: calls
                .iter()
                .filter(|(_, status)| status == "rejected")
                .count(),
        };

        transaction.execute(
            "INSERT INTO session_results (session_id, result) VALUES (?1, ?2)",
            params![self.session_id, serde_json::to_string(&result)?],
        )?;
        transaction.execute(
            "UPDATE sessions SET status = 'finalised', finished_at = CURRENT_TIMESTAMP
             WHERE session_id = ?1",
            [&self.session_id],
        )?;
        insert_event(&transaction, &self.session_id, "finalised", &json!(result))?;
        transaction.commit()?;
        Ok(result)
    }
}

fn insert_event(
    connection: &Connection,
    session_id: &str,
    kind: &str,
    data: &Value,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO session_events (session_id, kind, data) VALUES (?1, ?2, ?3)",
        params![session_id, kind, data.to_string()],
    )?;
    Ok(())
}

fn load_jobs(connection: &Connection) -> rusqlite::Result<Vec<Job>> {
    connection
        .prepare("SELECT job_id, name FROM jobs ORDER BY job_id")?
        .query_map([], |row| {
            Ok(Job {
                job_id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect()
}

fn seed_catalogues(connection: &Connection) -> rusqlite::Result<()> {
    let first_names = [
        "Alex", "Sam", "Lee", "Morgan", "Robin", "Jamie", "Taylor", "Casey", "Jordan", "Riley",
    ];
    let last_names = [
        "Adams", "Baker", "Clark", "Davis", "Evans", "Green", "Hall", "Jones", "King", "Lewis",
    ];
    for (first_index, first) in first_names.iter().enumerate() {
        for (last_index, last) in last_names.iter().enumerate() {
            connection.execute(
                "INSERT OR IGNORE INTO users (user_id, name) VALUES (?1, ?2)",
                params![
                    (first_index * last_names.len() + last_index + 1) as i64,
                    format!("{first} {last}")
                ],
            )?;
        }
    }
    let jobs = [
        "maintenance",
        "inspection",
        "cleaning",
        "security",
        "delivery",
        "inventory",
        "packing",
        "reception",
        "gardening",
        "support",
    ];
    for (index, name) in jobs.iter().enumerate() {
        connection.execute(
            "INSERT OR IGNORE INTO jobs (job_id, name) VALUES (?1, ?2)",
            params![(index + 1) as i64, name],
        )?;
    }
    Ok(())
}
