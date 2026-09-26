---
id: "BUG-043"
title: "A session run can end without a terminal state, leaving it stuck busy and uninterruptible"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-26"
---

# A session run can end without a terminal state, leaving it stuck busy and uninterruptible

## Summary

A session run can stop without ever writing a terminal record. When that happens the server keeps the
session `Busy` / `active` and the TUI keeps showing the turn "in progress" (reported as "stuck at
thinking forever"), no error is surfaced, and `Esc`/abort does not recover it.

This card originally proposed a per-command timeout. Investigation on 2026-09-26 disproved that
premise: no command is running and the bash tool already has a 2-minute bound. The real defect is
that the run has **no guaranteed terminal state** — matching a divergence from the reference
(`vanilla`) implementation, which always runs cleanup/halt and returns the session to idle.

## Reported behavior

- On `deepseek/deepseek-flash`, a turn stops making progress; the TUI shows it "in progress" (user
  described it as "stuck at thinking forever" / "unable to interrupt").
- No error is shown and the session never returns to idle.
- `Esc`/abort does not recover it; the user must abandon the session.
- In the captured case the user could not get the runtime to move on to the next conversation step.

## Evidence

Exported transcript: [`deepseek-flp-skill-hang-session.md`](../../docs/transcripts/deepseek-flp-skill-hang-session.md)

- Session ID: `ses_2c1168ee88f644f49315ca1736064d20` (2026-09-26).
- The export carries the in-progress warning: **1 tool call has no recorded result (bash)**.
- The same command fails immediately when run outside the product (the pyflp parse raises
  `pyflp.exceptions.NoModelsFound`), i.e. the command is not inherently slow.

## Investigation - 2026-09-26

Investigated directly against the live Rust product and its runtime state, not just the export.

### Runtime target

- Product: `$HOME/repos/opencode-modded-rust/target/debug/opencode` (built 2026-09-26 00:46).
- Live server `pid 99477` (`serve --port 3187`) + TUI `pid 99476` for session
  `ses_2c1168ee88f644f49315ca1736064d20`; the session was still open and stuck during investigation.
- Rust DB: `~/Library/Application Support/opencode/opencode.db`. Trace:
  `~/Library/Application Support/opencode/traces/tui.log`.

### What the runtime actually shows

- `GET /session/status` reports `{status: busy, idle: false, busy: true}` while the lifecycle status
  is `active`; `opencode session inspect <id>` says: *"Session is active; the last persisted message
  is a user prompt with no assistant reply."*
- The server's in-memory session has **12 messages**; the DB has only **7** (messages `[7]-[11]` are
  unpersisted). The last in-memory message `[11]` is an assistant message with
  `finish_reason = "tool-calls"` and **two `bash` tool calls, with no tool-result message**.
- The two tool calls in `[11]` (a bounded `pyflp` parse with `signal.alarm(45)` + `timeout: 60000`,
  and a `git status`/env probe) both finish in well under a second when run directly.
- **No child process is running** (`pgrep -P 99477` shows only the two long-lived plugin-host `bun`
  processes; no `bash`/`python`/`sh` anywhere).
- **No outbound provider socket** exists for the server (`lsof -a -p 99477 -i` shows only the
  localhost listener and the TUI's two localhost connections), so it is not awaiting a model response.
- **No pending permission or question** (`GET /permission` and `GET /question` both return `[]`), and
  no plugins/hooks are configured.
- A 5s `sample 99477` shows **every thread parked** (tokio workers on `__psynch_cvwait`, the process
  driver on `kevent`). No `opencode_session`/`SessionPrompt` frame is on any stack; the only live
  Rust work is TUI polling (`list_skills`).

### Failing stage

The bash tool already enforces a default 2-minute command timeout
(`crates/opencode-tool/src/bash.rs:13,214`) and observes the abort token (`bash.rs:217-226`), so the
command is not the wedge. The stall is one level up: **after the assistant step records an assistant
message whose `finish_reason` is `tool-calls`, the turn does not run the tools, appends no tool
results, and never reaches a terminal state.** The runner stays `Busy` and the lifecycle stays
`active` with no terminal record.

Because the drain runner is detached (`tokio::spawn(drain_session_queue(..))`,
`crates/opencode-server/src/routes.rs:2546-2552`) and only clears `Busy` after the turn returns
(`routes.rs:2565-2588`), any abnormal end of that task — a panic, or a permanent park on an await
that never resolves — leaves the session stuck `Busy` forever. `abort` cannot recover it either:
`abort_active_session_prompt` only cancels the token and rejects pending *questions*
(`routes.rs:4129-4158`); it does not force a terminal state, and the prompt loop only observes
cancellation at a few points (top of loop, stream select, bash select).

### Reference comparison (`vanilla`)

Checked the pinned reference commit `f54ce313b…` (branch `dev`,
`packages/opencode/src/session/processor.ts`):

- Vanilla has **no turn timeout** (the only timeout in the processor is a 250 ms drain wait during
  cleanup, `processor.ts:587`).
- Vanilla guarantees termination structurally: `Effect.onInterrupt` → `halt()` (sets
  `message.error`, publishes an error, `status.set(idle)`; `processor.ts:610-640`),
  `Effect.catch(halt)`, and `Effect.ensuring(cleanup())` which marks unfinished tool calls
  `"Tool execution aborted"`, sets `message.time.completed`, and persists (`processor.ts:553-608`).
- Vanilla's shell tool default timeout is `2*60*1000` ms and races abort vs timeout, killing the
  process tree (`packages/opencode/src/tool/shell.ts:347,533-564`) — already mirrored in the Rust
  bash tool.

So the parity gap is architectural: vanilla's Effect fibers propagate interruption into the awaited
work and always run cleanup to a terminal state; the Rust port's detached `tokio::spawn` drain only
clears `Busy` on normal return and lets abort miss anything outside the few explicit select points.

### Compounding gaps (split out into their own cards)

- Server stdout/stderr is discarded (`lsof -a -p 99477 -d 1,2` → `/dev/null`), so any panic/error is
  invisible and the exact failing mechanism could not be confirmed. See `BUG-045`.
- The 90s stream idle timeout (`BUG-038`) resets on every stream event, so a never-idle provider
  stream (e.g. endless reasoning) is unbounded. See `BUG-046`.
- Assistant progress is not persisted until the run completes, so a stalled run loses the whole turn
  and cannot be resumed. See `BUG-047`.

## Why this exists

The daily-driver workflow depends on a run always reaching a terminal state: finish, or fail loudly.
Today the Rust runtime has no such guarantee for a detached run, so a panic or a non-cancellable await
produces a session that is silently `busy` forever and cannot be interrupted. The reference
implementation guarantees this contract; aligning with it removes the silent wedge.

## Scope

- Guarantee every run ends in a terminal state: wrap the drain/runner so cleanup **always** runs
  (normal return, error, abort, and panic) and it resolves pending tool calls with durable error
  results, marks the assistant message complete/errored, persists it, and sets
  `SessionRunStatus::Idle`.
- Make `abort` actually interrupt the in-flight work: the cancel token must be observed by every long
  await on the run path (provider connect/stream, permission/question waits, tool execution, plugin
  hooks), or the run task must be cancelled directly — matching `Effect.onInterrupt` semantics.
- Contain a panicking run so the session still returns to idle with a durable error, rather than
  leaving `Busy` set.
- Treat the reference `processor.ts` cleanup/halt contract as the target behavior; a run-level
  watchdog is acceptable only as an implementation detail to achieve it, not as the product behavior.

## Non-goals

- Provider stream idle timeout / never-idle streams; that is `BUG-046`.
- Capturing server stderr/panics; that is `BUG-045`.
- Incremental persistence of in-flight turn progress; that is `BUG-047`.
- The continuation-point defect; that is `BUG-044`.
- Grep-specific runtime starvation; that is `BUG-040`.
- Changing `Esc` double-press semantics; owned by `BUG-019` / `BUG-029`.
- A new per-command timeout: bash already has a 2-minute default and it is not the gap.
- Fixing the FLP-pyflp parsing error itself; that is not a product bug.

## Done when

- A run that errors, panics, or is aborted always ends with a durable terminal record and the session
  returns to `idle` (`busy=false`).
- Aborting a stuck run unblocks the session within a bounded time.
- Unfinished tool calls are resolved to durable error results before the run ends.
- The terminal-state contract is covered by tests and documented with its vanilla reference.

## Recommended verification

- Test: a provider/tool that hangs or fails causes status to return to `idle` with a completed/errored
  assistant message.
- Test: aborting a run parked on a non-stream await terminates it and clears `busy`.
- Test: a panicking runner still clears `busy` and records an error.
- `cargo test -p opencode-session -p opencode-server`; `cargo check --workspace`.
- Live: reproduce the deepseek stall and confirm it no longer wedges; confirm `Esc` recovers.

## Related Items

- `BUG-045` Server runtime stderr/panics are invisible - diagnosability prerequisite.
- `BUG-046` A never-idle provider stream is unbounded - separate provider-level bound.
- `BUG-047` In-flight turn progress is not persisted until the run completes.
- `BUG-038` DeepSeek reasoning turn stalls mid-turn - provider stream idle timeout (source of the
  90s stream guard).
- `BUG-040` Grep tool blocks the async runtime and wedges the server and TUI - async starvation.
- `BUG-019` / `BUG-029` Escape interrupt behavior - the interrupt plumbing this card needs to use.
- `BUG-044` Continuing a session resumes from an earlier user prompt - companion continuation defect.
- `FEAT-035` Resume an interrupted session from where it left off - prior resume work.

## Notes

- Captured export: `docs/transcripts/deepseek-flp-skill-hang-session.md` (workspace
  `dump-musicproduction`).
- Relevant files: `crates/opencode-server/src/routes.rs` (`drain_session_queue`, `run_prompt_turn`,
  `abort_active_session_prompt`, `ACTIVE_PROMPTS`), `crates/opencode-session/src/prompt.rs`
  (loop, abort handling), and the tool implementations under `crates/opencode-tool/`.
- Reference: `packages/opencode/src/session/processor.ts` at `f54ce313b…` (`cleanup`, `halt`,
  `Effect.onInterrupt`, `Effect.ensuring`).
