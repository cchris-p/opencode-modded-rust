---
id: "BUG-043"
title: "A session run can end without a terminal state, leaving it stuck busy and uninterruptible"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
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
- The two normal escape hatches are both unreliable (user-reported, 2026-09-26):
  - Interrupt fails while stuck, so the user resolves to exiting the TUI.
  - On reopening the session, it resumes from a point **before** the chunk that had not even been
    output yet — i.e. the displayed/output progress is ahead of what gets persisted and resumed.
  - Net effect: neither interrupt nor continue works as a recovery path.
- The model "self-correcting" on a later turn (for example wrapping commands in `timeout`) is **not**
  an acceptable fix: it depends on the model remembering to behave, which is not durable across
  sessions and does not address the runtime defect.

## Reproduction notes (user-reported, 2026-09-26)

- Emergent, not deterministic: it has not been tied to a specific command. It reappears only after
  starting a **fresh session** and letting the agent run for a while. A current session may complete
  or self-correct, so it cannot be reproduced on demand.
- Because it is not reliably reproducible, the fix must not depend on reproducing it by hand:
  verification uses a deterministic fault-injection harness (hang a tool/await, panic the detached
  run task, never-idle stream) that asserts the terminal-state and interrupt guarantees.
- Live recurrence is QA evidence, classified later: when it appears, export the transcript and use
  the Pass A server log (`…/traces/server.log`, `BUG-045`) to classify panic vs. parked await vs.
  endless stream.
- Captured recurrence to date: session `ses_2c1168ee88f644f49315ca1736064d20` (export
  `docs/transcripts/deepseek-flp-skill-hang-session.md`); user re-confirmed the interrupt→exit→resume
  loss pattern after rebuilding.

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
## Dev Notes - 2026-09-26

- `opencode-session` (`crates/opencode-session/src/prompt.rs`):
  - Refactored abort marking into `mark_finished` and added
    `pub fn finalize_incomplete_turn(session, finish_reason, error)` which marks the last assistant
    turn terminal (error + finish_reason, provider-valid content) and resolves any tool calls that
    never produced a result.
- `opencode-server` (`crates/opencode-server/src/routes.rs`):
  - `SessionQueue` gains a run-scoped `run_cancel` token. `drain_session_queue` now selects the run
    future against `run_cancel`, a hard run budget (`OPENCODE_RUN_TIMEOUT_MS`, default 30 min; `0`
    disables), and `catch_unwind`. On abort/timeout/panic it drops the run future (cancelling it at
    its await point) and calls `finalize_run_without_terminal`, which marks the turn terminal,
    resolves pending tool calls, clears `ACTIVE_PROMPTS`, broadcasts, and persists.
  - `abort_active_session_prompt` now also cancels `run_cancel`, so abort drops a run parked on an
    await that ignores the prompt token instead of leaving it wedged.
- This mirrors the reference `processor.ts` contract (`onInterrupt`/`catch(halt)`/`ensuring(cleanup)`)
  using drop-to-cancel plus a backstop budget; the budget is an implementation detail, not the
  product behavior.

## Verification - 2026-09-26

- New test `opencode-session`: `finalize_incomplete_turn_marks_terminal_and_resolves_calls` passes;
  existing abort/mark tests still pass.
- `cargo test -p opencode-session` -> 166 passed, 2 failed
  (`instruction::tests::{test_find_up_stops_at_stop_dir,test_find_up_walks_parents}`, pre-existing and
  unrelated; also noted in `BUG-016`).
- `cargo test -p opencode-server` -> 64 + 3 integration passed, 0 failed.
- `cargo check --workspace` clean; `cargo fmt --all` clean.
- Not run here: live `ort` reproduction (emergent; see Reproduction notes). Fault-injection coverage
  for the drain paths is at the finalize level plus existing server tests; the live recurrence is QA
  evidence captured with the `BUG-045` server log.
## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/118
  (branch `bug/BUG-043-047-run-terminal-state-persistence`, base `development`, handoff H-012 Pass B).

## Classification - 2026-09-26 (Phase 0 resolved)

The recurrence was a **panic**, not a parked await or endless stream. The `BUG-045` hook captured it
in `…/traces/server.log`:

- `[PANIC] range end index 2 out of range for slice of length 1` at
  `crates/opencode-permission/src/arity.rs:10:30`, unwinding through
  `BashArity::prefix` → `BashTool::execute` → `execute_tool_calls` → `drain_session_queue`.
- Root cause filed and fixed as `BUG-048` (arity slice not clamped to token count).
- On the build that produced this (pre-`BUG-043`), the panicking run never finalized, leaving the
  session `busy` - exactly this card's mechanism. With `BUG-043`, the same panic is contained and the
  run finalizes; `BUG-048` removes the panic entirely.

## QA Report (self-QA) - 2026-09-26 - FAIL

QA: BUG-043 — guaranteed terminal state and interrupt on abort.

- commit: `34bc972` (`development`); binary `target/debug/opencode`; model `deepseek/deepseek-flash`;
  headless server + HTTP API (ports 3598/3599).
- Tests (PASS): `cargo test -p opencode-session finalize_incomplete_turn_marks_terminal_and_resolves_calls`;
  `cargo test -p opencode-server fingerprint_changes_when`.
- Live abort (PASS): stream a long turn, `POST /session/{id}/prompt/abort`, poll `GET /session/status`
  -> `busy=false`/`idle=true` within ~4s. The stuck-Busy wedge is gone.
- Live durable terminal record (FAIL, nondeterministic). Across 4 abort runs:
  - 2/4: after idle, both `/session/{id}/message` (in-memory) and the DB had no terminal record — the
    last assistant message was a reasoning part only, `error`/`finish` absent, with no
    `Run ended before completion: aborted` / `Aborted by user.` text.
  - 1/4: the DB transiently held `Run ended before completion: aborted`, then a follow-up
    continuation overwrote it back to reasoning-only.
  - 1/4: terminal text persisted and stabilized in the DB, but the API still reported `error`/`finish`
    as null.
- Likely mechanism: `run_prompt_turn` spawns the `BUG-047` `update_task`, which keeps applying
  buffered `Session` snapshots to `state.sessions` and persisting them. On abort, `drain_session_queue`
  drops the run future and `finalize_run_without_terminal` writes the terminal state; the still-draining
  `update_task` then re-applies a stale pre-finalize snapshot (`merge_session_snapshot` does
  `*existing = snapshot`) and persists it, clobbering the terminal record. The `time.updated` freshness
  guard in `sync_sessions_to_storage` does not reliably prevent this because the stale snapshot can
  carry a newer update time.
- Deterministic storage gap (contributes): message metadata (`error`, `finish_reason`, `completed_at`)
  is never written to storage — `MessageRepository::upsert` / `replace_for_session` serialize only
  `message.parts` (`crates/opencode-storage/src/repository.rs:592,621,811`) and `list_for_session`
  rebuilds messages with `metadata: HashMap::new()` (`repository.rs:701`). So no terminal record can
  survive a reload/restart, and `opencode session inspect` mislabels an aborted-but-finalized turn as
  "stalled" (its heuristic only recognizes ToolResult/StepFinish/Compaction parts).
- result: **FAIL**. Idle is guaranteed, but "always ends with a durable terminal record" is not met.
  Reopened for implementation.

## Live Context - 2026-09-26 (shared observation)

A live two-session capture (see `BUG-051`) found the observed `deepseek/deepseek-flash` runs were
**slowly progressing, not hard-wedged** (persisted assistant `data` grew monotonically, message
count advanced). It also directly observed the mechanism this card's QA Report hypothesizes: the
in-memory `state.sessions[...]` list alternated between a forward run snapshot and a reverted
persisted snapshot. Dated context only; this card's `qa` status and scope are unchanged.

## Dev Notes - 2026-09-27 (reopened root-cause fix, H-014)

- Confirmed root cause: `merge_session_snapshot` did `*existing = snapshot`
  (`crates/opencode-server/src/routes.rs`), so the still-draining `update_task` re-applied a stale
  pre-finalize run snapshot over the finalized shared session and `persist_sessions_if_enabled` wrote
  it back out, erasing the terminal record.
- Fix: `merge_session_snapshot` is now monotonic and keyed by message id. It never reduces the message
  count, keeps a newer terminal record, advances streamed parts without regressing them, preserves
  ordering, and does not resurrect intentionally removed messages. `Session::remove_message` now
  records a durable tombstone (`deleted_message_ids`) that the merge honors.
- Storage now persists and restores message metadata (`error` / `finish_reason` / `completed_at`):
  `messages.data` stores parts plus metadata and reads accept the legacy bare-array form
  (`crates/opencode-storage/src/repository.rs`). `finalize_incomplete_turn` also stamps `completed_at`.
- `opencode session inspect` now recognizes a persisted terminal record instead of reporting a
  finalized-without-terminal run as stalled (`crates/opencode-cli/src/main.rs`).

## Verification - 2026-09-27

- New tests: `merge_session_snapshot_preserves_terminal_record`, `..._never_regresses_message_count`,
  `..._appends_new_run_progress`, `..._advances_streamed_text_without_regressing`,
  `..._does_not_resurrect_deleted_message`; `message_metadata_round_trips`,
  `legacy_parts_only_data_loads_with_empty_metadata`; `diagnostic_verdict_recognizes_persisted_terminal_metadata`.
- `cargo test -p opencode-server` -> 71 + 3 integration passed; `cargo test -p opencode-storage` -> 3
  passed; `cargo check --workspace` and `cargo fmt --all -- --check` clean.
- Live (isolated HOME, no shared DB): abort a streaming run -> idle; reload from storage -> assistant
  message carries `error=aborted`, `finish_reason=aborted`, `completed_at` set; inspect no longer
  reports it as stalled.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/127
  (branch `bug/BUG-043-047-051-snapshot-merge-and-metadata`, base `development`, handoff H-014).

## Closeout - 2026-09-27

- Merged into `development` as PR #127 (merge commit `dc41ce3`); branch
  `bug/BUG-043-047-051-snapshot-merge-and-metadata` deleted (remote then local).
- Remains in `qa` until a post-merge QA report is recorded on `development`.

## QA Report (post-merge self-QA) - 2026-09-27 - PASS

QA: BUG-043 — an aborted run keeps a durable terminal record across a reload on `development`.
commit: `7377827` (fix `875e5e7`, merge `dc41ce3`)   binary: `target/debug/opencode`
harness: `scripts/qa/bug-043-047-051-snapshot-merge-qa.sh` (isolated HOME/DB, server 127.0.0.1:4197)

- `cargo test -p opencode-server` 71 + 3 passed; `-p opencode-storage` 3 passed; `-p opencode-tui`
  `continuation_prompt_renders_once_in_order_and_stays_put` passed; `cargo check --workspace` and
  `cargo fmt --all -- --check` clean. (`opencode-session` has 2 pre-existing
  `instruction::tests::test_find_up_*` failures from `e937c3c`; `instruction.rs` is untouched by
  this fix — unrelated macOS path-symlink issue.)
- Live (isolated HOME): streaming turn -> abort -> idle -> restart server -> `GET /session/{id}/message`
  returns the aborted assistant with `error=aborted`, `finish=aborted`, `completed_at` set.
- Raw storage: `sqlite3 messages.data` for the aborted assistant contains
  `{"metadata":{"completed_at":...,"error":"aborted","finish_reason":"aborted"},...}` — metadata
  survives reload (not `HashMap::new()`).
- `opencode session inspect <id>` reports persisted output and does not say `stalled`.
- result: **PASS**.