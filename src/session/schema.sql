CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS session_snapshots (
    session_id TEXT PRIMARY KEY REFERENCES sessions(session_id),
    data TEXT NOT NULL
);
