# Assignment tool harness

A small Rust harness that generates a job-assignment task in SQLite, lets a model
solve it through three tools, and compares the final database state with saved
ground truth.

## Run

Set `OPENAI_API_KEY` in your shell, then run:

```sh
cargo run
```

The app does not load `.env` files automatically. Optional environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `ASSIGNMENTS_DB` | `assignments.sqlite3` | SQLite file; its parent directory must exist |
| `TASK_SEED` | Random `u64` | Reproduce a task with the same catalogues and dependency versions |
| `OPENAI_MODEL` | `gpt-5.6-luna` | Model identifier |
| `OPENAI_API_URL` | `https://api.openai.com/v1/responses` | Responses endpoint |

Each process run creates a fresh session and prints its ID, seed, instruction,
and final evaluation. Earlier sessions remain in the same SQLite file.

## Task generation

On first use, the database is seeded with 100 uniquely named users and ten jobs.
Each session samples three distinct users and independently assigns each a job;
jobs may repeat. It randomly chooses one of these instructions:

1. Assign everyone to the first listed user's initial job, using its actual name
   in the instruction. Listing order is saved explicitly.
2. Assign one session user to a job from the full catalogue; leave others unchanged.
3. Assign one session user to one job and everyone else to a different job.

The generator first chooses structured intent and renders the instruction.
Expected assignments are computed directly from that intent and each initial row.
Intent is temporary Rust data and is not persisted. The instruction identifies
individual users by ID. Ground truth and structured intent are never sent to the
model.

## Tools and completion

- `list_assignments()` returns the session's current roster and the full job
  catalogue, including IDs and names.
- `update_assignment(user_id, job_id)` changes one session user's job. Unknown
  users/jobs and malformed arguments produce errors without changing assignments.
  An already-correct assignment succeeds as a recorded no-op.
- `commit()` freezes and grades the session, returning only `{"finalised": true}`.
  It ends the run even when the assignments are incorrect.

Tools are bound to the session by the harness. The model cannot read other
sessions, select a session ID, execute SQL, or access ground truth. Tool schemas
and descriptions are recorded with the run. Each update and its mutation event
are saved in one SQLite transaction; `commit()` finalises the task rather than
committing a transaction spanning the whole run.

The model is instructed to list first and emit exactly one tool call per response.
Multiple calls are recorded as rejected and execute no tools. Text without a call
ends the run as `missing_commit`. A run allows at most 20 model requests, with no
application retries; the twentieth response is processed before enforcing the
limit. API failures finalise as `infrastructure_error`. An abrupt process exit
can leave a `running` session on disk; automatic recovery is not implemented.

## Storage and scoring

| Table | Contents |
| --- | --- |
| `users`, `jobs` | Shared catalogues |
| `sessions` | Instruction, task type, seed, request count, lifecycle |
| `initial_assignments` | Immutable starting rows and listing order |
| `session_assignments` | Working rows changed by the model |
| `ground_truth_assignments` | Immutable expected final assignments |
| `tool_calls` | Ordered calls, arguments, results/errors, dispatch status |
| `session_events` | Messages, request starts, mutations, finalisation |
| `session_results` | One JSON evaluation per session |

Finalisation compares the complete persisted `(user_id, job_id)` sets, including
people who should remain unchanged. `state_match` records that comparison;
`success` also requires an explicit model commit. It does not penalise a temporary
wrong assignment that the model corrects before committing. Finalisation is
idempotent and prevents subsequent assignment updates.

`required_updates` counts differences between initial and expected jobs.
`reference_tool_calls` is one listing + required updates + one commit: 2–5 calls
for a three-person task. Results separately record actual model/tool calls,
listing/update/commit counts, no-op updates, failed calls, and rejected calls.
The model's terminal commit counts in its own results; forced finalisation does
not count as a tool call. Correctness and efficiency are separate measurements.

Inspect recent results:

```sh
sqlite3 assignments.sqlite3 \
  "SELECT session_id, result FROM session_results ORDER BY rowid DESC LIMIT 1;"
```

Task generation and SQLite persistence are in `src/assignment/`, the three tools
are in `src/tools/assignment_tools.rs`, and the bounded loop is in
`src/session/runner.rs`. The store owns the database connection, and the runner
passes it directly to each tool. SQLite holds lifecycle status and request counts;
the in-memory session holds only conversation identity and messages.
