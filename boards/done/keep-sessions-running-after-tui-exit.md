---
id: "FEAT-002"
title: "Keep sessions running after TUI exit"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "wiki/v1.md"
status: "done"
created: "2026-08-29"
---

# Keep sessions running after TUI exit

## Summary

Allow a user to leave the TUI while an active session continues its task and response in the background, then reopen the product and revisit that session with live progress still visible.

## Why this exists

The current Rust product already has server-side session execution and live session update streaming, but the default local TUI lifecycle still tears down its in-process backend on exit. That breaks the expected daily-driver workflow for long-running tasks.

## Current Rust behavior

- The server already executes session prompts asynchronously and broadcasts `session.updated` and `session.status` events.
- The TUI already reopens a selected session by syncing messages from the server and listening to the server event stream.
- The default `opencode` / `opencode tui` launch path previously started a local server inside the same process and stopped it when the TUI exited.

## Reference behavior check

- The TypeScript reference line has partial support through durable session admission, active-session discovery, history replay, and live event subscriptions.
- The reference line still treats execution ownership as process-local and does not fully deliver crash-safe detached session continuation as a completed product behavior.

## Scope

- Verify the current Rust and TypeScript behavior before changing product invariants.
- Make the default Rust TUI launch path preserve background session execution after the user exits the TUI.
- Make revisiting the product reconnect to the same local backend for that workspace so existing live session updates remain visible.
- Preserve the existing attach-to-server and session-list reopen flows.

## Non-goals

- Cross-machine session handoff.
- Crash-recovery for a killed backend process.
- Redesigning the session event model.
- Building a full background-job orchestration system beyond normal session continuation.

## Done when

- A user can start a session in the default Rust TUI, exit the TUI, and allow the session to continue running.
- Relaunching the Rust TUI in the same workspace reconnects to the same local backend instead of silently replacing it.
- Reopening the still-running session shows current live execution through the existing sync and event-stream behavior.
- The relevant invariants state this product requirement explicitly.

## Recommended verification

- Start a long-running session in the TUI, exit the TUI, relaunch `opencode`, reopen the same session, and confirm new assistant output continues appearing.
- Verify completed sessions still appear correctly after reconnect.
- Verify explicit `opencode attach <url>` behavior remains unchanged.

## Dev Notes

- Changed the CLI TUI boot path to start a detached local `serve` process instead of an in-process server that is aborted on TUI exit.
- Persisted a per-workspace local server record under the user's state directory so later launches reconnect to the same backend.
- Kept reuse workspace-scoped: if another OpenCode server is already on the requested port without a matching record, the CLI now fails safely and tells the user to use `opencode attach` or another `--port`.

## Verification

- `cargo fmt`
- `cargo check -p opencode-cli`

## PR

- Pending

## Completion

- Implemented and ready for merge on 2026-08-29.
