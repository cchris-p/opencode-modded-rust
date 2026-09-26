---
id: "BUG-046"
title: "A provider stream that never goes idle is unbounded (endless reasoning)"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-09-26"
---

# A provider stream that never goes idle is unbounded (endless reasoning)

## Summary

The provider stream guard added for `BUG-038` only fires on **silence**: it resets its timer every time
an event arrives. A provider (observed with `deepseek/deepseek-flash`) that keeps emitting reasoning
or deltas without ever finishing, closing, or going quiet will therefore never trip the timeout, and
the turn can run "forever" — matching the user's report of being "stuck at thinking forever". There is
no bound on total turn time or reasoning tokens.

## Evidence

- Stream guard implementation: `crates/opencode-provider/src/stream.rs:138-165`
  (`with_idle_timeout`) wraps `tokio::time::timeout(idle_timeout, stream.next())` **per event**;
  `DEFAULT_STREAM_IDLE_TIMEOUT` is 90s. Any event resets the wait, so a continuously-emitting stream is
  never idle.
- `BUG-038` notes that this idle timeout targets "silence" specifically and is provider-agnostic; it
  deliberately does not add a "stream closed without finish reason" check.
- Live stall `ses_2c1168ee88f644f49315ca1736064d20` (2026-09-26): the run wedged on
  `deepseek/deepseek-flash` with the last step's reasoning/text/tool-call state left unresolved
  (see `BUG-043`). The exact reason it stopped is not yet confirmed because server logs are discarded
  (`BUG-045`), so an endless non-idle reasoning stream remains a live hypothesis.

## Why this exists

Daily-driver turns must be bounded in wall-clock time as well as by silence. A model that streams
reasoning indefinitely (or a provider/bridge that emits keep-alive deltas) can hold a turn open with
the stream never idle, so the existing guard never applies. Neither the Rust runtime nor the vanilla
reference has a total turn/step bound, so this is a real robustness gap for reasoning models.

## Scope

- Add a bounded budget for a single provider step (and/or the whole turn) — wall-clock and/or
  reasoning/completion token cap — configurable, defaulting to a safe value.
- On exceeding the budget, cancel the provider stream and convert it into a visible
  `StreamEvent::Error` so the turn terminates with a durable error instead of running on.
- Keep the existing silence-based idle timeout; this is additive.
- Record the chosen default and configuration surface.

## Non-goals

- The run-terminal-state / interrupt contract; that is `BUG-043`.
- The silence idle timeout itself; that is `BUG-038`.
- Counting or displaying reasoning tokens; that is the `/thinking` display work.
- Provider transport rewrites beyond what the bound requires.

## Done when

- A provider stream that emits events forever is stopped within the configured budget and surfaces a
  visible error.
- The turn reaches a terminal state after the bound fires.
- The default and configuration for the bound are documented.

## Recommended verification

- Test: a mock stream that never ends but emits deltas is terminated at the configured bound with an
  error event.
- Test: a stream that goes silent still trips the existing idle timeout (no regression).
- `cargo test -p opencode-provider -p opencode-session`; `cargo check --workspace`.

## Related Items

- `BUG-038` DeepSeek reasoning turn stalls mid-turn - the silence idle timeout this extends.
- `BUG-043` A session run can end without a terminal state - the consumer of the terminal error.
- `BUG-045` Server runtime errors are discarded - needed to confirm the endless-stream hypothesis.
- `BUG-044` Continuing a session resumes from an earlier prompt - related deepseek continuation work.

## Notes

- Captured stall: `ses_2c1168ee88f644f49315ca1736064d20` (2026-09-26).
- Relevant files: `crates/opencode-provider/src/stream.rs`, `crates/opencode-provider/src/deepseek.rs`,
  `crates/opencode-session/src/prompt.rs`.

## Reproduction notes (user-reported, 2026-09-26)

- Emergent, not deterministic: not tied to a specific command; it reappears only after starting a
  **fresh session** and letting the agent run for a while. Cannot be reproduced on demand.
- The model "self-correcting" (for example wrapping commands in `timeout`) is not a fix; this bound
  must live at the provider/runtime layer and not depend on model behavior or memory.
- Verification is by fault injection (a mock stream that keeps emitting and never finishes is stopped
  at the configured budget), not by waiting for a live recurrence. Live recurrence is QA evidence,
  classified later with the `BUG-045` server log.
## Dev Notes - 2026-09-26

- `crates/opencode-provider/src/stream.rs`:
  - Added `DEFAULT_STREAM_BUDGET` (15 min) and `stream_budget_from_env()`
    (`OPENCODE_STREAM_BUDGET_MS` override; `0` disables).
  - Added `with_stream_budget(stream, budget)`: bounds the whole provider step by wall clock and ends
    the stream with `StreamEvent::Error` on exhaustion. Unlike `with_idle_timeout`, it cannot be
    reset by keep-alive deltas.
- `crates/opencode-session/src/prompt.rs`: the provider stream is wrapped with both
  `with_idle_timeout` (silence) and `with_stream_budget` (total step time).

## Verification - 2026-09-26

- New test `never_idle_stream_is_stopped_by_budget` (a never-ending, never-idle stream ends with a
  budget error).
- `cargo test -p opencode-provider` -> 104 lib + 7 integration passed, 0 failed.
- `cargo check --workspace` clean; `cargo fmt --all` clean.