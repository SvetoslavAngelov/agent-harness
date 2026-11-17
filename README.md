# Agent harness

A small Rust harness for models that use tools through an OpenAI-compatible
Responses API. It runs a bounded tool loop and stores messages, tool calls and
results in SQLite. Model inference is served separately, locally or remotely.

The included task asks a model to inspect a roster, update job assignments and
commit the result.

## Run

With a Rust toolchain installed, set your API key and run the harness.

```sh
export OPENAI_API_KEY="your-api-key"
cargo run
```

For a local model server, set the endpoint and model name.

```sh
OPENAI_API_KEY=local \
OPENAI_MODEL=functiongemma-270m-it \
OPENAI_API_URL=http://127.0.0.1:8080/v1/responses \
OPENAI_REQUEST_TIMEOUT_SECONDS=60 \
cargo run
```

| Variable | Default |
| --- | --- |
| `OPENAI_MODEL` | `gpt-5.6-luna` |
| `OPENAI_API_URL` | `https://api.openai.com/v1/responses` |
| `OPENAI_REQUEST_TIMEOUT_SECONDS` | `30` |
| `ASSIGNMENTS_DB` | `assignments.sqlite3` |
| `TASK_SEED` | Random `u64` |

Environment variables must be set in the shell. The harness reads no `.env` file.

## How it works

The model receives three tools.

- `list_assignments()` returns the roster and available jobs.
- `update_assignment(user_id, job_id)` updates one person's assignment.
- `commit()` ends the session and grades the final state.

Each run creates a fresh session and allows up to 20 model requests. Batched tool
calls execute in order. Success requires the correct final assignments and an
explicit commit. SQLite retains the session history and evaluation.

## Check

The controlled checks use a local mock endpoint and require Python 3.

```sh
cargo build
python3 scripts/check_assignment_harness.py
```

The [experiment report](docs/report.md) compares baseline FunctionGemma, LoRA and
Luna, including Raspberry Pi latency and memory measurements.
