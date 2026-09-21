---
id: "FEAT-048"
title: "Background subagents and notification injection"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# Background subagents and notification injection

## Summary

Child of `GATE-004` (parity gap 4). Implement reference behavior for background subagents: an
experimental-gated async `task` run that returns immediately, injects a synthetic completion/error
message into the parent session, and can be triggered from a running foreground task with `ctrl+b`.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 4.

## Problem

- `run_in_background` is accepted by the Rust task tool but never read
  (`crates/opencode-tool/src/task.rs:28-29`); every task runs synchronously in-line.
- There is no background job registry, no completion notification, and no parent message injection.
- There is no `session_background`/`ctrl+b` keybind or capability flag in the TUI.

## Vanilla reference

Reference `f54ce313b99a`:

- `background` parameter and `experimentalBackgroundSubagents` gate; off-gate fails with an explicit
  message (`packages/opencode/src/tool/task.ts:56-62,97-102`).
- Background start/promote and `notify` inject a synthetic `<task ...>` completion or error message
  into the parent session (`:227-319`).
- Started/updated guidance text (`:25-41`).
- `session_background` = `ctrl+b`, "Background synchronous subagents"
  (`packages/tui/src/config/keybind.ts:98`).
- The "view subagents" hint row shows `ctrl+b background` for running foreground tasks when the
  capability is enabled (`packages/tui/src/routes/session/index.tsx:1516-1529`).

## Scope / deliverables

- Add a config/runtime flag equivalent to `experimentalBackgroundSubagents` (config key and/or env)
  and gate the `background` parameter on it.
- Implement background execution so `task` returns immediately with a running-state output, and the
  subagent session continues in the background.
- On completion or failure, append a synthetic completion/error message to the parent session in the
  reference format so the parent agent sees the result.
- Add `session_background` (`ctrl+b`) to background a running foreground task, and show it in the hint
  row when the capability is enabled.
- Expose the capability to the TUI the way the reference exposes
  `capabilities.experimentalBackgroundSubagents`.

## Acceptance criteria

- With the feature disabled, `background: true` fails with a clear reference-style error and does not
  start a background run.
- With the feature enabled, `task` with `background: true` returns a running-state output
  immediately and the subagent session keeps running.
- On completion, the parent session receives a synthetic message containing the reference
  `<task ... state="completed">` result; on failure, a `<task ... state="error">` result.
- `ctrl+b` backgrounds a running foreground task and the hint row reflects availability.
- Cancelling the parent/run cancels the background task.
- `cargo test -p opencode-tool -p opencode-session -p opencode-tui` passes with background tests.

## Verification

- Unit/integration tests: gate off errors; gate on returns immediately; completion injects the parent
  message; failure injects an error message; cancel propagates.
- Manual: enable the flag, run a background task, and confirm the parent is notified without polling.
- Side-by-side with the reference task tool for the started/completed/error output text.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-046` task tool contract parity - owns the `background`/schema plumbing.
- `FEAT-047` TUI subagent navigation - shares the hint row and child sessions.
