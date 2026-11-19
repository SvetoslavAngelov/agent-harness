# Agent harness

A small Rust harness for exercising model tool use through an OpenAI-compatible
Responses API. The harness sends conversation history and tool schemas to the
model, dispatches tool calls in order, and sends their results back. Each run
allows up to 20 model requests and saves its conversation and experiment data
as a SQLite snapshot. The bundled roster assignment prints its session ID and grade after
each run. See [the assignment task](src/tasks/assignment/README.md) for details.

## Run

Install a Rust toolchain with Rust 2024 edition support, then set your API key and
the model ID served by your endpoint.

```sh
export OPENAI_API_KEY="your-api-key"
export OPENAI_MODEL="your-model-id"
cargo run --locked
```

The default endpoint is `https://api.openai.com/v1/responses`. To use a local or
custom server, set `OPENAI_API_URL` to its full Responses API URL, for example
`http://127.0.0.1:8080/v1/responses`. Each request has a 60-second timeout.

The harness reads environment variables from the shell.

## Code layout

- `src/client/` adapts conversation messages and tool schemas to the Responses API.
- `src/session/` contains the reusable model/tool runner.
- `src/domain/` defines the session wrapper, messages, identifiers, and tool registry.
- `src/tools/` defines the shared tool interface.
- `src/tasks/` holds each task's setup, persistence, and tools.

`Session` holds the messages, request/tool counts, and a data map for experiments.
The runner updates it in memory. Add any serializable data and save when needed:

```rust
session.set("progress", &progress)?;
session.save(&connection)?;
```

Read values directly from `session.data`. Saving replaces the snapshot in
`session_snapshots`; the assignment experiment saves at setup and completion.
Only messages enter the model's context. Each experiment supplies its own tools,
state, and grading to the shared runner.
