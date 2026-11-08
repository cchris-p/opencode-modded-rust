---
id: "BUG-056"
title: "Session continues instead of stopping when the question tool is presented to the user"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-30"
---

# Session continues instead of stopping when the question tool is presented to the user

## Summary

When the agent presents a question to the user (the `question` tool), the `ort` session should
**stop and wait** for the user's answer, then ingest that answer and continue with it. Reported
behavior: the session keeps running when a question is presented instead of parking on the pending
question. The user asked to match vanilla OpenCode exactly wherever possible.

This is an investigation-first card: the exact continuation shape is not yet pinned down, and the
question path has several plausible mechanisms (missing callback on one entrypoint, a run loop that
does not actually suspend on the ask, or a synthetic auto-continue turn). Root cause is not yet
confirmed; hypotheses and the diagnostics needed to confirm one are recorded below.

## Reported behavior

User report (live use, `ort` TUI):

> ort continues its session when presenting a question to the user. It should stop so it can ingest
> that question and process it.

Intended behavior (parity target): when the agent asks a question, the run suspends at the question
and does not issue further model steps or tool calls until the user replies or rejects; the answer
is then delivered to the model as the question tool result and the run resumes.

Observed behavior needs one of the following to be pinned down before fixing (see Diagnostics):

- V1 — The `question` prompt is shown, but the run keeps producing further model steps / tool calls
  before the user answers (the run does not block on the ask).
- V2 — The prompt is shown and the run does block, but once the user answers the session continues
  **without ingesting the answer** (the answer is not the thing the next step is built from).
- V3 — The agent asks in plain assistant text and the session keeps running instead of ending the
  turn and waiting for the user.

## Vanilla reference

Frozen reference line per `AGENTS.md`: `$HOME/repos/opencode-modded` `dev` at
`f54ce313b99a6661d7758ad042f7a6e05c8e0972`.

- `packages/core/src/tool/question.ts`: the `question` tool calls `QuestionV2.ask(...)` and awaits it;
  the tool does not return until the ask resolves (reply/reject). The model receives answers only via
  `toModelOutput`, i.e. the question is a real blocking tool result in the same turn.
- `packages/core/src/question.ts`: `ask` stores a deferred per request and awaits it; `reply`
  resolves it, `reject` fails it; pending entries are deleted on completion and rejected on shutdown.
- Net effect: the session is parked on the tool while a question is pending. There is no second model
  step and no auto-generated turn between the question and the user's answer.

Parity requirement: the Rust session must observe the same suspend-until-answered contract for the
`question` tool on the `ort` prompt path.

## Code evidence

Expected blocking path (main session):

- `QuestionTool::execute` awaits `ctx.question(...)` and returns the model-facing answer summary:
  `crates/opencode-tool/src/question.rs:96-121` (call at `:110`).
- `ToolContext::question` delegates to the injected callback; with no callback it errors immediately
  (`"Question callback not configured"`) instead of blocking: `crates/opencode-tool/src/tool.rs:618-630`.
- The prompt loop installs the callback and awaits tool execution, so the loop should park on the ask:
  `crates/opencode-session/src/prompt.rs:1562-1566`, call site at `:1615`.
- The server's question callback inserts the request, broadcasts `question.asked`, then awaits the
  waiter before returning the tool result: `crates/opencode-server/src/routes.rs:4114-4171`
  (`question_resolution_result(rx.await)` at `:4169`).
- The main prompt runner installs it: `crates/opencode-server/src/routes.rs:4400-4404`.
- TUI presents and answers the pending request through the session-scoped routes:
  `crates/opencode-tui/src/app/app.rs:3352-3408` (open), `:3544-3599` (submit), `:3600-3620` (reject).

Candidate non-blocking / continuation paths:

- A **second** `SessionPrompt` in the server is constructed **without** `with_ask_question_callback`
  (child/subagent sessions): `crates/opencode-server/src/routes.rs:3325-3344`; a `question` tool call
  from that runner hits `ToolContext::question`'s no-callback error and the loop continues.
- Compaction **auto-continue** injects a synthetic user turn with
  `generate_continue_message()` = `"Continue if you have next steps, or stop and ask for clarification
  if you are unsure how to proceed."`: `crates/opencode-session/src/compaction.rs:578-651`, text at
  `:827-828`; the TUI already filters these as synthetic (`crates/opencode-tui/src/app/app.rs:4911`).
- Loop termination/continuation logic that decides whether to issue another model step:
  `crates/opencode-session/src/prompt.rs:1188-1212` (turn-finished check) and `:1637-1647`
  (`finish_reason != "tool-calls"` break).

## Hypotheses (confidence-ranked, pre-measurement)

- **H1 (high) — The `ort` prompt path is not actually suspending on the ask.** Either the callback
  is not installed on the exact runner handling the turn, or the ask resolves/errors immediately
  (e.g. missing callback -> `ToolContext::question` error at `tool.rs:625`), so the loop falls through
  to another model step. The subagent runner at `routes.rs:3325` is a confirmed instance of a runner
  with no question callback; confirm whether the reported turn used that path or a different one.
- **H2 (medium) — The loop issues another step after a question-ending turn.** The turn-finished
  check (`prompt.rs:1188-1212`) and/or `finish_reason` handling (`:1637-1647`) classify a turn that
  ended on a question as unfinished, so the loop re-prompts the model while the question is pending.
- **H3 (medium) — A synthetic auto-continue turn is the "continuation."** After compaction, the
  auto-continue user turn (`compaction.rs:578-651`) drives another model step; if a question was in
  flight around compaction the session visibly keeps going. Confirm whether the observation was tied
  to compaction/context overflow.
- **H4 (low-medium) — Answer ingestion is the real defect (V2).** The ask blocks and resolves, but the
  next model step is not built from the question tool result (e.g. the answer arrives as a separate
  user turn), so the session "continues" without processing the question. Check the message list at
  the point of answer for the `ToolResult` part.
- **H5 (low) — Plain-text question (V3).** The model asked in text rather than the tool, and the loop
  did not stop because the turn also carried tool calls / a non-stop finish reason. This is a model
  behavior issue, not a tool-path bug; rule it out from the transcript.

## Diagnostics to capture (do this before fixing)

1. Reproduce on `ort-build` then `ort`: trigger a `question` and record whether any new assistant
   step or tool call appears between the prompt opening and the answer.
2. Export/capture the live session (`opencode session inspect <id>` or the DB) and record, in order,
   the assistant `ToolCall(question)` part, any intervening model steps, and the `ToolResult` for the
   question call. This distinguishes V1/V2/V3 and H1/H2/H4.
3. Confirm which runner drove the turn: top-level prompt (`routes.rs:4400`) vs the child/subagent
   runner (`routes.rs:3325`, no question callback). Record the agent and whether a subagent was used.
4. Check `traces/server.log` for `question.asked` / `question.replied` broadcasts and any
   `Tool execution error` / `"Question callback not configured"` around the event.
5. Note whether the observation correlated with compaction/context overflow (tests H3).

## Required solution direction

- Make the `ort` prompt path genuinely suspend on a pending question: no further model step or tool
  execution until the ask resolves, on every runner that can execute the `question` tool.
- Ensure every session runner that can run tools has the question callback installed (or the
  `question` tool withheld), so a no-callback runner can never silently downgrade a question into an
  immediate error-and-continue.
- When the ask resolves, deliver the answers to the model as the question tool result in the same
  run, exactly as vanilla `toModelOutput` does.
- Do not introduce synthetic turns as a substitute for waiting; verify any auto-continue path cannot
  run past a pending question.
- Match vanilla behavior exactly where practical; document any deliberate deviation in `GATE-002`.

## Scope

- The Rust `ort` session prompt loop and server question callback wiring
  (`crates/opencode-session/src/prompt.rs`, `crates/opencode-server/src/routes.rs`,
  `crates/opencode-tool/src/question.rs`/`tool.rs`).
- Any runner (`routes.rs:3325` child/subagent path) that omits the question callback.
- Tests that assert the run stays parked on a question and resumes only after reply/reject.

## Non-goals

- Reworking the question schema/model output or the TUI question layout (`FEAT-038`, `FEAT-041`,
  `BUG-039`).
- Changing the question permission decision (`FEAT-043`).
- CLI/direct-run question handling (`CLI-009`, on hold behind `GATE-002`).
- General agent reasoning quality; this card is about the runtime suspend/continue contract.

## Done when

- Presenting a `question` in `ort` suspends the run: no further model step or tool call occurs until
  the user replies or rejects.
- After reply, the answers are ingested as the question tool result and the same run resumes; after
  reject, the run resumes with a distinguishable rejection result.
- No runner can execute the `question` tool without a working callback (either wired or tool withheld).
- A regression test covers the suspend-until-answered contract on the prompt path.
- `cargo test` passes for `opencode-session`, `opencode-server`, and `opencode-tool`, plus
  `cargo fmt --all -- --check` and `cargo clippy` for the touched crates.

## Recommended verification

- Unit/regression: a mocked provider + a `question` tool call asserts the loop does not issue a
  second provider step while the ask is unresolved, and does resume after the callback resolves.
- Confirm a no-callback runner returns an error and does not silently continue a run (or does not
  offer the tool).
- Live: `ort-build` then `ort`; ask a question whose answer materially changes the next step and
  confirm the model's next action depends on the answer, with no intervening step.
- Side-by-side parity note against the frozen reference commit recorded on `GATE-002`.

## Related Items

- `GATE-002` Gate: question tool must match vanilla OpenCode exactly (qa) - parent gate; this card is
  a runtime-contract gap it did not cover.
- `FEAT-038` Question schema and tool contract parity (qa) - made the tool use the callback instead of
  stdin; establishes the callback contract this card relies on.
- `FEAT-040` Question runtime event and lifecycle parity (qa) - ask/reply/reject events and waiter
  cleanup; adjacent to the suspend behavior.
- `BUG-016` Plan-mode session stalls after tool calls without tool results (qa) - opposite failure
  (run stops without results) on the same loop; useful contrast when isolating the loop paths.
- `BUG-039` question prompt layout (done) - TUI question prompt work; not the runtime contract.

## Notes

- 2026-09-30: Created from a live-use report that `ort` keeps going when a question is presented.
  Framed as an investigation card because the report does not yet say which continuation shape was
  observed; the code path that *should* block is clear, so the first job is to find where the actual
  run diverges from it.
- The strongest known concrete gap is the child/subagent runner constructed without the question
  callback (`routes.rs:3325-3344`); it must be confirmed against the reported session before treating
  it as the cause.
- The auto-continue turn (`compaction.rs:578-651`) is a real "session continues" mechanism in the
  product and must be checked for interaction with a pending question, even though vanilla also
  auto-continues after compaction.
- 2026-09-30: Additional live-use report (defect B of `BUG-058`): the run also keeps going when the
  user **quits out of** a question (Esc/reject via `app.rs:3600-3626`) or leaves it **unanswered**,
  instead of parking until an answer is produced. Confirms H1/H2 and adds the reject/abandon path to
  the diagnostics: capture whether a `question.rejected` resolves the ask and then issues further
  model steps, and whether that is vanilla-compatible (vanilla resolves the ask on reject and
  resumes, so a "stop forever" expectation would be a deliberate deviation to record on `GATE-002`).
  The separate answer-revision defect (defect A) is tracked by `BUG-058` and is out of scope here.
