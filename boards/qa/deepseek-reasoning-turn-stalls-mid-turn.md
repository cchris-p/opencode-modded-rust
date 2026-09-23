---
id: "BUG-038"
title: "DeepSeek reasoning turn stalls mid-turn and never completes"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
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
