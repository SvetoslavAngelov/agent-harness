use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Serialize;
use serde_json::{Value, json};

use super::task::{self, Assignment, Job, Task, User};
use crate::domain::session::RunStats;
use crate::session::HarnessResult;

#[derive(Serialize)]
pub(crate) struct RunResult {
    termination_reason: String,
    state_match: bool,
    model_committed: bool,
    success: bool,
    required_updates: usize,
    reference_tool_calls: usize,
    model_calls: usize,
    tool_calls: usize,
    listing_calls: usize,
    update_calls: usize,
    commit_calls: usize,
    noop_update_calls: usize,
    failed_tool_calls: usize,
    rejected_tool_calls: usize,
}

pub(crate) struct AssignmentStore<'db> {
    connection: &'db Connection,
    session_id: String,
    noop_update_calls: usize,
}

pub(crate) fn generate(connection: &Connection, seed: u64) -> HarnessResult<Task> {
    connection.execute_batch(include_str!("schema.sql"))?;
    seed_catalogues(connection)?;

    let users = connection
        .prepare("SELECT user_id, name FROM users ORDER BY user_id")?
        .query_map([], |row| {
            Ok(User {
                user_id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let jobs = load_jobs(connection)?;
    if users.len() < 3 || jobs.len() < 2 {
        return Err("Task generation needs at least three users and two jobs".into());
    }
    Ok(task::generate(users, &jobs, seed))
}

pub(crate) fn seed(connection: &Connection, session_id: &str, task: &Task) -> rusqlite::Result<()> {
    for (position, row) in task.initial.iter().enumerate() {
        connection.execute(
            "INSERT INTO initial_assignments (session_id, user_id, job_id, position)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, row.user_id, row.job_id, position as i64],
        )?;
        connection.execute(
            "INSERT INTO session_assignments (session_id, user_id, job_id)
             VALUES (?1, ?2, ?3)",
            params![session_id, row.user_id, row.job_id],
        )?;
        // Derive ground truth from structured intent, never from task wording.
        connection.execute(
            "INSERT INTO ground_truth_assignments (session_id, user_id, expected_job_id)
             VALUES (?1, ?2, ?3)",
            params![
                session_id,
                row.user_id,
                task.intent.expected_job_id(row.user_id, row.job_id)
            ],
        )?;
    }
    Ok(())
}

impl<'db> AssignmentStore<'db> {
    pub(crate) fn new(connection: &'db Connection, session_id: &str) -> Self {
        Self {
            connection,
            session_id: session_id.into(),
            noop_update_calls: 0,
        }
    }

    pub(super) fn list_assignments(&self) -> HarnessResult<Value> {
        let connection = self.connection;
        let session_id = &self.session_id;
        let assignments = connection
            .prepare(
                "SELECT a.user_id, u.name, a.job_id, j.name
             FROM session_assignments a
             JOIN users u ON u.user_id = a.user_id
             JOIN jobs j ON j.job_id = a.job_id
             JOIN initial_assignments i
               ON i.session_id = a.session_id AND i.user_id = a.user_id
             WHERE a.session_id = ?1 ORDER BY i.position",
            )?
            .query_map([session_id], |row| {
                Ok(Assignment {
                    user_id: row.get(0)?,
                    user_name: row.get(1)?,
                    job_id: row.get(2)?,
                    job_name: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(json!({ "assignments": assignments, "jobs": load_jobs(connection)? }))
    }

    pub(super) fn update_assignment(&mut self, user_id: i64, job_id: i64) -> HarnessResult<Value> {
        let session_id = &self.session_id;
        let transaction =
            Transaction::new_unchecked(self.connection, TransactionBehavior::Immediate)?;
        let current: Option<(i64, String)> = transaction
            .query_row(
                "SELECT a.job_id, u.name FROM session_assignments a
         JOIN users u ON u.user_id = a.user_id
         WHERE a.session_id = ?1 AND a.user_id = ?2",
                params![session_id, user_id],
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
            params![session_id, user_id, job_id],
        )?;
        let changed = previous_job_id != job_id;
        transaction.commit()?;
        self.noop_update_calls += usize::from(!changed);

        Ok(json!(Assignment {
            user_id,
            user_name,
            job_id,
            job_name
        }))
    }

    pub(super) fn grade(&self, reason: &str, stats: &RunStats) -> HarnessResult<RunResult> {
        let connection = self.connection;
        let session_id = &self.session_id;
        // Compare both complete sets, detecting omitted or extra people as well as wrong jobs.
        let actual = connection
            .prepare(
                "SELECT user_id, job_id FROM session_assignments
         WHERE session_id = ?1 ORDER BY user_id",
            )?
            .query_map([session_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let expected = connection
            .prepare(
                "SELECT user_id, expected_job_id FROM ground_truth_assignments
         WHERE session_id = ?1 ORDER BY user_id",
            )?
            .query_map([session_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let required_updates: usize = connection.query_row(
            "SELECT COUNT(*) FROM initial_assignments i
         JOIN ground_truth_assignments g
           ON g.session_id = i.session_id AND g.user_id = i.user_id
         WHERE i.session_id = ?1 AND i.job_id != g.expected_job_id",
            [session_id],
            |row| row.get::<_, i64>(0).map(|count| count as usize),
        )?;
        let calls = &stats.tool_calls;
        let count_calls = |name: &str| {
            calls
                .iter()
                .filter(|(tool, status)| tool == name && status != "rejected")
                .count()
        };
        let count_status = |status: &str| calls.iter().filter(|(_, value)| value == status).count();
        Ok(RunResult {
            termination_reason: reason.into(),
            state_match: actual == expected,
            model_committed: reason == "model_commit",
            success: actual == expected && reason == "model_commit",
            required_updates,
            reference_tool_calls: required_updates + 2,
            model_calls: stats.model_calls,
            tool_calls: calls
                .iter()
                .filter(|(_, status)| status != "rejected")
                .count(),
            listing_calls: count_calls("list_assignments"),
            update_calls: count_calls("update_assignment"),
            commit_calls: count_calls("commit"),
            noop_update_calls: self.noop_update_calls,
            failed_tool_calls: count_status("failed"),
            rejected_tool_calls: count_status("rejected"),
        })
    }
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
