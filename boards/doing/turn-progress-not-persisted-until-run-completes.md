---
id: "BUG-047"
title: "Assistant turn progress is not persisted until the run completes"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "doing"
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
## Dev Notes - 2026-09-26

- `crates/opencode-server/src/routes.rs`: the per-turn `update_task` in `run_prompt_turn` now calls
  `persist_sessions_if_enabled` whenever a snapshot adds a message (assistant step or tool result),
  so turn progress is durable before the next provider request instead of only at turn end. The
  end-of-turn flush remains.

## Verification - 2026-09-26

- Covered by `opencode-server` tests (64 + 3 integration passed) and `cargo check --workspace` clean.
- Live check pending: after a stalled/interrupted run, confirm the DB contains the latest produced
  parts rather than only the last user prompt (`BUG-044` continuation should then resume correctly).
## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/118
  (branch `bug/BUG-043-047-run-terminal-state-persistence`, base `development`, handoff H-012 Pass B).
## Reopened - 2026-09-26 (the first fix was insufficient)

The first fix persisted only when the message **count** changed. Streamed assistant content (text and
reasoning) and tool calls are written **in place** into the last assistant message, so they were not
durable until a new message was appended or the turn ended. A live session confirmed it:
`opencode session inspect` reported 120 in-memory and 120 DB messages, but *"the last assistant
message has no persisted parts"* - the in-flight chunk was missing. On exit/resume that chunk is
lost, which is the "resumes before the chunk that hadn't outputted yet" symptom on `BUG-044`.

## Dev Notes - 2026-09-26 (reopened)

- `crates/opencode-server/src/routes.rs`: `update_task` now computes a cheap
  `session_content_fingerprint` (message count, part count, text/reasoning bytes) and persists when
  the fingerprint changes, throttled to `STREAM_PERSIST_INTERVAL` (1s). New messages still persist
  immediately. This flushes in-flight assistant output during a run.

## Verification - 2026-09-26 (reopened)

- New tests: `fingerprint_changes_when_in_flight_text_grows`,
  `fingerprint_changes_when_message_count_grows`.
- `cargo test -p opencode-server` -> 66 + 3 integration passed; `cargo test -p opencode-session`
  166 passed (2 pre-existing unrelated failures); `cargo check --workspace` and `cargo fmt --all`
  clean.
- Pending live confirmation: after a mid-turn exit, `opencode session inspect` should show the last
  assistant message's parts persisted (not "no persisted parts").

## QA Report (self-QA) - 2026-09-26 - PARTIAL

QA: BUG-047 — in-flight assistant output is persisted during a run.

- commit: `34bc972` (`development`); model `deepseek/deepseek-flash`; measured against the DB
  (`messages.data`) and the API.
- Tests (PASS): `fingerprint_changes_when_in_flight_text_grows`,
  `fingerprint_changes_when_message_count_grows`.
- Live (partial): 4s into a streaming turn, the DB assistant row had no parts (`[]`) in 3/3 measured
  runs, even when the API already showed a reasoning part; the reasoning part only appeared in the DB
  after the abort. Post-abort the assistant row did contain the reasoning part (so not "no persisted
  parts"), but the appended terminal text is subject to the `BUG-043` clobber race and the
  `error`/`finish` metadata is never persisted (see `BUG-043` QA Report).
- result: **PARTIAL** — the narrow "parts exist after a mid-turn exit" ask held in these runs, but
  timely in-flight flushing was not demonstrated and terminal progress can still be lost. Keep open
  alongside `BUG-043`.

## Live Context - 2026-09-26 (shared observation)

A live two-session capture (see `BUG-051`) showed the latest assistant message's persisted `data`
bytes growing monotonically (`130 -> ... -> 1185`) during a streaming turn, i.e. in-flight content
*was* flushed; at the same time the in-memory session list alternated with a reverted snapshot.
Dated context only; `qa` status and scope unchanged.

## Dev Notes - 2026-09-27 (reopened, H-014)

- The remaining gap was on the persist/merge path, not the flush trigger: `merge_session_snapshot`
  replaced the shared session wholesale, so a stale run snapshot could drop the latest in-flight
  parts; and storage serialized only `message.parts`, so metadata (`error` / `finish_reason` /
  `completed_at`) could never be restored on reload.
- Fix: the merge is now monotonic and per-message/per-part, so streamed progress is never rolled
  back; `messages.data` now stores parts plus metadata and reads accept the legacy bare-array form
  (`crates/opencode-storage/src/repository.rs`). The existing fingerprint flush and end-of-turn
  flush are unchanged.

## Verification - 2026-09-27

- `cargo test -p opencode-storage` -> metadata round-trip + legacy-format load pass.
- Live (isolated HOME): after a streaming turn and a reload, the last assistant message still carries
  its parts and `completed_at` (persisted, not `metadata: HashMap::new()`).
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/127
  (branch `bug/BUG-043-047-051-snapshot-merge-and-metadata`, base `development`, handoff H-014).