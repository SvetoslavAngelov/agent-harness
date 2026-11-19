# Roster assignment

The model receives an instruction to change job assignments in a synthetic
three-person roster. It uses `list_assignments` to discover the current roster and
valid job IDs, `update_assignment` to apply individual changes, and `commit` to
finish. A run passes when the model commits with the exact requested final
assignments.

The runner keeps the conversation and call counts in the session wrapper. The
assignment owns the roster tables, supplies the prompts and tools, and grades
the persisted roster after the runner returns. The session, including its grade,
is saved as JSON in SQLite's `session_snapshots` table.

Set `TASK_SEED` to an unsigned 64-bit integer to reproduce a case. The harness
uses a random seed by default. Set `ASSIGNMENTS_DB` to choose where to store runs.
The default is `assignments.sqlite3`.
