# Job assignment through tools

Implementation note: [the README](../../README.md) describes the current small
prototype: 100 users, ten job IDs, three users per session, and the implemented
SQLite schema and tools. This document retains the broader experiment plan;
the two-job examples, run/suite schema, training, and reporting sections below
are design notes rather than a description of everything currently implemented.

## Purpose

Explore whether a tiny local language model can interpret varied natural-language instructions, read the available tool definitions, inspect current assignments, and make the correct edits within a constrained resource budget.

Start with the configured GPT Luna to validate the task, tools, and evaluator. The expectation is that this assignment task is straightforward for a model of that class; API and infrastructure errors may be more prominent than incorrect decisions. Record both separately rather than assuming every failed run is a reasoning failure. Once the task is stable, fine-tune FunctionGemma 270M on verified demonstrations and repeat the same experiments. Teacher success establishes that the task is workable; student capability must be measured separately.

Keep the implementation small: a seeded SQLite database on disk, three tools, and a deterministic evaluator. Ordinary code owns storage and validates individual operations. The model resolves the user's intent, chooses which assignments to update, and explicitly calls `commit()` when it considers the task complete. It receives structured observations and needs no SQL knowledge.

## Task

Each person has exactly one current job type: `maintenance` or `inspection`. Multiple people may have the same job type. Assigning someone a different job replaces their previous assignment; it does not create an additional assignment or displace another person.

Example request:

> Assign Sam to maintenance. Assign everyone else to inspection.

| Person | Initial assignment | Expected assignment | Action |
| --- | --- | --- | --- |
| Sam | Inspection | Maintenance | Update |
| Alex | Maintenance | Inspection | Update |
| Lee | Inspection | Inspection | None |

The efficient solution makes one list call, two update calls, and one `commit()` call: four tool calls across four model requests. The model must recognise that Sam needs reassignment, Alex must also change, and Lee is already correctly assigned, then signal completion.

“Everyone else” means every other person returned by the listing tool. For a request that names only Sam and says nothing about others, everyone else's assignment must remain unchanged. Requests are initially unambiguous and achievable, with unique display names and known people.

## Data and storage

Create a new task-run instance when a new session is created. The current app creates one runner and one session per process run, so starting the app creates one task run. Associate it with a unique `run_id` and its `session_id`. For repeated evaluation, instantiate the same versioned task and seed under a new `run_id`; do not regenerate a different task merely because the session is fresh.

Use one SQLite file on disk with shared tables partitioned by `run_id`, rather than creating a new physical table for every run:

| Table | Contents |
| --- | --- |
| `task_runs` | Session ID, task/suite ID and version, task type/request family, instruction, seed, split, model/configuration, tool definitions or immutable version, timestamps, and lifecycle status |
| `initial_assignments` | Immutable starting roster and assignments |
| `work_assignments` | Current roster and assignments exposed through the tools |
| `ground_truth_assignments` | Immutable expected final roster and assignments, private to the evaluator |
| `session_events` | Ordered messages, tool calls, tool results/errors, actual mutations, request outcomes, and runner finalisation events |
| `run_results` | One final result per run: state match, constraint/completion grades, termination reason, call counts, and resource measurements |

The assignment tables each have one row per person per run:

| Field | Meaning |
| --- | --- |
| `run_id` | Identifies the task run; combined with `person_id` forms the primary key |
| `person_id` | Unique, stable identifier within the episode |
| `name` | Public display name, unique within the initial task |
| `job_type` | Current assignment: `maintenance` or `inspection` |

Start with three people as in the example, and vary names, row order, and initial assignments across episodes. Modestly larger rosters can measure how the number of required edits affects performance without changing the task itself.

At session creation, persist the run metadata, initial rows, a working copy, and independently computed ground truth before the first model request. The initial snapshot is retained after the work table changes. Bind tools to the active `run_id` in the harness; the model cannot select another run. It has no direct database access and cannot read ground truth, grading metadata, or other sessions. Tools perform validated, parameterised operations.

Only `job_type` is editable. Each valid update persists its row change and mutation record atomically before returning its resulting row. There is no transaction spanning the whole episode and no automatic rollback of earlier successful updates when a later call fails. The model-facing `commit()` finalises the experiment; it is distinct from these individual SQLite transaction commits.

Persist events as the session progresses, not only at finalisation. Use a monotonic event sequence within each run and retain tool-call IDs so actions and results can be reconstructed in order. An abrupt process exit leaves an unfinished run visible; on recovery, finalise it as interrupted when storage is available rather than inventing a model commit or silently replaying writes.

## Tools

The model receives these three tool definitions with the user request, plus instructions to issue one tool call per response and finish with `commit()`. Discovery of people and current state happens through the listing tool; no separate registry or inspection call is required.

### `list_assignments()`

Takes no arguments and returns the complete roster, current assignments, and supported job types:

```json
{
  "supported_job_types": ["maintenance", "inspection"],
  "assignments": [
    {"person_id": "p17", "name": "Sam", "job_type": "inspection"},
    {"person_id": "p42", "name": "Alex", "job_type": "maintenance"},
    {"person_id": "p08", "name": "Lee", "job_type": "inspection"}
  ]
}
```

The response reports facts, not which people match the request or which edits to make. Repeated listing returns the same values unless a successful update has changed an assignment. Keep row order fixed within a trial and vary it independently between task instances. Listing has no side effects and needs no filtering or pagination in the first version.

### `update_assignment(person_id, job_type)`

Sets one person's assignment to one explicit supported job type:

```json
{
  "person_id": "p17",
  "job_type": "maintenance"
}
```

Returns the committed row:

```json
{
  "person_id": "p17",
  "name": "Sam",
  "job_type": "maintenance"
}
```

Unknown people, unsupported job types, malformed arguments, and extra fields return errors without changing any row. Setting the existing assignment succeeds as a no-op and is recorded as an unnecessary update call.

The tool validates the operation, not the user's policy. Assigning Alex to maintenance is mechanically valid even when the instruction requires inspection. The evaluator must detect that mistake. The tool must not accept a natural-language goal, select people on the model's behalf, or use hidden reference intent to block an otherwise valid edit.

### `commit()`

Takes no arguments. It means: “I consider the requested assignment changes complete.” This explicit decision is part of the capability being measured. The tool does not accept claimed results, counts, or expected assignments from the model. Reject malformed or extra arguments without finalising; the model may correct the call within its remaining budget.

The handler invokes the harness's finalisation operation, which freezes the work state, compares it with ground truth, grades mutation history, and writes the results for this run. Store the results, terminal event, and final lifecycle status atomically, with at most one results row per run. Finalisation is idempotent: a repeated internal invocation returns the existing result without grading again or adding another completion event.

The tool returns a minimal acknowledgement such as `{"finalised": true}`. Detailed grades remain in evaluator storage. A valid model commit ends the session immediately, whether the table matches or not; it is not an opportunity to inspect a score and try again. There is no subsequent model request and no further work-table mutation.

The same internal finalisation operation is used when the runner stops a run. Record who initiated it and why; a forced finalisation is a runner event, not an assistant tool call. It must never be counted or exported as a model-selected `commit()`.

## Episode behaviour

- Every trial starts with fresh working rows and a fresh conversation, while prior runs remain on disk.
- Names, IDs, roster membership, and supported job types remain fixed during the trial. Only explicit assignment updates change business state.
- Tools execute sequentially, without background changes or external services. Initially require exactly one tool call per model response, for both Luna and the student. If a response contains multiple calls, execute none and finalise with `protocol_error`; retain the emitted calls as rejected protocol evidence.
- The model normally lists assignments, updates people whose assignments differ from the request, and calls `commit()`. A second listing is optional because updates return their resulting state. Do not require a fixed update order.
- A response without a tool call does not satisfy the completion requirement. Finalise with `missing_commit`, retaining any state match as a separate grade.
- An already-satisfied request can be completed with `list_assignments()` followed by `commit()`, with zero updates.
- Allow at most **20 actual model network requests**, counting failed requests as well as successful ones. Increment the counter when a request is attempted, not when checking whether another request is allowed. Initially use no automatic API retries; any later retry policy must consume this same budget and be recorded.
- Process the twentieth response and its single tool call before deciding whether to stop. A valid `commit()` in that response is normal completion. If it does not finalise the run, the runner invokes finalisation with `network_limit` before any twenty-first request. Forced finalisation makes no model request.
- The one-call protocol permits at most 20 dispatched tool calls, including the model's commit. Declare output and context limits too, and ensure generated tasks have reference solutions that fit within the budget.
- Recoverable tool errors return to the model and consume the request already used. Terminal API/infrastructure errors finalise with `infrastructure_error` when storage is available. These errors remain distinct from wrong tool decisions.

Finalisation reasons are `model_commit`, `network_limit`, `missing_commit`, `protocol_error`, `infrastructure_error`, and `interrupted`. Baselines that use a deterministic executor record `executor_commit` for normal completion. The origin is set by the harness, not supplied in tool arguments. Preserve a more specific error category alongside the reason where available.

## Requests and formal intent

Use three request families within this assignment task:

1. **Named person:** “Assign Sam to maintenance.” Everyone else remains unchanged.
2. **Everyone:** “Assign everyone to inspection.” Every person has the requested assignment.
3. **Named person and everyone else:** “Assign Sam to maintenance and everyone else to inspection.” The named person gets one assignment; the rest get the other.

Vary the named person, requested job types, and wording independently of the initial state. Include reversed assignments, such as Sam to inspection and everyone else to maintenance. Names and IDs must not predict a person's current or desired job.

Represent intent with a small fixed structure. For the main example:

```json
{
  "person_id": "p17",
  "job_type": "maintenance",
  "others_job_type": "inspection"
}
```

Interpretation rules:

- With a non-null `person_id`, assign that person to `job_type`.
- With a non-null `person_id` and null `others_job_type`, leave everyone else unchanged.
- With both fields non-null, assign all other people to `others_job_type`.
- With null `person_id`, assign everyone to `job_type`; `others_job_type` must then be null.

`job_type` must be non-null and contain a supported job type. `others_job_type`, when non-null, must also contain a supported job type. A non-null person ID must exist in the public roster. Require all three fields and reject extra fields or invalid combinations before a translated intent is executed.

The reference intent belongs to the generator/evaluator. It is never supplied to the tool-using model. A translator baseline may produce this structure from the natural-language request and public listing.

## Generation and ground truth

Generate the initial roster, assignments, and formal intent, then render the intent as a natural-language request. Compute the expected final state from a copy of the initial database. Preserve every person's ID and name.

Include zero, one, and multiple required changes. Pair the same instruction with different initial assignments so the correct update count changes. Pair different paraphrases with equivalent states so language variation can be distinguished from execution difficulty.

Begin with hand-written paraphrase templates and manually check their meaning against the reference intent. Hold out some wording templates and combinations of named targets and requested assignments. A random split of near-identical examples or merely changing IDs is insufficient evidence of generalisation.

For every task, compute the minimum required number of updates:

```text
required_updates = count of people whose initial job differs from their expected final job
```

Each update tool call can affect only one person, so this count is the lower bound for a correct solution. Listing and commit calls are counted separately. A reference solution uses one listing, `required_updates` updates, and one commit; generated tasks must fit those calls within the episode budget.

## Approaches to compare

Build the tool-using configuration first and validate it with Luna. Then compare approaches using the same task and tool semantics:

| Approach | Interpretation and execution |
| --- | --- |
| Parser baseline | A hand-written parser translates supported wording into formal intent; ordinary code lists assignments, applies the necessary updates, and commits |
| Model translator | The model uses the public roster to resolve names and produces formal intent; ordinary code applies the necessary updates and commits |
| Tool-using model | The model lists assignments, interprets the request, chooses individual updates, and explicitly commits |

All approaches receive the same request, tool definitions, and access to public state. Neither baseline receives the generator's reference intent or reads private database state to select actions. The deterministic executor uses the same listing, update, and commit operations, and its calls and execution time count in the comparison. It can reuse the listing already obtained for name resolution because the environment has no concurrent changes. Record executor-originated calls separately from model-originated calls. Baseline completion is not evidence that its model learned when to call `commit()`; that measurement belongs to the tool-using configuration. The one-call response protocol applies to tool-using turns; a translator's structured intent is its declared output contract.

Reject invalid translated intents before execution and finalise with `protocol_error` and an intent-rejection category. Parser rejection counts as failure on these supported requests. Compare translator and agent configurations using the same model checkpoint and documented prompts, decoding settings, and budgets. Repeat the model comparisons for the untuned student, tuned student, and teacher.

The comparison asks whether model interpretation improves on a parser, and whether repeated model decisions add value beyond one translation followed by ordinary code. If the translator is more reliable or cheaper, that is a useful result.

## Evaluation

Implement grading with ordinary Rust functions invoked by finalisation. SQLite is the durable source of task runs, events, and results; JSONL is an export format for analysis and training. Grade persisted state and action history independently of the assistant's claimed success. No model judge is needed initially.

### Correctness

| Grade | Definition | Use |
| --- | --- | --- |
| Final state | Every person's assignment equals the reference final assignment; IDs, names, and roster membership are unchanged | Required for success |
| Instruction constraints | Every actual assignment mutation sets that person to their reference final assignment; people required to remain unchanged are never mutated | Required for success |
| Completion | Tool-using model explicitly called `commit()` within limits, or a baseline executor committed normally, without a terminal error or rejected intent | Required for success |
| Partial completion | Fraction of people who initially required changes and now have the correct assignment; N/A when no changes were required | Diagnostic only |
| Protocol errors | Invalid intents, malformed calls, unknown IDs, unsupported job types, and invalid arguments | Diagnostic only; recovery may still succeed |

Store `state_match`, `constraints_ok`, `completion_ok`, and overall `success` separately, along with termination reason and finalisation origin. Trial success requires the first three grades. A matching table at the network limit is a state match but not successful explicit completion. A premature model commit is explicit completion but fails if the table is wrong. If a terminal infrastructure error prevents grading, mark the affected grade unavailable rather than claiming a mismatch; overall success is false.

The history rule deliberately rejects a wrong committed reassignment even if it is later corrected. This applies to the named person and to everyone else, including requests that select the whole roster. Successful no-op calls do not mutate state and affect efficiency rather than correctness.

Do not require the exact teacher trace, a fixed update order, or a final listing. A correct no-update trial with an explicit commit can pass. Partial completion cannot override a forbidden change or missing commit.

Validate graders with hand-checked fixtures: correct reassignment, omitted person, wrong person, wrong job, unintended changes to others, a wrong assignment followed by correction, already-satisfied requests, and recoverable errors. Also verify premature commit, missing commit with a matching table, model commit on request 20, forced finalisation after request 20, infrastructure failure, and repeated internal finalisation. Use a reference executor to check solvability, but validate the grader independently of shared generation/execution logic.

### Calls and resource costs

Record calls dispatched to each tool, including failed calls, and distinguish model versus executor origin. Separately count successful state-changing updates, successful no-op updates, failed update attempts, and emitted calls rejected before dispatch. Record the task's `required_updates` alongside those counts. A dispatched model `commit()` counts as one tool call and uses its model request; forced finalisation counts as neither. Include the terminal commit in the frozen counts used to generate its own results row.

For successful trials, report excess update calls as `update_assignment_calls - required_updates`. For failed trials, show raw counts without interpreting fewer calls as greater efficiency: an agent that omitted everyone may make zero updates and still fail completely.

The example's efficient result is one listing, two updates, and one commit. Two updates to the wrong people fail; three updates with one no-op followed by a commit may still be correct but less efficient. Correctness and efficiency remain separate rather than being combined into a weighted score.

Report model calls, tool calls, input/output tokens, median and p95 end-to-end latency, and peak resident memory. Include sample counts, hardware, runtime, quantisation, and cold/warm conditions. Report latency for all attempts and successful attempts separately. Hosted teacher memory is unavailable, not zero; hosted latency includes network and service overhead.

### Development and held-out suites

Start with 30 development tasks: ten per request family. Cover both job types, different named people, changed and already-correct assignments, and zero/one/multiple required updates. Include controlled state variants and equivalent paraphrases. These counts are an initial experiment budget, not evidence of deployment reliability.

Before fine-tuning, create a separate frozen evaluation suite with held-out wording and intent combinations. Keep related paraphrases and paired variants within one split. Tune prompts, parser rules, and checkpoints on development data only. Maintain a regression subset for established behaviour and a capability subset for harder wording or larger rosters within the same assignment task.

For each model configuration, run three fresh trials per task. Report mean single-attempt success (pass@1) and the fraction of tasks passing all three trials (empirical pass^3). Repetitions measure variability; they are not retries awarded to a user, and the best trial must not replace the full results. A deterministic parser needs one correctness trial per task.

Report results by request family and edge-case tag, with task/trial counts. Keep infrastructure failures in the primary all-attempt denominator and report them separately. Do not silently repeat failures until they pass. If reporting uncertainty, account for trials being grouped by task.

One report row per approach/model should include state-match rate, explicit model-commit rate where applicable, success, all-three success where measured, forbidden-change rate, termination reasons, infrastructure/protocol errors, required versus actual updates, listing/commit/no-op calls, latency, tokens, and memory. Resource budgets and any deployment acceptance thresholds must be declared before interpreting results as deployment readiness.

## Traces, training, and replication

Save task/suite versions, generation seed, initial database contents, reference intent, expected final state, and required update count as evaluator metadata. Separately save the model-visible request, tool schemas, assistant responses, arguments/results, actual committed mutations, timings, model configuration, termination reason, and grades in the run's persistent records. Hidden reference data must never enter model context.

Every task run contributes to the experiment archive. It does not automatically become an example to imitate. Initially export demonstrations only from teacher runs that match ground truth, satisfy instruction constraints, and end with a model-selected commit. Retain failed and interrupted runs for diagnosis; do not relabel a runner's forced finalisation as a successful assistant action. Record demonstration acceptance rates by task family so filtering cannot conceal systematic failures.

### Student and training format

Use **FunctionGemma 270M**, the function-calling variant of Gemma 3 270M, as the initial student candidate. Its documented base training covers single-turn and parallel tool calls; dependent multistep workflows require task-specific evaluation and fine-tuning. Do not assume that the ordinary Gemma checkpoint or the untuned FunctionGemma checkpoint already performs this sequence reliably. [FunctionGemma formatting and training scope](https://ai.google.dev/gemma/docs/functiongemma/formatting-and-best-practices)

Keep complete runs as the source data and derive one training example per assistant decision:

```text
input = task instruction
      + tool definitions and execution/completion instructions
      + all preceding assistant tool calls
      + their tool results and errors, in conversation order

target = next assistant tool call
```

Tool results are essential: they tell the model which people exist, what their assignments were, and whether earlier actions succeeded. Include the full preceding history, not just the last call. Do not include future responses, the ground-truth table, final grades, or private provider reasoning in the training input. The target is one observed valid next action; it need not be the only correct update order.

For the example task, one verified run yields four training examples. Each row below also includes the original instruction and all three tool definitions:

| Example | Preceding history in the input | Target call |
| --- | --- | --- |
| 1 | No calls yet | `list_assignments()` |
| 2 | Listing call and returned roster | `update_assignment("p17", "maintenance")` |
| 3 | All preceding history, including Sam's successful update and result | `update_assignment("p42", "inspection")` |
| 4 | All preceding history, including Alex's successful update and result | `commit()` |

An already-satisfied run yields a listing example and a commit example, with no update examples. Include such cases so the student learns to stop without unnecessary edits. A model commit is a training target only when the run passes the demonstration criteria; committing prematurely must not be copied as correct behaviour.

Train with loss on the target assistant completion, including its call-ending tokens. Earlier messages and tool results supply context; they are not output targets. During supervised training the tool results are recorded data, not live database operations. At inference the runner executes each generated call and feeds back the actual result before requesting the next action.

A complete conversation can also be a training example with loss restricted to assistant outputs; one-call targets are a deliberate initial format, not a fundamental limitation of supervised fine-tuning. Explicit history-to-next-call examples make the intended decision boundary easy to inspect. TRL supports tool-call conversations with tool results and schemas, as well as completion-only training; verify the loss mask and template behaviour on the selected runtime. [TRL SFT documentation](https://huggingface.co/docs/trl/en/sft_trainer)

Store provider-neutral calls and results, then export them using FunctionGemma's documented tool declaration, call, response, and turn markers. Use the same tokenizer, serialization, and stopping rules during training and inference. Do not copy Luna's provider-specific opaque state into the student dataset. Keep full bounded history initially; any later context truncation or summarisation is a versioned configuration applied consistently to training and evaluation.

Assign training/development/evaluation splits to whole task groups before expanding runs into per-action examples. All steps of one run, repeated trials of the same task, and related paraphrases or paired state variants belong to the same split. Otherwise, prefixes and near-duplicate decisions could leak from training into evaluation.

### Iteration sequence

After relevant prompt, tool, parser, or model changes, rerun development tasks and inspect failures plus a sample of successes. Distinguish language interpretation errors, name resolution errors, missed reassignments, unnecessary calls, protocol failures, and grader bugs. Add discovered development failures to regression coverage. If a held-out case guides a fix, treat it as development information thereafter and version the evaluation suite.

Use this sequence:

1. Implement session-bound task creation, on-disk SQLite seeding, all three tools, durable event/mutation logging, and the deterministic finalisation/grading operation. Verify a scripted solution and independent grader fixtures.
2. Update the runner to enforce the one-call protocol, stop immediately on commit, and finalise at the 20-request boundary or other terminal exit. Run the Luna development pilot and refine the task until the tools and intended behaviour are stable.
3. Freeze the task contract and held-out suite. Run an untuned student baseline before investing in a large training corpus.
4. Generate teacher demonstrations from training episodes. Accept successful demonstrations only after they pass the independent grader and include a model commit; record rejection rates by task family.
5. Export the verified runs into history-to-next-call examples in FunctionGemma's format and fine-tune the student. Include already-satisfied cases, commit decisions, and valid alternative update orders. Keep training examples out of the frozen evaluation suite.
6. Repeat the same evaluation for teacher, untuned student, and tuned student, including translator and parser baselines where applicable.

Record model/checkpoint identifiers, tokenizer and chat format, prompts, decoding settings, context policy, training seeds, dataset versions, runtime, quantisation, hardware, and code revision. Keep public information and tool permissions equivalent across configurations. Any format adapter or constrained decoding is part of the measured system, not an unreported advantage.

Evaluate complete student runs through the real tools, not only next-call accuracy on teacher histories. A student can predict isolated actions well yet repeat edits or fail to commit when following its own generated history. The first result should establish how reliably the model maps language and current assignments to the correct individual updates, recognises completion, and what those decisions cost relative to simpler software.

## Future work: local decisions and tool routing

After the initial experiment, explore a local decision model for bounded choices, inspired by [OpenAI's Decisions API](https://openai.com/index/devday-2026-recap/) and [TypeSafe's Jev](https://typesafe.ai/blog/introducing-system-one-models-and-jev). The deployment goal remains on-device inference within a constrained resource budget; hosted services can inform the approach but do not satisfy that goal.

For the assignment task, every intent parameter can be selected from the public roster, supported job types, and explicit everyone/unchanged choices. A decision model could therefore implement the existing translator baseline: select a formal intent once, validate its fields and combinations, then let ordinary code execute the necessary updates through the same tools. Compare this with the parser, generative translator, and tool-using model using end-to-end success, latency, and memory. Selecting an allowed value does not guarantee semantic correctness, and executor completion remains distinct from a model-selected commit. Tools requiring arbitrary text still need generation or another extraction mechanism.

When the harness grows to include many tool sets, explore a small local router that uses the request and relevant conversation state to select schemas for the local LLM, which then produces tool arguments. A classifier trained for known tool families is a sufficient starting point; selecting candidates from a changing roster requires separate evaluation. Allow multiple tool sets and broader discovery when uncertain, with thresholds validated on held-out requests. Measure whether reduced prompt processing outweighs router latency and memory, including fallback costs. Defer routing until the tool catalogue is large enough to test a meaningful benefit.

## Evaluation background

The protocol adapts the earlier design's task-specific grading, repeated trials, trace inspection, held-out evaluation, and resource reporting to this assignment task. These are local experimental choices rather than a claim of compliance with an industry standard.

Background references retained from the earlier evaluation design:

- [Anthropic: Demystifying evals for AI agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
- [OpenAI: Evaluation best practices](https://developers.openai.com/api/docs/guides/evaluation-best-practices)
- [OpenAI: Evaluate agent workflows](https://developers.openai.com/api/docs/guides/agent-evals)
