# Toy Agentic Harness — System Design

Status: learning-oriented design

This document describes a small, language-agnostic agent harness that talks to an LLM through an API, maintains sessions, and executes one local tool. It is intentionally not a terminal application, editor integration, plugin host, or production agent platform.

The design is distilled from the session, processor, LLM, tool, and V2 specification code in the OpenCode repository. The goal is to preserve the underlying control-flow patterns while removing product-specific complexity.

## 1. Goals and non-goals

### Goals

- Understand the model/tool loop under the hood.
- Keep conversation state across multiple requests.
- Make every model turn and tool invocation inspectable.
- Use provider-neutral internal types, with one adapter at the API boundary.
- Validate tool input and normalize tool output.
- Handle retries, cancellation, malformed tool calls, and bounded loops.
- Start with a simple implementation that can later grow without redesigning the core.

### Non-goals

- Terminal or shell execution.
- TUI, desktop, web, or streaming UI integration.
- Multi-agent delegation.
- Long-term memory, retrieval, embeddings, or a database cluster.
- Plugin loading, permissions prompts, sandboxing, or filesystem access.
- Provider-specific reasoning/signature preservation.

## 2. Mental model

An agent is not a single LLM request. It is a state machine around repeated provider turns:

```text
user input
    │
    ▼
admit into session history
    │
    ▼
assemble model request + tool definitions
    │
    ▼
LLM provider turn
    │
    ├── final assistant text ──► persist response ──► idle
    │
    └── tool call ──► validate ──► execute ──► persist result
                                      │
                                      └──────────────► next provider turn
```

The harness, not the model SDK, owns the loop. The provider adapter only translates a request into an API call and translates the response into normalized events.

## 3. Proposed architecture

```text
                    ┌──────────────────────┐
                    │  Harness API / CLI   │
                    │  (thin future layer) │
                    └──────────┬───────────┘
                               │ submit / inspect / cancel
                    ┌──────────▼───────────┐
                    │    Session Manager   │
                    │ identity + history   │
                    └──────────┬───────────┘
                               │ one active drain per session
                    ┌──────────▼───────────┐
                    │    Session Runner    │
                    │ serialized turn loop│
                    └──────┬─────────┬────┘
                           │         │
             ┌─────────────▼───┐ ┌──▼──────────────┐
             │ Request Builder │ │  Tool Registry  │
             │ context + defs  │ │ lookup + invoke │
             └─────────────┬───┘ └──┬──────────────┘
                           │         │
                    ┌──────▼─────────▼─────┐
                    │    Provider Adapter  │
                    │ API request/response │
                    └──────────┬───────────┘
                               │
                         ┌─────▼─────┐
                         │ LLM API   │
                         └───────────┘
```

### Components

**Session Manager**

Creates sessions, records messages, returns history, and exposes status. A session is the durable conversation boundary. For the toy version, an in-memory store is sufficient; define the store interface so a file or SQL implementation can be added later.

**Session Runner**

Owns execution for one session. It serializes submissions, prevents two drains from using the same history concurrently, enforces turn/tool limits, and coordinates cancellation. This is the most important component.

**Request Builder**

Converts internal history into the provider-neutral request shape: system instructions, user/assistant/tool messages, model settings, and the advertised tool schema. It is the only place that decides what the model sees.

**Provider Adapter**

Provides one operation such as `complete(request, cancellation) -> ProviderResponse`. It translates provider-specific tool-call formats into normalized response objects and maps API errors into categories such as retryable, authentication, invalid request, context overflow, and permanent failure.

**Tool Registry**

Stores named tool definitions. It validates the model-facing name, decodes arguments, invokes the handler with an explicit invocation context, validates/normalizes the result, and returns a tool settlement. The runner supplies session and message identity; the registry should not infer it.

**Event Log / Observer**

Emits lifecycle events for debugging and tests. The toy can keep these in memory. Durable history and live progress are separate concepts: final messages/tool results belong in history; token deltas are optional ephemeral events.

## 4. Core data model

Use stable IDs even in the toy. They make retries, logs, and tool-call correlation understandable.

```text
Session {
  id: SessionId
  createdAt: Timestamp
  updatedAt: Timestamp
  status: idle | running | cancelling | failed
  messages: Message[]
}

Message {
  id: MessageId
  sessionId: SessionId
  role: system | user | assistant | tool
  createdAt: Timestamp
  content: string | StructuredContent
  toolCall?: ToolCall
  toolResult?: ToolResult
  error?: ErrorRecord
}

ToolCall {
  id: ToolCallId
  name: string
  arguments: JSON
}

ToolResult {
  callId: ToolCallId
  ok: boolean
  output: JSON or string
  error?: ErrorRecord
  startedAt: Timestamp
  finishedAt: Timestamp
}
```

A tool result must reference the exact tool-call ID and owning assistant message. Provider call IDs may be reused or scoped differently across turns; the harness should treat its own invocation ID as the durable identity.

## 5. The session turn algorithm

For `submit(sessionId, userText)`:

1. Load the session.
2. Admit and persist the user message before starting execution.
3. Acquire the session’s single-runner lock/queue.
4. While the session has not reached a terminal result:
   1. Build a request from current history and the tool catalog.
   2. Call the provider adapter once.
   3. Persist the normalized assistant response, including text and any complete tool calls.
   4. If there are no tool calls, mark the run complete and return the assistant text.
   5. For each tool call, resolve the tool by name.
   6. Decode and validate arguments. Invalid arguments become a tool error message; do not invoke the handler.
   7. Invoke the handler with `{ sessionId, assistantMessageId, toolCallId, cancellation }`.
   8. Persist exactly one tool result for each call, including failures.
   9. Continue with the updated history.
5. Release the runner and publish the final status.

The key invariant is: a provider turn is complete before its tool results are fed into the next provider turn. Parallel tool execution is an optimization for later; start with sequential execution because it makes ordering and failure behavior obvious.

### Pseudocode

```text
run(session, newUserMessage):
  store.append(user(newUserMessage))

  for turn in 1..MAX_PROVIDER_TURNS:
    request = requestBuilder.build(store.history(session), tools.definitions())
    response = provider.complete(request, cancelSignal)
    assistant = normalizeAssistant(response)
    store.append(assistant)

    if assistant.toolCalls is empty:
      status = idle
      return assistant.text

    for call in assistant.toolCalls:
      result = tools.invoke(call, invocationContext(assistant, session))
      store.append(toolMessage(call, result))

  store.append(systemError("turn limit exceeded"))
  status = failed
```

## 6. The one learning tool

Use a deterministic `lookup_note` tool backed by a small in-memory map. It demonstrates the complete tool lifecycle without introducing shell, network, or filesystem authority.

```text
name: lookup_note
description: Look up a short note by topic.
input:
  topic: string, required, 1..80 characters
output:
  found: boolean
  topic: string
  note: string | null
```

Example registry entry:

```text
lookup_note({ topic }, context):
  normalizedTopic = trim(lowercase(topic))
  note = notes[normalizedTopic]
  return { found: note exists, topic: normalizedTopic, note: note or null }
```

The model receives only the JSON schema and description. It does not receive the handler or the in-memory map. The harness executes the handler after the model emits a tool call, then sends the validated result back as a tool message.

This tool is intentionally boring. A predictable tool makes it easier to study whether the harness correctly handles IDs, ordering, malformed arguments, repeated calls, and continuation.

## 7. Provider-neutral interfaces

Keep interfaces small and make provider-specific types stop at the adapter boundary.

```text
Provider.complete(Request, Cancellation) -> Response | ProviderError

Request {
  model: ModelRef
  system: string[]
  messages: ModelMessage[]
  tools: ToolDefinition[]
  temperature?: number
  maxOutputTokens?: number
}

Response {
  assistantText?: string
  toolCalls: ToolCall[]
  finishReason: stop | tool_call | length | error
  usage?: Usage
}

ToolDefinition {
  name: string
  description: string
  inputSchema: JSONSchema
}

ToolDefinition.execute(input, context) -> ToolOutput | ToolFailure
```

If a provider supports streaming, the adapter may emit deltas, but the runner should consume them into a complete normalized response before committing the provider-turn boundary. This keeps the first implementation simple and avoids feeding partially assembled calls into tool execution.

## 8. Correctness and safety rules

These are the practices worth retaining from OpenCode even in a toy:

- **One active runner per session.** Concurrent callers enqueue or join; they never mutate the same history concurrently.
- **Persist before continuing.** Record the assistant tool call before executing it, and record the tool result before making the next API request.
- **Validate at boundaries.** Validate tool names, input JSON, and output JSON. Unknown tools become explicit model-visible errors.
- **Separate expected failure from cancellation.** A tool can return a recoverable error for the model; cancellation should stop work and mark the run interrupted rather than pretending it was a normal tool result.
- **Bound the loop.** Enforce both a maximum provider-turn count and a maximum tool-call count per run. This prevents accidental infinite tool loops.
- **Bound output.** Limit tool output sent back to the model. Retain a complete result only if the harness has an explicit retention mechanism; otherwise return a clear truncation marker.
- **Correlate by IDs.** Never match a tool result only by name or array position.
- **Do not replay side effects blindly.** If a process dies after a tool starts, mark the invocation interrupted or unknown. Do not silently execute it again unless the tool is explicitly idempotent.
- **Use typed error categories.** Retry only transient provider failures. Do not retry authentication errors, malformed requests, invalid tool arguments, or context overflow indefinitely.
- **Keep model-visible history separate from internal logs.** Internal diagnostics should not accidentally become prompt content.

## 9. Persistence strategy

Start with an in-memory `SessionStore`:

```text
SessionStore.create() -> Session
SessionStore.get(sessionId) -> Session
SessionStore.append(sessionId, Message) -> void
SessionStore.history(sessionId) -> Message[]
SessionStore.updateStatus(sessionId, Status) -> void
```

Design the store as append-oriented even if the implementation uses mutable objects. An append-oriented history makes replay, debugging, and eventual persistence straightforward. A later file/SQL store can add an event sequence number and idempotency key without changing the runner algorithm.

For prompt submission, an optional `requestId` should make exact retries idempotent: the same `(sessionId, requestId)` returns the original admission/result instead of appending a duplicate user message.

## 10. Context window strategy

The first version may fail clearly when the request is too large. Before adding compaction, measure the assembled request and expose its estimated size in logs.

The next upgrade is a checkpoint-based compaction boundary:

1. Keep the full durable transcript.
2. Summarize older messages into a compact checkpoint.
3. Build future provider requests from the checkpoint plus recent messages.
4. Never replay incomplete tool calls across the boundary.

This mirrors the useful separation in OpenCode: compaction changes the model-visible representation without deleting the underlying conversation record.

## 11. Observability

Every run should produce structured events like:

```text
run.started(sessionId, requestId)
provider.requested(sessionId, turn, messageCount, toolCount)
provider.completed(sessionId, turn, finishReason, usage)
tool.started(sessionId, assistantMessageId, toolCallId, name)
tool.completed(sessionId, toolCallId, ok, duration)
run.completed(sessionId, turns)
run.failed(sessionId, category, message)
```

At minimum log session ID, request ID, turn number, tool-call ID, model, duration, and error category. Do not log API keys or blindly log full prompts/tool outputs.

## 12. Testing plan

Use a fake provider before connecting to a real API. It should return scripted responses:

1. Plain assistant text → one provider call, no tool call.
2. `lookup_note` call followed by final text → two provider calls and one tool result.
3. Malformed arguments → tool handler is not called; model receives a structured error.
4. Unknown tool → explicit tool error; no crash.
5. Provider transient failure → bounded retry, then success or terminal failure.
6. Cancellation during provider call → run stops and session is not left `running`.
7. Two submissions to one session → serialized order, no interleaved history.
8. Repeated identical tool calls → loop guard eventually stops the run.
9. Request too large → clear context error; later add compaction tests.

The most valuable assertion is the history trace, not just the final answer:

```text
user → assistant(tool_call) → tool(result) → assistant(final)
```

## 13. Implementation sequence

### Milestone 1 — deterministic skeleton

- Internal types and IDs.
- In-memory session store.
- Fake provider.
- Session runner with no tools.
- History-trace tests.

### Milestone 2 — tool loop

- Tool definition and registry.
- JSON-schema validation.
- `lookup_note`.
- Tool-call/result correlation.
- Turn and tool limits.

### Milestone 3 — real provider

- One provider adapter.
- API-key configuration through environment/config injection.
- Error normalization and bounded retry.
- Optional streaming-to-events adapter.

### Milestone 4 — resilience

- Cancellation propagation.
- Idempotent request admission.
- File/SQL session store.
- Context estimation and compaction.
- Replayable event log.

## 14. Deliberate simplifications

The following OpenCode concerns are useful references but should not enter the first toy harness: project/worktree placement, plugin scopes, permission systems, shell/filesystem tools, provider-native reasoning metadata, multi-location registries, background jobs, child sessions, live UI deltas, snapshots, and distributed execution ownership.

The design should retain seams for these later, but implementing them now would obscure the central lesson: the harness is a serialized state machine that alternates between model decisions and controlled tool effects.

## 15. Source notes

The main source patterns reviewed were:

- OpenCode session admission, serialized execution, continuation, interruption, and compaction: [`specs/v2/session.md`](/Users/svetoslavangelov/code/opencode/specs/v2/session.md)
- Provider-turn processing, tool-call tracking, tool settlement, retry, and cleanup: [`session/processor.ts`](/Users/svetoslavangelov/code/opencode/packages/opencode/src/session/processor.ts)
- Provider-independent stream input and adapter boundary: [`session/llm.ts`](/Users/svetoslavangelov/code/opencode/packages/opencode/src/session/llm.ts)
- Typed tool context, input validation, output normalization, and failure semantics: [`tool/tool.ts`](/Users/svetoslavangelov/code/opencode/packages/opencode/src/tool/tool.ts)
- Tool registration and effective lookup: [`tool/registry.ts`](/Users/svetoslavangelov/code/opencode/packages/opencode/src/tool/registry.ts)
- Focused V2 tool design laws and execution contract: [`specs/v2/tools.md`](/Users/svetoslavangelov/code/opencode/specs/v2/tools.md)

