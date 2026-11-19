CREATE TABLE IF NOT EXISTS users (
    user_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS jobs (
    job_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
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

