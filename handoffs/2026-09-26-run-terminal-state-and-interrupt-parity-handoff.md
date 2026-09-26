---
id: "H-012"
title: "BUG-043..047 run terminal-state, interrupt, and persistence cluster - Handoff"
status: "in_progress"
created: "2026-09-26"
updated: "2026-09-26"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["BUG-043", "BUG-044", "BUG-045", "BUG-046", "BUG-047"]
---

# Run Terminal-State, Interrupt, And Persistence Cluster - Handoff

## Objective

Fix the class of session runs that stop without a terminal record, leaving the session stuck
`busy`/`active` and uninterruptible, and restore parity with the reference (`vanilla`) contract that
every run ends in a known terminal state. The cluster also removes the diagnosability and persistence
gaps that made the 2026-09-26 stall impossible to root-cause or resume.

The work is triggered by the live stall `ses_2c1168ee88f644f49315ca1736064d20`
(`deepseek/deepseek-flash`, workspace `dump-musicproduction`), exported to
`docs/transcripts/deepseek-flp-skill-hang-session.md`.

## Included Board Items

| Item | Title | Priority | Lane | Role in cluster |
|------|-------|----------|------|-----------------|
| `BUG-045` | Server runtime stderr and panics are discarded, hiding why a run failed | P1 | `todo` | Phase 0 diagnosis prerequisite |
| `BUG-043` | A session run can end without a terminal state, leaving it stuck busy and uninterruptible | P1 | `todo` | Core contract fix |
| `BUG-047` | Assistant turn progress is not persisted until the run completes | P1 | `todo` | Makes stalled runs inspectable/resumable; prerequisite for `BUG-044` |
| `BUG-046` | A provider stream that never goes idle is unbounded (endless reasoning) | P1 | `todo` | Provider-level bound (parallel-safe after Phase 0) |
| `BUG-044` | Continuing a session resumes from an earlier user prompt instead of the latest tool call | P1 | `todo` | Likely resolved by `BUG-047`; confirm, then fix any residual selection defect |

## Refinement Gate / Open Risk

The refinement gate passed for implementation, but one **factual** uncertainty is intentionally
carried into Phase 0 rather than resolved by assumption:

- `BUG-043`'s exact failing mechanism is unconfirmed: a panicking detached drain task, a permanently
  parked await, or an endless non-idle provider stream. The terminal-state/interrupt contract is
  implementable and testable independently of the cause, but **the specific cause may add follow-up
  work** (for example a panic site fix). Phase 0 exists to settle this before the core fix is
  considered complete.
- This is a diagnostic gate, not a planning ambiguity: ordering and dependencies are unambiguous.

## Dependency Order

Explicit dependencies (stated on the cards):

1. `BUG-045` before the core of `BUG-043` — durable server logs/panics are required to confirm the
   failing stage.
2. `BUG-047` before `BUG-044` — continuation resumes from the last **persisted** message; if the
   post-`Yes` turn was never persisted, `BUG-044`'s symptom is a consequence of the persistence gap.
   `BUG-044` explicitly records this as its likely root cause to confirm.
3. `BUG-046` is independent of the others (provider layer) and parallels `BUG-043`/`BUG-047`, but is
   sequenced after Phase 0 so diagnosis can tell an endless stream apart from a Rust loop defect.

Inferred ordering (rationale marked):

- `BUG-043` before `BUG-047` (inferred): persistence only matters once runs are guaranteed to reach
  a terminal state, and `BUG-043` may change where completes/errors are written. If implemented in
  the other order, `BUG-047` would persist a still-unbounded run; landing the terminal-state
  guarantee first keeps the persistence change bounded. They are small enough to share one PR.

Strict implementation sequence: **045 → 043 → 047 → 044 → 046**.

## Phase 0 - Diagnosability (BUG-045)

Deliverable: server stdout/stderr routed to a durable sink, plus a panic hook writing message,
location, and backtrace (including detached-task panics), with the sink discoverable per session.

Why first: without it, the only evidence for a wedged run is OS-thread sampling, which cannot see a
parked async task, and a panicking task writes nowhere. The 2026-09-26 investigation could not
distinguish panic from parked await specifically because of this.

Exit gate (must pass before Phase 1 is treated as complete):

- A forced server/detached-task panic leaves a durable record.
- A failing run leaves a durable, session-scoped error record.
- Reproduce the `deepseek/deepseek-flash` stall against the diagnostic build and classify it as
  **panic**, **parked await**, or **endless stream**; record the finding on `BUG-043` (and feed
  Phase 3 if it is the endless-stream case).

## Phase 1 - Guaranteed terminal state and real interruption (BUG-043)

Deliverable: every run ends in a terminal state and abort actually interrupts.

- Wrap the drain/runner so cleanup **always** runs on normal return, error, abort, and panic:
  resolve pending tool calls with durable error results, mark the assistant message
  complete/errored, persist it, and set `SessionRunStatus::Idle`.
- Make the cancel token observable by every long await on the run path (provider connect/stream,
  permission/question waits, tool execution, plugin hooks), or cancel the run task directly —
  matching `Effect.onInterrupt` semantics in the reference.
- Contain panics so `Busy` is always cleared and a durable error is recorded.
- Reference contract to mirror: `packages/opencode/src/session/processor.ts` at `f54ce313b…`
  (`cleanup`, `halt`, `Effect.onInterrupt`, `Effect.ensuring(cleanup())`).

Key files: `crates/opencode-server/src/routes.rs` (`drain_session_queue`, `run_prompt_turn`,
`abort_active_session_prompt`, `ACTIVE_PROMPTS`), `crates/opencode-session/src/prompt.rs`.

Verification gate:

- A provider/tool that hangs or fails returns status to `idle` with a completed/errored message.
- Aborting a run parked on a non-stream await terminates it and clears `busy`.
- A panicking runner still clears `busy` and records an error.

## Phase 2 - Incremental persistence (BUG-047)

Deliverable: assistant messages/parts and tool results are durable before the next provider request,
not only at turn end.

- Persist incrementally as parts are produced (tool calls, tool results, completed assistant steps),
  matching the reference `updatePart`/`updateMessage` behavior.
- Keep the turn-end flush as a final write.
- This is the enabler for `BUG-044`: continuation must be able to resume from the latest persisted
  part.

Key files: `crates/opencode-server/src/routes.rs` (`apply_session_snapshot`,
`persist_sessions_if_enabled`), `crates/opencode-session/src/session.rs`, `crates/opencode-storage/`.

Verification gate:

- After a turn stalls/interrupts after a tool result, the DB contains that tool result.
- The persisted session no longer ends at the last user prompt when a run was partially produced.

## Phase 3 - Provider bound and continuation (BUG-046, BUG-044)

`BUG-046` (parallel-safe with Phase 2 after Phase 0):

- Add a configurable single-step/turn budget (wall-clock and/or reasoning/completion token cap) on
  top of the existing silence-based idle timeout (`crates/opencode-provider/src/stream.rs:138-165`,
  which resets per event).
- On exceeding the budget, cancel the stream and surface a visible `StreamEvent::Error`; keep the
  existing idle timeout.
- Mandatory if Phase 0 classifies the stall as an endless stream; otherwise still a robustness fix.

`BUG-044` (after Phase 2):

- Re-test the captured "continue from `Yes`" case against the persisted-state fix.
- If continuation still selects a stale point, fix selection to use the latest persisted part
  (resolve/annotate an unresolved tool call rather than replaying an older user prompt).

Verification gate:

- A never-ending mock stream is stopped at the configured bound with an error event; a silently
  stalled stream still trips the idle timeout (no regression).
- Continuing the captured stall resumes from the latest persisted state, not the older user prompt.

## PR Plan

| PR | Items | Branch | Notes |
|----|-------|--------|-------|
| A | `BUG-045` | `bug/BUG-045-server-runtime-diagnostics` | Small, unblocks diagnosis. Target `development`. |
| B | `BUG-043`, `BUG-047` | `bug/BUG-043-047-run-terminal-state-persistence` | Cohesive run-lifecycle change. |
| C | `BUG-046`, `BUG-044` | `bug/BUG-046-044-provider-bound-and-continuation` | After B (044 depends on 047). |

`BUG-046` may be split into its own PR if it needs independent QA. Keep PRs one at a time in
dependency order; do not start C until B is merged into `development`.

## Board Updates Per PR

1. `BUG-045`: `todo -> doing -> qa` with Dev Notes, Verification, and the Phase 0 classification
   result.
2. `BUG-043`: `todo -> doing -> qa`; record the confirmed failing stage from Phase 0 and the
   terminal-state/interrupt tests.
3. `BUG-047`: `todo -> doing -> qa`; record the incremental-persistence tests.
4. `BUG-044`: confirm whether the persistence fix resolves it; if so record that and close, else
   implement and keep in `qa`.
5. `BUG-046`: `todo -> doing -> qa`; record the budget default and configuration surface.
6. Move this handoff to `handoffs/archive/` only when every covered PR is merged.

## Overall Verification

- `cargo test -p opencode-provider -p opencode-session -p opencode-server -p opencode-storage` and
  `cargo check --workspace`; `cargo fmt --all -- --check`.
- Live `ort-build`/`ort`: reproduce the `deepseek/deepseek-flash` stall and confirm the session
  returns to `idle` with a durable error and that `Esc` recovers it.
- Confirm no run can leave `GET /session/status` reporting `busy:true` indefinitely.

## Risks And Rollback

- **Phase 0 classification may reveal a cause outside the current scope** (for example a provider
  bridge bug). Surface it as a follow-up card rather than expanding this cluster silently.
- **Run watchdog value**: if a run-level bound is added, it is an implementation detail to achieve
  the terminal-state contract, not product behavior; make it configurable and conservative.
- **Incremental persistence write volume**: batch on part boundaries, not per token, to avoid DB
  write amplification.
- Each PR is independently revertible; `BUG-046` and `BUG-047` are isolated by subsystem.

## Deferred / Out Of Scope

- Rebuilding the continuation/queue architecture; only the selected resume point is in scope.
- DB schema or storage-engine changes beyond incremental writes.
- Reasoning-token display work (the `/thinking` surface).
- Provider transport rewrites beyond the step/turn bound.

## Execution Progress

### Pass A - BUG-045 (server diagnostics) - implemented, in `qa`

- Branch: `bug/BUG-045-server-runtime-diagnostics` (target `development`).
- Delivered: durable server log at `dirs::data_local_dir()/opencode/traces/server.log`
  (`OPENCODE_SERVER_LOG` override; `0`/`false`/`off`/empty disables), TUI spawn redirects server
  stdout/stderr to it instead of `/dev/null`, global panic hook appends message + location +
  backtrace, and `debug paths` / `session inspect` surface the path.
- Verification: `cargo test -p opencode-cli` 10 passed; `cargo check --workspace` clean;
  `cargo fmt --all` clean; live serve smoke test captured stderr to the sink.
- Remaining for Phase 0 exit gate: with the merged diagnostic build, reproduce the deepseek stall
  and classify it as panic / parked await / endless stream, then record the result on `BUG-043`.

### Pass B - BUG-043 + BUG-047 - not started

### Pass C - BUG-046 + BUG-044 - not started
