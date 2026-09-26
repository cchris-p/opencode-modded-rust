---
id: "BUG-047"
title: "Assistant turn progress is not persisted until the run completes"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-26"
---

# Assistant turn progress is not persisted until the run completes

## Summary

Assistant messages and tool results produced during a turn exist only in server memory until the run
returns; they are written to the database only at turn completion. If a run stalls, panics, or is
interrupted, the entire turn is lost: the persisted session still ends at the last user prompt, and a
continuation has no choice but to resume from that stale prompt. This is the mechanism behind the
observed "continues as `Yes`" behavior and part of why a stalled session looks like it never did any
work.

## Evidence

- Live stall `ses_2c1168ee88f644f49315ca1736064d20` (2026-09-26): the server's in-memory session had
  **12 messages** while the Rust DB (`~/Library/Application Support/opencode/opencode.db`) had only
  **7** — messages `[7]-[11]` (the post-`Yes` turn: tool calls and results) were never persisted.
- `opencode session inspect` reports: *"Session is active; the last persisted message is a user
  prompt with no assistant reply."* The last DB-persisted message is the user prompt `Yes`.
- Persistence is applied in bulk at the end of `run_prompt_turn`
  (`crates/opencode-server/src/routes.rs:4064-4073`, `apply_session_snapshot` +
  `persist_sessions_if_enabled`), not incrementally as messages/parts are produced.
- Reference comparison: vanilla persists incrementally with `session.updatePart(...)` /
  `session.updateMessage(...)` throughout `process()` (for example
  `packages/opencode/src/session/processor.ts`), so completed and in-progress parts survive a stall.
- Directly feeds `BUG-044` (continuation resumes from the last *persisted* message).

## Why this exists

A stalled or crashed run must be inspectable and resumable. If nothing is persisted until the turn
ends, then exactly the cases that need diagnosis and recovery (stalls, panics, interrupts) leave no
durable trace, and the next turn repeats work or resumes from the wrong point.

## Scope

- Persist assistant messages/parts incrementally as they are produced (tool calls, tool results,
  completed assistant steps), matching the reference's `updatePart`/`updateMessage` behavior, instead
  of only at turn end.
- Ensure the persisted state is consistent enough that a stalled run can be inspected and continued
  from the latest recorded part.
- Keep the turn-end persistence as a final flush; this is additive, not a replacement.

## Non-goals

- The terminal-state / interrupt contract; that is `BUG-043`.
- Choosing the continuation point; that is `BUG-044`.
- Capturing server stderr/panics; that is `BUG-045`.
- DB schema or storage-engine changes beyond what incremental writes require.

## Done when

- A completed assistant step and its tool results are durable before the next provider request is made.
- After a stalled/interrupted run, the persisted session includes the latest produced parts (not just
  the last user prompt).
- Continuation can resume from the latest persisted part.

## Recommended verification

- Test: run a turn that stalls after a tool result and assert the DB contains the tool result.
- Test: after an interrupted run, the persisted session ends at the latest produced part.
- `cargo test -p opencode-server -p opencode-session -p opencode-storage`; `cargo check --workspace`.

## Related Items

- `BUG-043` A session run can end without a terminal state - the stall this makes unrecoverable.
- `BUG-044` Continuing a session resumes from an earlier user prompt - direct consumer.
- `BUG-016` / `BUG-023` Unresolved tool-call state and the plan-mode stall persistence gap.
- `FEAT-035` Resume an interrupted session from where it left off - relies on persisted state.

## Notes

- Captured stall: `ses_2c1168ee88f644f49315ca1736064d20` (2026-09-26); in-memory 12 vs DB 7.
- Relevant files: `crates/opencode-server/src/routes.rs` (`run_prompt_turn`,
  `apply_session_snapshot`, `persist_sessions_if_enabled`),
  `crates/opencode-session/src/session.rs`, `crates/opencode-storage/`.
