---
id: "FEAT-035"
title: "Resume an interrupted session from where it left off"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-21"
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
