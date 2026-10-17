# A first computer-use task for a tiny language model

**Recommendation:** build a small local maintenance-task manager and ask the model to update selected records through visible lists and forms. Give it compact textual observations and element-addressed actions. Validate the task with Luna, then compare an untuned and fine-tuned student on the same frozen environment.

This is a proposed experiment, not a measured result or an accepted replacement for the existing design. “Tiny” is provisionally treated as roughly 270M–1B parameters; the exact checkpoint, hardware, memory ceiling, and latency requirement remain open. Those choices will determine the feasible context and episode length.

## Relationship to the existing design

The [existing design](/Users/svetoslavangelov/code/agent-harness/src/tools/design.md) asks a useful question: can a small model translate varied instructions and observed state into reliable operations, and is that flexibility worth its cost relative to ordinary software? Its compartment task isolates conditional selection. Its SDL layout task adds geometric constraint satisfaction and application integration.

The proposed task keeps conditional selection and adds a practical interaction sequence: find a record, inspect its fields, edit the right control, save, and continue. It avoids making exact arithmetic and geometry part of the first student's job. The research question becomes: **can a tiny model reliably complete a short administrative workflow through a constrained application interface?**

Preserve the existing design's deterministic grading, mutation history, no-action cases, held-out language, repeated trials, and parser/translator/agent comparisons. Keep the compartment task as a cheap diagnostic baseline. Defer the SDL task as a separate capability experiment; no evidence gathered here establishes that a tiny model cannot learn spatial tasks.

The current code already selects `gpt-5.6-luna` in [main.rs](/Users/svetoslavangelov/code/agent-harness/src/main.rs), but runs only a weather demonstration. The [runner](/Users/svetoslavangelov/code/agent-harness/src/session/runner.rs) executes tool calls sequentially and continues until the model returns no calls or its network-call limit is exceeded. A task environment and evaluator still need implementation. This inspection establishes local configuration, not successful API access or any Luna performance result.

## Research evidence

The literature supports simplifying observation and action spaces, but does not provide a reliable success forecast for this particular tiny-model workflow. Published scores below belong to their original configurations and dates; they are not current leaderboards or directly comparable with the proposed experiment.

| Source | Relevant finding | Implication and limit |
| --- | --- | --- |
| WorkArena, 2024 | Introduced 33 enterprise web tasks. Its experiments found that adding coordinate features did not improve WorkArena as it did some MiniWoB tasks; element-ID actions worked well for its business interfaces. [1](https://arxiv.org/html/2403.07718v3) | Supports removing coordinate selection from a forms-and-lists task. Does not establish sub-billion-parameter performance. |
| BrowserGym, maintained project | Provides browser-agent infrastructure, multiple benchmark integrations, and support for custom tasks. [2](https://github.com/ServiceNow/BrowserGym) | A possible later adapter and external comparison route. Installing the whole ecosystem is unnecessary for the first Rust experiment. |
| MiniWoB++, documentation | Supports element-addressed clicks as well as coordinate clicks, and allows a restricted action subset. [3](https://miniwob.farama.org/content/action_space/) | Useful for control-level tests. Success on small widgets alone would not establish useful workflow completion. |
| WorkArena++, 2024; revised 2025 | Provides 682 compositional workflow tasks and a mechanism for producing observation/action traces. [4](https://arxiv.org/abs/2407.05291) | Supports adding composition after atomic operations. Its breadth would overshoot the initial experiment. |
| OSWorld, 2024 | Provides 369 real computer tasks with initial-state setup and execution-based evaluation, spanning desktop and web applications. [5](https://arxiv.org/abs/2404.07972) | Useful precedent for grading outcomes. Full desktop operation introduces substantially more environment engineering. |
| FunctionGemma model card, updated April 2026 | Describes a 270M text-only function-calling model intended for task-specific fine-tuning, including multiturn scenarios. [6](https://ai.google.dev/gemma/docs/functiongemma/model_card) | Direct precedent at the intended scale. The card does not demonstrate this proposed UI workflow. |
| Agent Distillation, 2025 | Studies students down to 0.5B using retrieval and code tools on factual and mathematical reasoning tasks. [7](https://arxiv.org/abs/2505.17612) | Supports investigating small tool-using students. Different tasks, tools, and inference procedures prevent transferring the reported performance. |
| WebRL, ICLR 2025 | Reports Llama-3.1-8B improving from 4.8% to 42.4% on WebArena-Lite with its RL approach. [8](https://arxiv.org/html/2411.02337v2) | Training can materially improve a web agent, but 8B is larger than the intended student and the remaining failures matter. |

WorkArena also reports very large enterprise DOM observations, which reinforces the need to control observation size. Its BrowserGym interface assigns element IDs and exposes structured page information and element-based primitives. [1](https://arxiv.org/html/2403.07718v3) The proposed compact observation is a local design choice inspired by that pattern, not a claim that arbitrary accessibility trees are small or easy.

There is useful counterevidence to dismissing all spatial tasks: FunctionGemma's model card describes a specialized Tiny Garden application that maps commands to plot-oriented actions. [6](https://ai.google.dev/gemma/docs/functiongemma/model_card) Discrete plot selection differs from satisfying viewport, spacing, non-overlap, and protected-size constraints across rectangles. The reason to defer SDL is experimental focus and implementation cost, rather than an assumed fundamental inability.

## What “computer use” means here

Three interfaces would test different capabilities:

| Interface | Model's work | Suitability for the first experiment |
| --- | --- | --- |
| Domain operations, such as `assign_job(id, owner)` | Interpret intent, inspect records, select operations | Easiest diagnostic baseline; little UI operation |
| Textual UI observations with `click`, `fill`, and `select` | Interpret intent, identify controls, navigate, edit, save | Recommended main experiment |
| Screenshots with mouse coordinates and keystrokes | All the above plus visual interpretation, targeting, focus, and timing | Separate later experiment |

The middle option is accessibility-style application control. It is a legitimate, useful interface, but success should be described as operation of this constrained application. It would not establish screenshot understanding, general desktop competence, or transfer to arbitrary websites.

A first environment can expose semantic screens directly from an application state machine. That is a UI simulation until a real frontend is connected. Add a small manually usable frontend using the same state transitions before describing the result as operating a real app. A later browser adapter should extract only public page information and invoke actual controls; the evaluator may inspect private storage, but the model may not.

## Candidate tasks

These judgments reflect fit with the repository's research objective, not measured benchmark difficulty.

| Candidate | Practical value | Main additional difficulty | Decision |
| --- | --- | --- | --- |
| Maintenance-task assignment and status updates | Routine administrative work with clear outcomes | Conditions, exceptions, navigation, saving | Best initial choice |
| File sorting and renaming | Useful personal workflow; file outcomes can be checked | Paths, duplicate names, content inspection, collisions | Strong second choice if files are the desired domain |
| Inbox labeling and archiving | Recognizable triage workflow | Message interpretation, thread semantics, accumulating context | Later; initially avoid reply generation |
| Calendar changes | Useful assistance | Dates, time zones, conflicts, recurrence | Too many new concepts for the first task |
| Device settings panel | Closest to the original device-operation motivation | May add little beyond the compartment experiment | Good alternative if device control is the main product goal |
| SDL layout | Visible results and several valid solutions | Exact geometry, constraints, application launch/render work | Preserve as a separate later task |

Maintenance jobs provide continuity with the device-oriented design while producing a more recognizable workflow. The model should apply explicitly stated policies. Inferring the true urgency of an engineering fault from prose would introduce a separate classification or diagnostic experiment.

## Proposed first task

Start with a local application containing four to six maintenance records, a list view, and one detail/edit view. Each record has a stable public identifier, title, category, status, priority, and assignee. Initially use a small fixed vocabulary: categories such as `maintenance` and `inspection`, status `open` or `closed`, priority `normal` or `urgent`, and three named assignees plus `unassigned`.

Example instruction:

> Assign every open, unassigned maintenance job to Sam. Leave urgent jobs alone and do not change any other fields.

| Job | Category | Status | Priority | Assignee | Expected result |
| --- | --- | --- | --- | --- | --- |
| Replace workshop filter | Maintenance | Open | Normal | Unassigned | Assign to Sam |
| Repair cooling alarm | Maintenance | Open | Urgent | Unassigned | Unchanged |
| Check crew ventilation | Inspection | Open | Normal | Unassigned | Unchanged |
| Inspect cargo latch | Maintenance | Closed | Normal | Unassigned | Unchanged |
| Replace hallway lamp | Maintenance | Open | Normal | Alex | Unchanged |

Titles are incidental; category and priority are explicit fields. Vary the same titles across different states so the student cannot use “alarm” as a permanent exclusion or “filter” as a permanent target.

This instruction is the eventual composed example. Begin with a shorter progression:

1. **Named update:** “Assign JOB-104 to Sam.”
2. **Conditional update:** “Assign all unassigned jobs to Sam.”
3. **Exception:** “Assign unassigned jobs to Sam, except urgent jobs.”
4. **Composition:** combine category, status, assignee, and an exception as above.

Keep named no-ops and zero-match cases in every applicable family. Start the fourth family only after the first three behave sensibly with Luna. Completing more clicks does not by itself make an experiment more informative.

### Formal intent and observability

The generator can represent the composed example using a fixed schema:

```json
{
  "job_ids": null,
  "category": "maintenance",
  "status": "open",
  "assignee": "unassigned",
  "exclude_priority": "urgent",
  "set_assignee": "sam"
}
```

Null fields apply no restriction. Conditions refer to the initial business state; exclusions remove targets. Selected records receive the requested assignee, while every other field and record remains unchanged. The model sees only the natural-language request and public UI observations. Hidden intent and expected state stay in the evaluator.

Initially, the list shows all filtering fields. A later version can place priority or category only in the detail view, making inspection necessary. Do not simultaneously add pagination, hidden fields, error recovery, and more complex language: their failures would become difficult to distinguish.

### Minimal tools

Use four small, explicit tools rather than a broad command language:

| Tool | Meaning |
| --- | --- |
| `observe()` | Return the current public screen and available controls. |
| `click(ref)` | Activate one visible, enabled control. |
| `fill(ref, text)` | Replace the contents of one editable text field. |
| `select(ref, value)` | Choose one exposed option in a select control. |

Every action returns the updated observation. An ordinary final assistant response ends the episode, matching the existing design. `fill` supports simple search or an exact user-specified title edit; it need not appear in every assignment episode. It can be omitted from the very first assignment-only capability slice.

An illustrative detail observation is:

```text
screen: Job details
job: JOB-104 — Replace workshop filter
category: Maintenance
status: Open
priority: Normal

[e7] select Assignee = Unassigned
     options: unassigned, sam, alex, lee
[e8] button Save (disabled)
[e9] button Back to jobs
```

After `select("e7", "sam")`, the view shows the new draft value and enables Save. `click` on the current Save reference commits it and returns the list with updated state. The model must distinguish editing a draft from saving a record.

References are opaque and must come from observations. Keep them valid while the same control exists, invalidate them when the screen is replaced, and never silently resolve a stale reference to a different control. Issue new references on navigation and vary them between episodes. Support recoverable errors for stale or invalid targets without mutating state.

Observations must preserve labels, grouping, option values, disabled state, and visible validation messages. They must not contain recommendations, matching-target flags, hidden database fields, or a task-success signal. Condense with deterministic rules independent of the request and reference intent. Do not use a larger model to select the “relevant” controls for the student.

The app may reject an invalid assignee. It must still allow mechanically valid edits that violate the instruction, such as assigning an urgent job to Sam. The grader, rather than hidden tool-side policy, must catch those mistakes.

## Scope and difficulty controls

The following are proposed engineering budgets, not established limits of small models:

- Four to six records, one application, one active screen, fixed state outside the agent's edits.
- A few hundred tokens per observation, measured with the actual student tokenizer before freezing the interface.
- Common single-record paths of roughly three state-changing UI actions: open, select, save.
- A declared episode allowance, initially around 24 tool calls and 32 provider turns, including a turn for completion. Confirm reference solutions fit before fixing the limits.
- No coordinate actions, arbitrary scripts, concurrent edits, date arithmetic, attachments, or bulk policy-execution tool in the first suite.

Count the entire accumulated request, not just the latest observation. Full tool history can make a compact screen expensive over a longer episode. Start with bounded full history to keep the harness straightforward. If it exceeds the resource budget, evaluate an explicitly versioned context policy; do not silently give the teacher complete history and the student less information.

Increase one axis at a time: number of predicates, number of records to update, detail-only fields, an extra navigation step, then recoverable operational errors. Preserve a baseline slice at the original difficulty so improvements and regressions remain visible.

## Evaluation and experimental controls

Success requires the correct committed business state, no forbidden committed changes anywhere in the history, and normal completion within limits. A wrong edit followed by a restoring save still fails. A correct unsaved form does not pass. For version one, canceled draft edits are diagnostics rather than forbidden business mutations; state that convention explicitly, and allow stricter task-specific rules later.

Use fresh application state and conversation for every trial. The grader should compare all records and fields, not merely the requested assignee. Detect omitted targets, changes to excluded records, unintended status edits, duplicate records, and deletion. Keep protocol errors and no-op actions as diagnostics rather than automatically failing a successfully recovered trial.

Validate the grader independently with hand-checked examples: correct update, omitted update, wrong target, wrong field, forbidden change then restoration, correct no-op, unsaved draft, stale-reference recovery, and exhausted budget. An oracle script verifies solvability but does not replace independent grader checks.

### Baselines

Retain the existing three-way comparison on the same UI:

| Configuration | Interpretation | Execution |
| --- | --- | --- |
| Parser + scripted UI driver | Hand-written supported-language parser | Deterministic navigation, inspection, and editing through public tools |
| Model translator + scripted UI driver | Model emits the fixed intent schema using public names/options | The same driver and public tools |
| Tool-using model | Model interprets the instruction and chooses each UI action | Public tools chosen step by step |

The driver must discover record state and controls through the same public interface. It receives the parser/model output, never the reference intent. Count its tool calls, latency, and failures. Its application knowledge is part of the configured baseline and should be documented.

Optionally compare a domain-API agent with the same underlying business task. Report it separately because its action space is easier. A supplied-reference-intent oracle is an infrastructure test, not a competing model score.

The translator baseline is especially valuable here. This workflow can be solved by interpreting a policy once and executing a loop in ordinary code. If that system is more reliable and cheaper, the experiment has answered a useful question; there is no need to manufacture complexity to make autonomous UI decisions win.

### Splits, repetitions, and reporting

Begin with the design's 30 development tasks: ten each for direct, conditional, and exception requests. Keep composed requests as a separate capability slice initially. Cover both matching and nonmatching distractors, already-correct records, and zero, one, and multiple updates.

Before fine-tuning, freeze a separate test suite. Group related paraphrases, paired state variants, and templates within one split. Hold out language templates and combinations of supported predicates; use new names, IDs, row orders, and initial assignments. Randomizing record IDs alone is not evidence of generalization.

Use a separate challenge set for changed control order or detail-only fields. Label those results as transfer to a changed interface, rather than pooling them into the original task score. Any test failure used to design a fix becomes development information and requires test-set versioning.

Run three fresh trials per model/task configuration. Report mean single-attempt success, fraction passing all three, forbidden-change rate, errors, calls, no-ops, input/output tokens, latency, and peak memory. Repetitions of one task are correlated; confidence intervals should resample or otherwise account for task grouping. Keep infrastructure failures visible in the all-attempt denominator.

Do not interpret zero forbidden changes in a small pilot as proof of a very low deployment failure rate. Report sample counts and uncertainty, and keep resource costs separate from correctness. Hosted-model memory is unavailable; hosted latency includes service and network overhead.

## Luna validation before student training

First build one manually executable vertical slice with a deterministic initial state, public observations, tools, and an independent grader. Run a scripted solution to establish that the task is possible and Save truly persists. Then run Luna using exactly the information and actions intended for the student.

Use the 30 development tasks to identify whether failures arise from intent interpretation, control targeting, saving, stopping, malformed calls, or environment bugs. Review successes as well as failures to catch evaluator leakage and accidental shortcuts. Freeze task and interface versions once the workflow is coherent.

A reasonable provisional readiness gate is at least 95% trial success on the simple development slice, no observed forbidden changes there, and no unresolved environment/grader defects. These are local triage thresholds, not deployment guarantees or published standards. Predeclare them before the pilot, keep the harder capability slice separate, and avoid relaxing the gate after seeing results without recording that decision.

Luna success establishes that the task and interface can support a competent policy. It does not establish that a tiny student can learn that policy. Run an untuned student pilot before investing in a large demonstration corpus, so context overflow, decoding incompatibility, and latency problems surface early.

## Fine-tuning and replication

FunctionGemma-270M is a relevant candidate to test if a checkpoint has not already been chosen. Its special function-calling format and task-specific training requirements mean it needs its own verified adapter. [6](https://ai.google.dev/gemma/docs/functiongemma/model_card) It should not be selected solely because it is small; evaluate runtime support, the intended hardware, and performance on this task.

Generate training episodes from public requests and initial states. Let Luna operate through the same tools as the student. Accept a trajectory as a successful demonstration only if the independent grader passes it, and record the acceptance rate by task family. Otherwise, filtering can hide that the teacher only succeeds on easy examples.

Train on observable action decisions and termination behavior. Do not assume access to private teacher reasoning or make hidden reasoning a dependency. Tool observations provide context; the supervised targets should be the student-formatted assistant outputs. Include no-action examples, multiple valid action orders, and carefully validated recovery demonstrations. The official FunctionGemma guide explicitly includes larger-model distillation as a tool-calling fine-tuning use case. [9](https://ai.google.dev/gemma/docs/functiongemma/finetuning-with-functiongemma)

Use a small learning curve, such as 100, 500, and 2,000 successful training episodes, expanding only when development results justify it. Those are experimental budgets, not claims of sufficient data. Keep episode groups intact, balance task families, and evaluate complete rollouts rather than next-action accuracy alone.

After initial supervised training, inspect states reached by the student's own mistakes. On training/development episodes, collect corrected continuations from those states and validate the resulting behavior. Merely cloning clean teacher trajectories may leave the student unprepared for its own errors. WebRL's analysis of repetitive imitation-learning behavior is a reason to watch for loops, not a requirement to adopt its full RL method. [8](https://arxiv.org/html/2411.02337v2)

Replicate with the same frozen tasks, initial states, tool semantics, termination rules, and grader. Record model/checkpoint identifiers, prompts, tokenizer and chat format, training seed, sampling configuration, context policy, quantization, runtime, hardware, dataset version, and code revision. Teacher and student training formats can differ mechanically, but their public information and permitted actions must be equivalent.

If constrained decoding is used, declare it as part of the system and apply comparable validation across configurations. Grammar validity can reduce malformed outputs; it does not prove that the selected record or action is correct. Quantization and any teacher fallback are separate configurations, with their costs included.

## Implementation sequence in this repository

1. Add the record model, list/detail/draft state, save transition, and mutation log. Keep business state separate from UI state.
2. Implement the minimal tools against shared per-episode state. The existing `Tool::execute(&self, ...)` interface requires interior mutability or a small ownership change for shared mutable state.
3. Add hand-checked evaluator fixtures, one scripted solution, task generation, and JSONL trial records.
4. Add explicit total tool-call and output/context budgets. The inspected runner currently bounds network calls; that alone does not bound the number of tool calls in a response.
5. Make the app usable manually through a small frontend; validate that manual and tool interactions produce equivalent state transitions.
6. Run the Luna pilot, freeze the initial interface, then run the untuned student and compare translator/agent configurations.
7. Collect verified demonstrations, fine-tune, and repeat the frozen evaluation. Add a real browser adapter or external benchmark subset only when it answers a specific transfer question.

The next implementation decision is therefore small: a maintenance-record assignment workflow, a list and an edit form, and element-based tools. SDL rendering, broad browser compatibility, and training infrastructure need not precede that first evidence-producing experiment.

## Sources

Research checked on 13 September 2026. Paper-specific findings retain their original experimental context. Living documentation and local code can change.

1. Alexandre Drouin et al. **WorkArena: How Capable Are Web Agents at Solving Common Knowledge Work Tasks?** ICML 2024. [Paper](https://arxiv.org/abs/2403.07718); [inspected HTML version, sections 3–5 and Appendix B](https://arxiv.org/html/2403.07718v3). Used for enterprise task structure, observation complexity, element IDs, and coordinate-action ablations.
2. ServiceNow. **BrowserGym repository and README.** Living project, accessed September 2026. [Repository](https://github.com/ServiceNow/BrowserGym). Used for integration choices and custom-task support.
3. Farama Foundation. **MiniWoB++ Action Space.** Living documentation, accessed September 2026. [Documentation](https://miniwob.farama.org/content/action_space/). Used for element actions and configurable action subsets.
4. Léo Boisvert et al. **WorkArena++: Towards Compositional Planning and Reasoning-based Common Knowledge Work Tasks.** NeurIPS 2024; arXiv revised February 2025. [Paper](https://arxiv.org/abs/2407.05291). Used for workflow composition and trajectory-generation precedent.
5. Tianbao Xie et al. **OSWorld: Benchmarking Multimodal Agents for Open-Ended Tasks in Real Computer Environments.** 2024. [Paper](https://arxiv.org/abs/2404.07972). Used for real-application task setup and execution-based evaluation.
6. Google DeepMind. **FunctionGemma model card.** Updated 16 April 2026. [Model card](https://ai.google.dev/gemma/docs/functiongemma/model_card). Used for 270M size, text function calling, specialized formatting, intended fine-tuning, and Tiny Garden scope.
7. Minki Kang et al. **Distilling LLM Agent into Small Models with Retrieval and Code Tools.** NeurIPS 2025; arXiv revised November 2025. [Paper](https://arxiv.org/abs/2505.17612). Used for evidence of small tool-using students and its domain limitations.
8. Zehan Qi et al. **WebRL: Training LLM Web Agents via Self-Evolving Online Curriculum Reinforcement Learning.** ICLR 2025. [Paper](https://arxiv.org/abs/2411.02337); [inspected HTML version, Table 1 and section 3.2](https://arxiv.org/html/2411.02337v2). Used for the reported 8B improvement and imitation-learning loop analysis.
9. Google. **Fine-tuning with FunctionGemma.** Living documentation, accessed September 2026. [Guide](https://ai.google.dev/gemma/docs/functiongemma/finetuning-with-functiongemma). Used for task-specific tool-calling distillation guidance.
10. Local project sources, inspected September 2026: [task design](/Users/svetoslavangelov/code/agent-harness/src/tools/design.md), [entry point](/Users/svetoslavangelov/code/agent-harness/src/main.rs), [runner](/Users/svetoslavangelov/code/agent-harness/src/session/runner.rs), [tool interface](/Users/svetoslavangelov/code/agent-harness/src/tools/tool_interface.rs), and [client configuration](/Users/svetoslavangelov/code/agent-harness/src/client/openai_client_config.rs). Used to distinguish proposed functionality from the current implementation.
