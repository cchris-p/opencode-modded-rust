---
id: "BUG-051"
title: "The sent continuation prompt flashes and never settles as one ordered message after interrupt-then-continue"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "doing"
created: "2026-09-26"
---

# The sent continuation prompt flashes and never settles as one ordered message after interrupt-then-continue

## Summary

After interrupting a running session and sending a new prompt (the "continue" path), the sent
prompt keeps flashing/re-rendering instead of behaving like a normal, single, ordered message in the
conversation.

**Classified 2026-09-26 from live evidence: the run is progressing, not stuck.** The defect is the
unstable display: the session's in-memory message list flip-flops between two snapshots, so each
poll/frame renders a different list and the continuation prompt (and surrounding messages) flash and
appear out of order. This card is a display/state-stability bug, not a stall.

## Confirmed live evidence (2026-09-26)

Runtime target: Rust product (`target/debug/opencode`), three TUI+serve pairs (`:3187` pid 55953,
`:3188` pid 90502, `:3189` pid 91261) sharing one DB
`~/Library/Application Support/opencode/opencode.db`.

Two live sessions observed (the operator's two ongoing sessions):

- A = `ses_9ffd6dfb41f74d99a3976731d0a7a4b2` ("qa feedback for ctrl t...") on `:3188` - the flashing
  session.
- B = `ses_99fd5d426a9a43e2902199a3b0ee0472` ("How do we resolve this board item?...") on `:3189` -
  control, progressing normally.

What was measured:

- **A is progressing, not frozen.** Persisted DB `messages` count went `39 -> 40`, and the latest
  assistant message `data` bytes grew monotonically `130 -> 285 -> 325 -> 448 -> 525 -> 565 -> 590
  -> 640 -> 1185` (in-place streamed part growth, ~`BUG-047` flush). The run is emitting slowly.
- **A's in-memory message list flip-flops between two snapshots.** `GET /session/{A}/message` on
  `:3188` alternated over 12 samples (4s apart) between `n=54` (last `msg_f203af785ffe...`) and
  `n=40` (last `msg_f202e91acffe...`). The last message ID toggles between two different values, so
  consecutive polls return materially different (and differently ordered) lists.
- **B does not flip-flop.** `:3189` returned a stable `n=36` list while the last assistant reasoning
  part grew `9082 -> ... -> 10733` bytes, then advanced `n=37 -> 38`.
- **Concurrent-writer contention is present.** `traces/server.log` shows repeated
  `failed to sync sessions to storage: Query error: ... database is locked`, with multiple servers
  sharing the one DB.

## Repro contract

- Surface: TUI session view after `Esc` interrupt then sending a new prompt.
- Entity: A = `ses_9ffd6dfb41f74d99a3976731d0a7a4b2` (workspace
  `/Users/cchrisleepyles/repos/opencode-modded-rust`).
- Expected: the sent continuation prompt appears once, in chronological order, and stays put.
- Actual: the message list alternates between two snapshots (`n=54` forward run state and `n=40`
  reverted/persisted state); the continuation prompt and neighbors re-appear/re-order each poll.

## Failure point (leading)

Server `state.sessions[A]` for `:3188` alternates between a **forward run snapshot** (`n=54`,
includes messages not yet persisted) and a **reverted snapshot** (`n=40`, matching the persisted DB).
Every TUI poll/stream snapshot therefore sees a different list, which renders as flashing and
out-of-order messages.

This is consistent with the stale-snapshot clobber already described in `BUG-043`'s QA report: the
still-draining `update_task` re-applies an older snapshot (`merge_session_snapshot` does
`*existing = snapshot`), and the `BUG-047` throttled flush races it. Two servers sharing one DB
(`database is locked`) is a plausible amplifier. The exact writer ordering still needs to be proven
in implementation, but the two-snapshot alternation is directly observed.

## Scope

- Stop `state.sessions[session]` from alternating between a forward run snapshot and a reverted
  snapshot while a turn is streaming (fix the clobber/merge race; coordinate with `BUG-043` and
  `BUG-047`).
- Ensure the sent continuation prompt is rendered exactly once, in chronological order, at a stable
  index across frames.
- Add regression coverage: poll the message endpoint during a streaming turn and assert the returned
  list is monotonic (no count decrease, no last-ID reversion to an older snapshot), plus a TUI frame
  test asserting the prompt does not duplicate/move.

## Non-goals

- Diagnosing or fixing a stall; live evidence shows these runs are progressing, so this is not the
  stall path.
- The liveness/observability UX gap ("can I tell slow-stream from wedge?"); that is the
  thinking-display / liveness cluster (e.g. `BUG-022`).
- The terminal-state contract, in-flight persistence, or continuation-point selection; those are
  `BUG-043` / `BUG-047` / `BUG-044`. This card owns the resulting **display instability**.
- The shared-DB lock itself; it is a candidate separate card (concurrent-writer contention over one
  `opencode.db`), distinct from the done `BUG-025`.

## Done when

- During an interrupt-then-continue streaming turn, `GET /session/{id}/message` returns a monotonic
  list (no count decrease, no reversion to an older snapshot), at least for the session under a
  single server.
- The continuation prompt renders once, in order, at a stable index across frames in the TUI.
- A regression test covers the monotonic-list / single-render behavior.

## Recommended verification

- Headless: start a streaming turn, poll `GET /session/{id}/message` every ~2s, and assert `n` is
  non-decreasing and the last ID never reverts to a previously seen older ID.
- Re-run the two-session observation above and confirm A no longer alternates `n=54`/`n=40`.
- TUI frame capture (or ratatui `TestBackend`) asserting the prompt appears once and does not move
  across frames.
- `cargo test -p opencode-server` (snapshot merge / monotonic list) and `cargo test -p opencode-tui`.

## Related Items

- `BUG-043` A session run can end without a terminal state - its QA report describes the same
  stale-snapshot clobber (`merge_session_snapshot` / `update_task`) now seen as the flashing.
- `BUG-047` Assistant turn progress is not persisted until the run completes - the throttled flush
  that races the snapshot merge.
- `BUG-044` Continuing a session resumes from an earlier user prompt - same snapshot/continuation
  state, adjacent symptom.
- `BUG-046` / `BUG-038` never-idle stream / deepseek reasoning stall - background on slow streams.
- `BUG-022` Thinking shows a count, not live content - the related "can't tell it's alive" UX gap.
- `BUG-025` Concurrent servers delete each other's sessions/messages (done) - adjacent multi-server
  DB contention; the observed `database is locked` is a distinct candidate.
- `FEAT-035` Resume an interrupted session from where it left off - prior resume behavior.
- `GATE-001` Prompt queuing and queue display - ordered display and optimistic reconciliation by ID.

## Notes

- Created 2026-09-26 from the operator's observation while watching new messages on
  `deepseek/deepseek-flash`.
- **Classified 2026-09-26:** the operator and an independent live check both confirm the runs are
  slowly progressing; "stuck / false activity" is not the defect and has been removed from this
  card. The confirmed defect is that the sent continuation message flashes and is not an ordered
  message. Root display cause: the in-memory message list alternates between two snapshots.
- Cross-checked against the operator's other-agent read (same day): it also found the runs are alive
  and not frozen; this card adds the two-snapshot alternation as the concrete flashing mechanism.
- Evidence commands used: `GET /session/{id}/status`, `GET /session/{id}/message` on `:3188`/`:3189`,
  read-only `sqlite3` on `opencode.db`, and `traces/server.log`.
- Likely files: `crates/opencode-server/src/routes.rs` (`update_task`, `merge_session_snapshot`,
  `sync_sessions_to_storage`, `drain_session_queue`), and
  `crates/opencode-tui/src/context/session_context.rs` / `app/app.rs` (message store reconciliation).

## Dev Notes - 2026-09-27 (H-014)

- Confirmed this is the same stale-snapshot clobber as `BUG-043`/`BUG-047`: `merge_session_snapshot`
  did `*existing = snapshot`, so a stale run snapshot reverted the shared list (observed `n=54`
  forward vs `n=40` reverted) and every TUI poll rendered a different list.
- Fix: monotonic, id-keyed `merge_session_snapshot` (`crates/opencode-server/src/routes.rs`) never
  reduces the message count or reverts the last message; it advances streamed parts and keeps newer
  terminal state. No TUI-side masking was added, per the handoff.

## Verification - 2026-09-27

- Live (isolated HOME, no shared DB) interrupt-then-continue probe: polled `GET /session/{id}/message`
  every second across two turns; message count was non-decreasing (`1 -> 2 -> 3 -> 4`) with no
  last-id reversion.
- TUI regression test `components::session::tests::continuation_prompt_renders_once_in_order_and_stays_put`
  asserts the sent prompt renders exactly once, after the answer, at a stable row across frames.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/127
  (branch `bug/BUG-043-047-051-snapshot-merge-and-metadata`, base `development`, handoff H-014).