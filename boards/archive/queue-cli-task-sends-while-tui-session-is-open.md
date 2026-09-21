---
id: "FEAT-021"
title: "Queue CLI task sends while TUI session is open"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "archived"
created: "2026-09-16"
---

# Queue CLI task sends while TUI session is open

## Archived

Archived 2026-09-21 at user request; not planned for implementation. Session prompt queuing and its
queued-message display are owned by `GATE-001` (done).

## Summary

When a CLI task prompt is sent to a session that is currently open in the TUI, enqueue that prompt for the session instead of rejecting it or allowing unsafe concurrent prompt execution.

## Why this exists

The desired CLI task workflow allows the user to keep a TUI session open while sending follow-up prompts from the shell. Those sends must target the same canonical session runtime and should appear in the TUI as normal session progress. If the session is busy or visible in the TUI, the CLI send should become ordered session work rather than racing the active prompt loop.

## Product Decisions

- Queue semantics and the queued-message display are owned by `GATE-001`; this card implements them and
  is not done until `GATE-001`'s Done-when and implementation constraints are satisfied.
- `--stream` on a send that is queued returns queued status immediately (target session, message ID,
  queue position) and follows/streams once the request becomes active. If the send starts immediately,
  it streams normally.
- Accept-time materialization: the server appends the accepted user message with a stable ID and returns
  it; the runner consumes that message rather than creating a second one.

## Scope

- Define the per-session prompt queue semantics used when CLI task sends target an open or busy TUI session.
- Ensure queued sends preserve submit order per target session.
- Ensure queued sends use the same server/session prompt runtime as normal TUI prompts.
- Ensure the TUI observes queued CLI prompts and their results through normal session updates.
- Return enough CLI output after enqueueing to show the target session and queued/submitted status.
- `--stream` follows a queued request once it becomes active; while queued it reports queued status
  immediately rather than pretending to stream.
- Add tests or smoke coverage that sends to a session while it is open or busy and verifies ordered execution.

## Non-goals

- Implementing the full `ort task` send/view surface; that is `FEAT-005`.
- Implementing default target selection; that is `FEAT-020`.
- Building a broad task dashboard; that is `FEAT-019`.
- Changing normal `ort` TUI launch attach/reuse behavior.

## Done when

- A CLI task send targeting a TUI-open session is accepted and queued.
- Queued prompts execute in submit order for that session.
- The TUI displays the queued prompt and result through the same session state it uses for TUI-originated prompts.
- CLI output identifies the target session and reports queued/submitted status clearly.
- Concurrent sends do not interleave prompt execution or corrupt session history.
- The behavior satisfies `invariants/cli-task-targeting.md`.

## Recommended verification

- Open a session in the TUI, send a prompt from the CLI to that session, and confirm the prompt/result appears in the TUI.
- While a prompt is running, send a second CLI prompt and confirm it waits until the first completes.
- Send multiple CLI prompts quickly and confirm session history preserves submit order.
- Confirm explicit target overrides and selected default target behavior still route to the expected session.

## Related Items

- `FEAT-005` Copy Cline-style CLI task send conventions
- `FEAT-019` Add CLI status visibility for tasks and background sessions
- `FEAT-020` Add default task target selection for CLI sends

## Notes

- Created on 2026-09-16 from the invariant that sending to a TUI-open session queues the request instead of conflicting with the open TUI view.
