---
id: "FEAT-048"
title: "Background subagents and notification injection"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
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

## Dev Notes

- **Gate.** Background subagents are gated by `experimental.background_subagents` in `opencode.json`
  or `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true`, resolved by
  `Config::experimental_background_subagents()` (`crates/opencode-config/src/schema.rs`). Off by
  default. With the gate off, `background: true` fails with the reference message
  `Background subagents require OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true` and creates nothing.
- **Task tool.** `crates/opencode-tool/src/task.rs` now has a real background branch: it resolves the
  subagent, creates the child session, calls the new async `background_subsession` callback, and
  returns immediately with `<task ... state="running">` plus the reference `BACKGROUND_STARTED`
  guidance. `ToolContext` gained `experimental_background_subagents` and `background_subsession`;
  `PromptSubsessionCallback` now returns `SubsessionPromptOutcome::{Completed,Backgrounded}` so a
  promoted run can return a running result.
- **Server runner.** `crates/opencode-server/src/routes.rs` runs every `task` subagent in a detached
  task registered in `TASK_RUNS` (keyed by child session). Explicit background runs are marked
  promoted from the start; a foreground run stays promotable. On completion/failure of a promoted
  run the server appends a synthetic `<task ... state="completed|error">` user message to the parent
  session (`inject_background_result`) and broadcasts `session.updated`.
- **Cancellation.** `run_child_task` selects on an abort token; a foreground child run is stopped by
  `AbortOnDrop` when the awaiting tool future is dropped (parent turn aborted), and an explicit
  background run uses the launching turn's `ctx.abort` token.
- **`ctrl+b` / capability.** New `POST /session/{id}/background` promotes a running foreground task
  (`promote_session_background`). TUI registers `session_background` = `ctrl+b`
  (`context/keybind.rs`), calls the endpoint (`api.rs` + `app.rs`), and appends `ctrl+b background`
  to the task hint row when the capability is on. The TUI resolves the capability from config/env in
  `App::new` (`AppContext::experimental_background_subagents`).
- **Deviation from reference (documented).** The reference runs a parent turn when injecting the
  background result (`ops.prompt`). This product appends the synthetic result as a parent-session
  user message, so the parent observes it on its next turn rather than auto-resuming. Also, the
  background run is tied to the launching turn's abort token only while that run is active.
- **Verification.** `cargo fmt --all`; `cargo check --workspace` green. Tests green:
  `cargo test -p opencode-tool -p opencode-agent -p opencode-config -p opencode-server --lib`
  (including `background_requires_the_experimental_gate`,
  `background_starts_without_waiting_and_reports_running_state`,
  `promoted_foreground_task_reports_running_state`,
  `render_background_task_message_matches_reference`,
  `inject_background_result_appends_synthetic_parent_message`), `cargo test -p opencode-tui --lib`
  (135 passed), and `cargo test -p opencode-session --lib` excluding the pre-existing environmental
  `instruction::` failures. Manual `ort-build`/`ort` background smoke is pending human verification.
- **Known limitation / follow-up.** Parent auto-resume after a background completion is not
  implemented (see deviation); a scoped follow-up card can add the parent-turn resumption.

### PR Link

- PR #99 (https://github.com/cchris-p/opencode-modded-rust/pull/99) — `feature/FEAT-048-background-subagents` → `development`.
- Program: GATE-004 (H-009), PR 5/7. Awaiting human test/merge on the checked-out branch.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-046` task tool contract parity - owns the `background`/schema plumbing.
- `FEAT-047` TUI subagent navigation - shares the hint row and child sessions.