---
id: "BUG-019"
title: "Escape does not interrupt the running session"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-09-19"
---

# Escape does not interrupt the running session

## Summary

The TUI treats `Esc` as the session interrupt key, but aborting does not actually stop the
running session. The server abort endpoints are stubs that only confirm the session exists and
return `{"aborted": true}` without cancelling the in-flight prompt. The TUI meanwhile optimistically
marks the session idle, so the visible state and the real runtime state disagree: generation keeps
running and streaming after the user interrupts.

## Desired behavior

- Pressing `Esc` (double-press within the confirmation window, matching the reference) actually
  interrupts the active session.
- After interruption, server-side generation stops, the session run status returns to idle, and the
  transcript records the aborted/error state instead of silently continuing.
- The server is the source of truth for run status; the TUI must not claim idle while the server is
  still busy.

## Reported behavior

- Start a prompt that streams a long response (or runs a long tool loop).
- Press `Esc` twice within the 5-second confirmation window.
- The prompt spinner/status can flip to idle in the TUI, but the response keeps streaming and tool
  calls keep executing until the run finishes on its own.
- There is no durable "aborted" marker in the session; the transcript looks like a normal completion.

## Recurrence - 2026-09-21 (issue reappeared)

The interrupt path still does not leave the session in a usable state after interruption. Interrupting
a running session causes the next request to fail with an invalid assistant-message shape:

- Error: `Provider error: API error: 400 Bad Request: {"error":{"message":"Invalid assistant message: content or tool_calls must be set",...}}`
- Evidence: [`docs/transcripts/tool-call-issue.md`](../../docs/transcripts/tool-call-issue.md)
  (`ses_33afb883…`, 2026-09-21).

Root cause (confirmed 2026-09-21):

- The aborted turn in `ses_33afb883…` persisted as a single assistant message holding only a
  `reasoning` part (message `msg_f3d8cd34cffek00hrEcxXQ4IPV`, no text and no tool call).
- `SessionPrompt::mark_aborted` treated a `Reasoning` part as a visible part, so it did not inject the
  `"Aborted by user."` text for a run interrupted during its thinking phase.
- `convert_assistant_parts` then emitted an assistant message with `content: null`, no `tool_calls`,
  and only `reasoning_content`. DeepSeek (OpenAI-compatible) rejects that shape, so the follow-up
  prompt 400s.
- The separate desired behavior, "interrupt should let the next turn pick up from where the run left
  off", is tracked by `FEAT-035`. This card owns the interrupt defect: abort must leave a valid,
  resumable session state.

## Why this exists

Stopping a runaway or wrong generation is the primary safety valve for a daily-driver agent loop.
If `Esc` only changes local UI state while the server keeps spending tokens and mutating the
workspace through tools, the user loses control of the session while believing it stopped.

## Investigation - 2026-09-19

Current code evidence:

- TUI interrupt path: `crates/opencode-tui/src/app/app.rs:469-495` matches `session_interrupt`,
  calls `client.abort_session(&session_id)`, then unconditionally
  `self.set_session_status(&session_id, SessionStatus::Idle)` and syncs the spinner.
- TUI double-press confirmation: `crates/opencode-tui/src/components/prompt.rs:617-625`
  (`register_interrupt_keypress`), 5-second window
  (`INTERRUPT_CONFIRM_WINDOW_SECS`, `prompt.rs:29`). Double-press matches the reference behavior in
  `$HOME/repos/opencode-modded` `packages/tui/src/component/prompt/index.tsx:408-419`.
- Server abort endpoints are no-op stubs:
  - `crates/opencode-server/src/routes.rs:2060` `abort_session`
  - `crates/opencode-server/src/routes.rs:2049` `abort_prompt`
  Both only check that the session exists, then return `{"aborted": true}`. Neither cancels the
  running generation nor clears run status.
- The prompt task builds a fresh `SessionPrompt` inside the spawned task and holds it local to that
  task (`crates/opencode-server/src/routes.rs:1964`), so no external handle can cancel it.
- The session layer already supports cancellation: `SessionPrompt::cancel` at
  `crates/opencode-session/src/prompt.rs:771-779`, backed by a `CancellationToken` registry
  (`start`/`resume`/`is_running`, `prompt.rs:734-760`). The server simply never calls it.
- Server run status is a separate global (`SESSION_RUN_STATUS`, `routes.rs:296-324`) driven by
  `set_session_run_status`; abort does not touch it, so `GET /session/status` keeps reporting busy
  until the prompt task eventually finishes (`routes.rs:2038`).

Likely failure area to confirm:

- The abort route needs to cancel the active prompt run (via the session-layer cancellation token or
  an equivalent registry), set run status idle, and emit the resulting status/session update so the
  TUI stops optimistically lying about state.

## Scope

- Make the server abort endpoints actually cancel the in-flight session prompt/tool loop.
- Make abort clear the server run status to idle and broadcast the status change.
- Wire the TUI interrupt path to the real server truth instead of assuming idle.
- Ensure an aborted run leaves a durable, visible aborted/error state on the session rather than a
  normal completion.
- Cancel pending tool calls safely so no unresolved tool-call part is left behind (see `BUG-016`).
- Leave the aborted assistant turn in a provider-valid shape so a follow-up prompt does not fail with
  `400 Invalid assistant message: content or tool_calls must be set`.

## Non-goals

- Changing the `Esc` double-press confirmation semantics or the interrupt keybinding.
- Redesigning run-status transport beyond what is needed to report abort truthfully.
- Changing provider-level streaming semantics unrelated to cancellation.
- Implementing graceful "abort current tool then stop" policy decisions beyond stopping safely.

## Done when

- Pressing `Esc` twice during an active run stops server-side generation and any pending tool calls.
- `GET /session/status` returns idle for the session after abort, and the TUI shows idle for the same
  reason rather than by local assumption.
- The transcript records an aborted/error state for the interrupted assistant turn.
- No tool call is left unresolved after an abort.
- The interrupted session is left in a provider-valid state, so the next prompt does not fail with
  `400 Invalid assistant message: content or tool_calls must be set`.
- Interrupting a run does not exit the TUI or corrupt the session for the next prompt.
- Resume-after-interrupt behavior itself is owned by `FEAT-035`; this card only guarantees the state
  it depends on.

## Recommended verification

- `cargo test -p opencode-session` for cancel-token/abort coverage.
- `cargo test -p opencode-server` or an integration test that issues `POST /session/{id}/abort` and
  asserts the run stops and status returns to idle.
- `cargo check -p opencode-session -p opencode-server -p opencode-tui`.
- Live: run `ort-build`, then `ort`; start a long generation, press `Esc` twice, confirm streaming
  stops, status returns to idle, and the transcript shows the abort.
- Confirm a subsequent prompt in the same session works normally.

## Related Items

- `BUG-016` Plan-mode session stalls after tool calls without tool results - abort must also leave
  resolved tool-call state.
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - same prompt-input key handling
  surface; explicitly deferred interrupt-semantics changes.
- `FEAT-017` Plan explicit detach command behavior for TUI-launched servers - distinguished normal
  exit from detach; abort must not be treated as exit.

## Dev Notes

- Added a server-side active prompt registry so `/session/{id}/abort` and
  `/session/{id}/prompt/abort` cancel the in-flight `SessionPrompt` instead of returning a stubbed
  success.
- Abort now clears and broadcasts server run status as idle after a real cancellation request.
- The prompt loop marks interrupted assistant turns with `error = "aborted"` and
  `finish_reason = "aborted"`, and preserves the existing pending-tool-call abort repair.
- The TUI no longer sets local session status to idle optimistically after `Esc`; it waits for the
  server status event.
- Follow-up fix (2026-09-21): `SessionPrompt::mark_aborted` no longer counts a `Reasoning` part as
  visible content. A run interrupted during its thinking phase now receives the `"Aborted by user."`
  text part, keeping the aborted assistant turn provider-valid.
- Follow-up fix (2026-09-21): `convert_assistant_parts` in `opencode-provider` skips an assistant
  message that carries neither text nor tool calls (reasoning-only aborted turns) so it can never
  emit an assistant message with `content: null` and no `tool_calls`.

## Verification

- `cargo check -p opencode-session -p opencode-server -p opencode-tui`
- `cargo check -p opencode-session -p opencode-provider`
- `cargo test -p opencode-session mark_aborted` (includes new
  `mark_aborted_adds_text_when_only_reasoning_present`)
- `cargo test -p opencode-provider reasoning` (includes new
  `reasoning_only_assistant_message_is_skipped`)
- `cargo test -p opencode-provider` -> 7 passed
- `cargo test -p opencode-session` -> 2 pre-existing environment-dependent failures in
  `instruction::tests::test_find_up_*` (macOS tempdir symlink canonicalization), present on
  `origin/development` before this change and unrelated to it.
- Attempted one invalid combined Cargo test filter command first; reran the focused tests
  separately because Cargo accepts only one test-name filter before harness args.

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/48
- Follow-up fix PR: https://github.com/cchris-p/opencode-modded-rust/pull/54

## Completion

- 2026-09-19: PR #48 merged into `development`; BUG-019 feature branch and temporary worktree
  cleaned up. Keeping this item in `qa` for observation and revisit if the issue appears again.
- 2026-09-21: Recurrence observed and root-caused (reasoning-only aborted turn). Follow-up fix merged
  via PR #54; item moves to `done`.
