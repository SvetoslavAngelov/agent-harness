PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
    user_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS jobs (
    job_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    task TEXT NOT NULL,
    task_type TEXT NOT NULL,
    seed TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    model_calls INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT
);

CREATE TABLE IF NOT EXISTS initial_assignments (
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    user_id INTEGER NOT NULL REFERENCES users(user_id),
    job_id INTEGER NOT NULL REFERENCES jobs(job_id),
    position INTEGER NOT NULL,
    PRIMARY KEY (session_id, user_id),
    UNIQUE (session_id, position)
);

CREATE TABLE IF NOT EXISTS session_assignments (
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    user_id INTEGER NOT NULL REFERENCES users(user_id),
    job_id INTEGER NOT NULL REFERENCES jobs(job_id),
    PRIMARY KEY (session_id, user_id)
);

CREATE TABLE IF NOT EXISTS ground_truth_assignments (
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    user_id INTEGER NOT NULL REFERENCES users(user_id),
    expected_job_id INTEGER NOT NULL REFERENCES jobs(job_id),
    PRIMARY KEY (session_id, user_id)
);

CREATE TABLE IF NOT EXISTS tool_calls (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    model_request INTEGER NOT NULL,
    call_id TEXT NOT NULL,
    name TEXT NOT NULL,
    arguments TEXT NOT NULL,
    status TEXT NOT NULL,
    output TEXT,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT
);

CREATE TABLE IF NOT EXISTS session_events (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    kind TEXT NOT NULL,
    data TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS session_results (
    session_id TEXT PRIMARY KEY REFERENCES sessions(session_id),
    result TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
