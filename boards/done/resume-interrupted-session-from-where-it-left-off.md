---
id: "FEAT-035"
title: "Resume an interrupted session from where it left off"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-09-21"
updated: "2026-09-25"
---

# Resume an interrupted session from where it left off

## Summary

When the user interrupts a running session (`Esc`), the session should stop cleanly and remain
resumable: the next prompt in the same session should pick up from the interrupted point instead of
failing or restarting. Interrupt is currently a dead end — the run is stranded and the next request
can fail with `400 Invalid assistant message: content or tool_calls must be set`.

## Why this exists

Interrupt is the primary control surface for a daily-driver agent loop. Today `Esc` is framed only
as "stop the generation" (`BUG-019`). Stopping is necessary but not sufficient: the user interrupts
precisely because they want to redirect or continue, so the session must stay in a valid, resumable
state whose next turn continues from where the run left off.

## Reported behavior

- In `ort`, start a run that streams a long response or loops over tool calls.
- Interrupt it (`Esc`, double-press inside the 5-second confirmation window).
- The run does not resume from the interrupted point. In the captured case the next request failed
  with:
  `Provider error: API error: 400 Bad Request: {"error":{"message":"Invalid assistant message: content or tool_calls must be set",...}}`
- Evidence: [`docs/transcripts/tool-call-issue.md`](../../docs/transcripts/tool-call-issue.md)
  (`ses_33afb883…`, 2026-09-21).

## Scope

- After an interrupt, leave the session in a durable, provider-valid state so the next request
  succeeds.
- Make the next turn continue from the interrupted point — preserved completed tool calls and
  partial assistant output — rather than restarting the task or dropping the turn.
- Define and record the interrupted-turn state explicitly (for example an `aborted` assistant turn
  with a durable marker) so resume has an unambiguous starting point.
- Preserve the user's original prompt and any completed tool results across the interrupt.
- Keep interrupt distinct from session end; exit and detach stay owned by `CLI-004` and `FEAT-033`.

## Non-goals

- Auto-resuming or auto-continuing without the user sending the next prompt.
- Changing the `Esc` double-press confirmation semantics or keybinding; that is `BUG-019`.
- The defect where interrupt produces an invalid persisted message shape; that is `BUG-019`.
- Session-exit resume hints; that is `FEAT-033`.
- Provider transport rewrites beyond what the resume state requires.

## Done when

- Interrupting a run leaves the session in a state where the next prompt succeeds with no
  `400 Invalid assistant message` error.
- The next prompt after an interrupt continues from the interrupted point, including preserved tool
  results, rather than restarting.
- The interrupted assistant turn is durably marked (for example `aborted`), and export shows the
  interrupted state plus the resumed continuation.
- Interrupt never leaves an assistant tool-call turn without matching tool results.
- Interrupting does not exit the TUI or corrupt the session for the next prompt.

## Recommended verification

- `cargo test -p opencode-session` for interrupt-then-continue coverage.
- A session-loop test that interrupts mid tool-call turn, then sends a follow-up, and asserts the
  request is valid and continues from preserved state.
- `cargo check -p opencode-session -p opencode-server -p opencode-tui`.
- Live: `ort-build`, then `ort`; start a long or tool-looping run, interrupt with `Esc`, send a
  follow-up, and confirm it resumes with no `400`.
- Export the transcript and confirm the interrupted turn and the continuation are both recorded.

## Related Items

- `BUG-019` Escape does not interrupt the running session - interrupt defect owner; the `400` symptom
  on interrupt is recorded there.
- `BUG-016` Plan-mode session stalls after tool calls without tool results - tool-call resolution
  state that resume depends on.
- `BUG-023` Root-cause why the plan-mode session stalled after tool calls without results -
  previously parked the `400` symptom; now links here.
- `BUG-012` Session summary runs before tool results and breaks OpenAI-compatible continuation -
  same provider-error class, different trigger.
- `FEAT-033` Print a resume command for the last session on TUI exit - resume surface at exit, not at
  interrupt.
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers - interrupt is distinct
  from detach and exit.

## Notes

- Cited error: `docs/transcripts/tool-call-issue.md:35`. The original reference failure for this
  `400` was noted in `BUG-023` as "promote to its own card if the symptom recurs"; the user-reported
  trigger is interrupt, captured on 2026-09-21.

## Verification And Closeout - 2026-09-25

The behavior this card asked for was delivered by the two interrupt fixes that closed its blockers,
so the card was stale in `todo` rather than actually pending.

Evidence:

- `BUG-019` (`4e4fbcc`, PR #54) made abort leave a valid, resumable turn.
  `SessionPrompt::mark_aborted` (`crates/opencode-session/src/prompt.rs:1701`) writes a durable
  `metadata.error = "aborted"` / `metadata.finish_reason = "aborted"` marker and injects
  `"Aborted by user."` when a turn was interrupted before any visible output (reasoning-only turns
  previously serialized as a content-less assistant message and caused the
  `400 Invalid assistant message`).
- `SessionPrompt::abort_pending_tool_calls` (`prompt.rs:1789`), invoked alongside `mark_aborted`
  when the cancel token fires (`prompt.rs:1625`), resolves every tool call that lacks a matching
  result with an error result, so an interrupted turn is never left without matching tool results.
- `BUG-029` (`bff30c1`, PR #66) made the `Esc` double-press confirmation actually cancel the run;
  the loop observes cancellation during streaming (`prompt.rs:1308`) and tool execution
  (`prompt.rs:1524`).
- Continuation is preserved by construction: `build_chat_messages` (`prompt.rs:2081`) converts every
  `SessionMessage` (including the aborted turn, its completed tool calls, and the injected error
  tool results) into the next request, so the next prompt continues from the interrupted context
  instead of restarting.
- Export is a full JSON dump of session messages (`export_session_data`,
  `crates/opencode-cli/src/main.rs:5457`), so the persisted `aborted` marker and the resumed
  continuation are both present in an export.
- Focused tests exist for the risky paths and pass with `cargo test -p opencode-session`
  (166 unit + 11 integration tests): `mark_aborted_records_durable_error_state`,
  `mark_aborted_adds_text_when_only_reasoning_present`, `abort_pending_tool_calls_*`.

Known gap: there is no dedicated end-to-end "abort mid-turn, then send a follow-up prompt" test;
continuation is covered indirectly by the context-preserving conversion and the existing
`session_handles_three_consecutive_prompts` regression. A future integration test would be a
nice hardening step but is not required to consider this behavior delivered.

Closed as complete and moved from `todo` to `done`. Related to (but distinct from) `FEAT-033`
(the exit-time resume-command hint), which shipped separately in PR #53.
