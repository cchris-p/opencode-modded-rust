---
id: "BUG-038"
title: "DeepSeek reasoning turn stalls mid-turn and never completes"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-09-23"
---

# DeepSeek reasoning turn stalls mid-turn and never completes

## Summary

On the default `deepseek/deepseek-flash` model, a turn can emit a reasoning block and then a partial
assistant text part, after which the session stops producing anything and the turn never completes.
No error is surfaced. The session stays `active`, the TUI keeps it "in progress", and the user is
left with a truncated reply until they abandon the session.

Captured transcript: `docs/transcripts/why-was-bug-035-moved-to-hold.md`.

## Reported behavior

- The assistant shows a thinking block, then a short partial reply, and then nothing.
- No tool call, no tool result, no error, and no completion is shown.
- The session is never marked finished, so the export is still tagged "in progress".

## Evidence

Session `ses_b5b1b754b9fa4e16a869ede1b3e76de7` (`Why was BUG-035 moved to hold?`), workspace
`/Users/cchrisleepyles/repos/opencode-modded-rust`:

- Created `2026-09-23T15:40:09Z`; last update `2026-09-23T15:40:12Z` (~3.7s later); session
  `status = active`.
- Two messages: one user prompt and one assistant message containing only two parts — a `reasoning`
  part ("...Let me look for board-related files in the repo.") and a `text` part ("I'll look for the
  board"), both at `2026-09-23T15:40:12Z`.
- The assistant message recorded no `provider_id`/`model_id`, and input/output/reasoning token usage
  is `0`, so the turn never reached a terminal/completion record.
- No `error` part, no tool call, and no tool result. The reply stops mid-sentence exactly where a
  tool call ("look for the board") would be expected.
- The exported transcript carries the in-progress warning because the session was still `active`
  when exported (generated from the `status == active` check,
  `crates/opencode-tui/src/app/app.rs:4278`).

## Why this exists

The daily-driver workflow depends on a turn either finishing or failing loudly. Here deepseek
reasoning began normally but the turn wedged silently: the model appears to have intended a tool
call, and no tool call, tool result, error, or completion followed. The user cannot tell whether the
model is still working or dead, and must abandon the session. This is a correctness/robustness defect
in the streaming/agent loop, not a display issue.

## Suspected root cause

Not confirmed. The stall occurs right at a reasoning→tool-call transition, and the turn records no
terminal state, so the failure is most likely in the stream/agent loop rather than in the UI.

## Hypotheses (confidence-ranked)

- **H1 — Reasoning passback / tool-call split fails.** After emitting `reasoning_content`, the
  follow-up request or the split tool-call handling fails so the loop never dispatches a tool or a
  next request. Prior fix/history:
  `deepseek-tool-loop-reasoning-passback-and-split-toolcall` (done).
- **H2 — Malformed/empty tool call dropped.** The model emitted a tool intent the parser discarded,
  leaving the loop waiting forever for a tool result.
- **H3 — Stream reader exits without a terminal record.** The stream ends and the runtime neither
  records usage nor surfaces an error, leaving `status = active`.

## Code evidence

- Reasoning and split tool calls are parsed together, and `reasoning_content` is surfaced
  specifically so it can be echoed back on follow-ups:
  `crates/opencode-provider/src/stream.rs:229-235`, `:285-316`.
- The earlier deepseek reasoning passback/split-toolcall work is recorded in
  `boards/done/deepseek-tool-loop-reasoning-passback-and-split-toolcall.md`; this card covers a
  remaining/regressed stall on that path.
- Session status stays `active` with zero usage for the stalled turn (see Evidence); the TUI only
  ends the in-progress state on a terminal update, and the export warning is keyed off that status
  (`crates/opencode-tui/src/app/app.rs:4278`).

## Scope

- Reproduce a deepseek turn that emits reasoning and then stalls before/at a tool call.
- Identify the loop stage where the turn stops and why no tool call, tool result, or error is
  recorded.
- Ensure a turn always terminates: dispatch the tool call, retry, or surface an error and clear the
  in-progress state.
- Ensure the persisted session status reflects reality (not stuck `active`).

## Non-goals

- Reasoning display semantics; `BUG-022` owns collapsed/count rendering.
- The TUI responsiveness freeze while the reasoning stream itself is healthy; `BUG-027` owns that.
- Provider transport rewrites beyond what the confirmed cause requires.

## Done when

- The stall is reproduced and the loop stage where it stops is named with evidence.
- A stalled turn can no longer leave the session silently `active` without a terminal record.
- Reasoning→tool-call transitions on deepseek either complete or fail with a visible error.
- `cargo check` and `cargo test` pass for the touched crates.

## Recommended verification

- Re-run the captured prompt ("Why was BUG-035 moved to hold?") on `deepseek/deepseek-flash` and
  confirm it either completes or errors visibly.
- Inspect the persisted assistant message for a terminal record (finish reason, usage, provider,
  model).
- Confirm the session leaves `active` once the turn ends and that the export no longer warns "still
  in progress".
- Add a regression test around the reasoning→tool-call transition path.

## Related Items

- `BUG-027` Session keeps freezing during thinking mode - UI freeze while reasoning streams, but the
  turn still completes (distinct symptom; this card is a silent non-completion).
- `BUG-023` Root-cause why the plan-mode session stalled after tool calls.
- `BUG-025` Concurrent server sync deletes sessions and messages.
- `BUG-019` Escape does not interrupt the running session.
- `deepseek-tool-loop-reasoning-passback-and-split-toolcall` (done) - prior deepseek reasoning
  passback/tool-call fix.

## Notes

- Captured export: `docs/transcripts/why-was-bug-035-moved-to-hold.md`.
- The stalled prompt is trivial and read-only, so the wedge is not caused by a slow or large tool.
- Relevant files: `crates/opencode-provider/src/stream.rs`,
  `crates/opencode-provider/src/deepseek.rs`, `crates/opencode-session/src/prompt.rs`,
  `crates/opencode-server/src/routes.rs`, `crates/opencode-tui/src/app/app.rs`.

## Dev Notes

Implemented a provider-stream idle timeout (2026-09-23).

- Added `opencode_provider::stream::with_idle_timeout` and
  `DEFAULT_STREAM_IDLE_TIMEOUT` (90s) in `crates/opencode-provider/src/stream.rs`.
- `SessionPrompt::loop_inner` now wraps every provider stream with
  `with_idle_timeout(stream, DEFAULT_STREAM_IDLE_TIMEOUT)` right after
  `provider.chat_stream`, so a quiet stream is converted into a visible
  `StreamEvent::Error` instead of an unbounded `stream.next().await`.
- Root cause (confirmed by code path, not live reproduction): the inner stream
  loop had no bound on a stream that stops emitting without closing. When that
  happened, `loop_inner` never returned, `finish_run` never ran,
  `SESSION_RUN_STATUS` stayed `Busy`, and the session stayed `active` with no
  terminal record. That matches the captured evidence (session `active`, no
  usage, partial reasoning+text, no error). The wrapper bounds the wait.
- The emitted `StreamEvent::Error` flows through the existing error arm, which
  returns `Err`. The server then records an error assistant message with
  `finish_reason: "error"` and the queue drain returns the session to `idle`, so
  the turn fails loudly instead of hanging.
- Deliberately did **not** add a "stream closed without a finish reason" error at
  the session layer: providers such as Google end the SSE body without an
  explicit terminal event on normal completion, so that heuristic would have
  broken them. The idle timeout targets the actual stall (silence) and is
  provider-agnostic.

## Verification

- `cargo test -p opencode-provider idle_timeout` -> 2 passed
  (`stalled_stream_ends_with_visible_error`,
  `active_stream_passes_events_through_unchanged`).
- `cargo test -p opencode-session stalled` -> 1 passed
  (`stalled_stream_error_fails_the_turn_visibly`).
- `cargo test -p opencode-provider` -> 99 passed + 7 integration passed, 0 failed.
- `cargo test -p opencode-session` -> 159 passed + 11 integration passed, 0 failed.
- `cargo check --workspace` clean; `cargo fmt --all -- --check` clean.
- Live `ort-build`/`ort` reproduction on `deepseek/deepseek-flash` was **not**
  run in this headless environment; the behavior is covered by the stream-level
  and session-level regression tests. Re-run the captured prompt on the PR
  branch for live confirmation.
