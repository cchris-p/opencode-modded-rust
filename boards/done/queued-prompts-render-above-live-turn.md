---
id: "BUG-073"
title: "Queued prompts render as ingested below the live turn"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/message-queuing.md"
status: "done"
created: "2026-11-12"
---

# Queued prompts render as ingested below the live turn

## Summary

While a turn was running, a newly submitted prompt was accepted into the server queue but the TUI
rendered it *below* the live assistant output (the "build text" and running tool row). During the
tool-call phase the row could look fully ingested because the queued state was inferred from
transcript position rather than the server's explicit queue marker. The queue therefore read as
ignored, and the only way to make it visibly advance was to hard-stop the turn.

Reported by the user 2026-11-11/12.

## Root Cause

- The TUI derived `is_queued` solely from `idx > pending_assistant_idx` in
  `render_messages`/`message_sig`/`render_message_body`.
- A tool-call step is finalized with `completed_at` set and `finish_reason: "tool-calls"` *before*
  its tools execute, so `pending_assistant_idx` is `None` during exactly the phase where the user is
  most likely to submit an ad-hoc message. The queued message then rendered as a normal ingested
  user row after the live assistant/tool output, with no reliable queue signal.
- The server already knew the truth (`queued_pending` metadata) but did not expose it to clients.

## Fix

- Server exposes an explicit `queued` boolean on the message DTO, derived from the existing
  `queued_pending` metadata (`crates/opencode-server/src/routes.rs:951`, `:1144`,
  `is_queued_pending` at `:3140`).
- TUI carries `queued` on `Message` (`crates/opencode-tui/src/context/session_context.rs:39`), maps
  it from the API (`crates/opencode-tui/src/api.rs:127`,
  `crates/opencode-tui/src/app/app.rs:5208`), and marks optimistic messages queued when a turn is
  already running.
- Transcript rendering now uses a render order that lifts pending queued user messages above the live
  assistant turn while keeping the `QUEUED` badge
  (`crates/opencode-tui/src/components/session.rs:1209`, `:1228`, `:1238`).
- Queued state prefers the explicit marker and keeps the old position heuristic only as a fallback
  for older snapshots. When the server consumes the prompt it clears the marker, and the message
  automatically returns to normal transcript order above the assistant turn it started.

## Queue / Abort Semantics (unchanged)

- Abort cancels only the active run; waiting queued prompts are preserved
  (`crates/opencode-server/src/routes.rs:4962`).
- The per-session drain loop then runs the queued prompt as the next turn
  (`drain_session_queue` at `crates/opencode-server/src/routes.rs:3042`; regression test
  `abort_cancels_active_run_without_clearing_queued_prompts` at `:1639`).
- Ingestion still happens at the turn boundary, not mid-turn. This card is a presentation fix only.
  Mid-turn ingestion at a thinking/tool-call boundary would be a separate server/agent-loop change.

## QA Report - 2026-11-12

- Checkout: `development` after PR #189; `cargo` workspace version 0.1.2.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-tui -p opencode-server`: **pass**.
- `cargo test -p opencode-tui --lib components::session::tests`: **29 passed, 0 failed**.
- `cargo test -p opencode-server --lib message_info_marks_accepted_prompts_as_queued`: **pass**.
- New regression tests:
  - `queued_messages_render_above_the_live_assistant_turn`
    (`components/session.rs:3071`)
  - `queued_message_stays_above_a_finalized_tool_call_turn` (`:3126`) — reproduces the reported
    finalized `tool-calls` phase where `pending_assistant_idx` is `None`.
  - `ingested_message_returns_to_transcript_order_and_drops_the_badge` (`:3187`)
  - `message_info_marks_accepted_prompts_as_queued` (server DTO)
- Result: **PASS** for the queued-queue presentation contract.

## Merge Closeout - 2026-11-12

- Merged into `development` via PR #189 (merge commit `060b73b`).
- Remote branch `fix/BUG-073-queued-message-queue-presentation` deleted; local branch deleted.
- `invariants/message-queuing.md` decision 7 / invariant 7 updated from "no reorder in V1" to the
  queue-above-live-turn presentation.
- The PR's `native (macos-14)` check was cancelled before starting because GitHub retired the
  `macos-14` runner image on 2026-11-02. It was a non-required check and did not block the merge.
  The workflow runner labels were migrated separately.

## Related Items

- `GATE-001` Gate: prompt queuing and queue display must match vanilla (owns the queue contract and
  the original `QUEUED` badge).
- `BUG-054` Queued user-message rows jitter while a turn streams (stability of the same rows).
- `FEAT-075` Animated running indicator on in-flight tool calls (made the live tool phase obvious;
  this card makes the queue above it accurate).
- `BUG-051` Continuation prompt flashes / unordered (adjacent in-flight message stability).
