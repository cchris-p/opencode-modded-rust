---
id: "H-004"
title: "GATE-001 session prompt queue and queued display parity - Handoff"
status: "closed"
created: "2026-09-21"
updated: "2026-09-22"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["GATE-001", "CLI-007", "CLI-008", "CLI-001", "CLI-006"]
---

# GATE-001 Session Prompt Queue and Queued Display Parity - Handoff

## Objective

Implement a per-session prompt queue and a queued-message display that satisfy
`GATE-001` exactly, then build the blocked CLI task stories on top of it. The queue must accept work
while a session is running, preserve admission order, materialize each accepted prompt exactly once,
and surface queued prompts to every client. The TUI must show the vanilla `QUEUED` badge.

## Binding Artifacts

- `GATE-001` (`boards/todo/gate-session-prompt-queuing-and-display-match-vanilla.md`) is the binding
  acceptance spec for the queue and its display.
- `invariants/message-queuing.md` documents the current behavior and target invariants.
- `invariants/cli-task-targeting.md` remains binding for the CLI stories.
- Frozen reference: `$HOME/repos/opencode-modded` at `e62912b5d18b73316c7bfd6e894b040698f6c880`.
  All vanilla line references below were verified at that commit.

## Included Board Items

- `GATE-001` Gate: prompt queuing and queue display must match vanilla OpenCode exactly (acceptance).
- `CLI-008` Queue CLI task sends while TUI session is open (core implementation).
- `CLI-001` Copy Cline-style CLI task send conventions (CLI task send/new/view on the queue).
- `CLI-006` Add CLI status visibility for tasks and background sessions (surfaces queued state).

## Excluded / Not Owned Here

- `CLI-007` is already merged and in `qa`; it is not blocked.
- `FEAT-007` (polling), `FEAT-036` (input-box draft recall), and `BUG-025` (concurrent DB sync loss) are
  out of scope.
- Mid-turn steer injection is resolved out of scope in `GATE-001` (reference-only).
- **CLI run-footer manage surface is conditional and currently unowned.** `GATE-001` requires it "where
  that surface exists"; the Rust CLI has no interactive run queue/footer today. Record it as a
  documented partial and open a follow-up card only when a CLI run surface exists.
- Do not bundle all items into one PR. The queue and the CLI command surfaces are large enough to
  deserve separate QA.

## Vanilla Reference Summary (full detail in GATE-001)

- Delivery enum `steer | queue`; admitted inputs carry `admitted_seq` and null `promoted_seq` until
  promoted (`packages/schema/src/session-delivery.ts:5`, `packages/core/src/session/input.ts:118-167`).
- FIFO promotion of one queued input at a time and batch steer promotion
  (`input.ts:245-285`); runner precedence and drain loop
  (`packages/core/src/session/runner/llm.ts:190-193,394-415`).
- Shipped clients effectively use `steer`: the app normalizes `followup: "queue"` back to `"steer"`
  (`packages/app/src/context/settings.tsx:354-355,373-378`) and the TUI sends no delivery.
- TUI queued boundary: `completed` = last completed assistant, `pending` = last in-flight assistant
  after it, queued = user message index `> pending`
  (`packages/tui/src/routes/session/index.tsx:244-250,1388`).
- TUI badge: agent-color background, contrast-selected foreground, bold, literal `QUEUED` with one
  leading/trailing space, replacing the timestamp row
  (`index.tsx:1388-1391,1438-1453`). Keybind `session_queued_prompts` = `<leader>q`
  (`packages/tui/src/config/keybind.ts:102`) has no TUI handler in the frozen tree.
- CLI run queue and footer: serial drain, `queue` count and `queued.prompts` list, "Manage queued
  prompts" only when queued exist, removal via `onQueuedRemove`
  (`packages/opencode/src/cli/cmd/run/runtime.queue.ts:73-84,280-330`,
  `footer.command.tsx:414-431`, `footer.view.tsx:552-590`, `types.ts:45-49,83-92`).

## Current Code Evidence

- `POST /session/{id}/prompt` spawns one run per request with no serialization
  (`crates/opencode-server/src/routes.rs:1747,2069-2073`).
- `ACTIVE_PROMPTS` is last-writer-wins (`routes.rs:1983-1986,2043-2050`).
- The `SessionPrompt` busy guard is per-instance and ineffective because a new runner is built per
  request (`routes.rs:1976-1982`, `crates/opencode-session/src/prompt.rs:137-149,178,734,817`).
- `prompt_async` appends messages and returns `queued` without executing (`routes.rs:2280-2313`).
- Run status is `{ Idle, Busy, Retry }` (`routes.rs:282-286`).
- TUI user rendering has no badge (`crates/opencode-tui/src/components/session_message.rs:15-74`);
  the message loop has the index and an active-assistant check via `finish.is_none()`
  (`crates/opencode-tui/src/components/session.rs:560-589,606-608`); the message model exposes `finish`
  and `completed_at` (`crates/opencode-tui/src/context/session_context.rs:21-35`).
- TUI leader key is `Ctrl+X` (`crates/opencode-tui/src/app/app.rs:435-437`); no queued-prompts command.
- Rust CLI has no interactive run queue/footer (`crates/opencode-cli/src/main.rs:81-82,1650`).

## Implementation Constraints (binding)

Reproduced from `GATE-001`; the implementation is wrong if any are violated.

1. Accept-time materialization with single ownership: append one user message with a stable ID on
   accept; the runner consumes that exact message; never create a second user message.
2. Shared per-session server state queue; exactly one runner per session; `ACTIVE_PROMPTS` is the
   active-run registry only and queued prompts never enter it.
3. Ordering by a monotonically increasing per-session acceptance sequence; strict FIFO.
4. Run status gains `Queued { position, depth }`; `busy` while draining, `idle` only when idle+empty.
5. `/session/{id}/prompt` returns `started` or `queued` (+ message ID, position, depth);
   `prompt_async` aliases the queued path and never claims `queued` without executing.
6. Abort cancels the active run only and leaves queued prompts visible; explicit cancel removes a
   waiting prompt and its materialized message.
7. Queued-but-unstarted messages survive restart and are not auto-executed; never silently deleted.
8. Optimistic clients reconcile by message ID; the server is the single source of truth.
9. Queued boundary derives from assistant completion; with no in-flight assistant, nothing is queued.
10. Concurrent-send regression test passes, plus materialization and abort tests.

## Recommended Implementation Sequence

### PR 1 - `CLI-008` Server queue + TUI queued display (satisfies GATE-001)

Branch: `feature/CLI-008-session-prompt-queue`

1. Add shared per-session queue state to `ServerState` (ordered pending prompts keyed by session; each
   entry carries the materialized user message ID plus resolved provider/model/agent context) and a
   per-session drain guard.
2. Change `session_prompt` to resolve, materialize the user message with a stable ID, enqueue, and
   either start the drain loop or return `queued` with position/depth; return `started` when it begins
   immediately.
3. Run exactly one runner per session from the shared state (consume the materialized message; do not
   re-add it); on completion start the next queued prompt, and only set `idle` when the queue is empty.
4. Repurpose `ACTIVE_PROMPTS` as the active-run registry; remove reliance on the per-instance busy
   guard.
5. Extend `SessionRunStatus` with `Queued { position, depth }`; expose it in `GET /session/status` and
   `session.status` events.
6. Implement abort (cancel active, keep queue) and an explicit cancel for a waiting prompt that removes
   its materialized message.
7. Make `prompt_async` an alias of the queued path and delete the message-append stub.
8. TUI: compute the queued set from `finish`/`completed_at` in `session.rs`; pass a `queued` flag into
   `render_user_message`; render the badge exactly (agent color bg, contrast fg, bold, ` QUEUED `,
   replacing the timestamp row, shown even when timestamps are hidden).
9. Tests: concurrent-send regression; single-user-message materialization; abort preserves queued
   prompts; badge rendering unit tests.

Verification gate:

- `GATE-001` Done-when checklist is fully satisfied (server, status, TUI badge, restart, tests).
- From the TUI, submit several prompts while one turn runs; confirm ordered execution and the badge on
  every queued user message, with no duplicate or lost messages.
- `cargo test -p opencode-server -p opencode-tui` (plus the new tests) and `cargo check` for touched
  crates.
- Side-by-side badge comparison against vanilla at the pinned commit.

### PR 2 - `CLI-001` CLI task send/new/view

Branch: `feature/CLI-001-cli-task-commands`

1. Add `opencode task new`, `send`, `view` using the explicit or selected (`CLI-007`) target.
2. Submit through the canonical queued prompt path; `task new` updates the current/default pointer only
   after session creation succeeds.
3. Return immediately by default with target session, message ID, and `started`/`queued` status;
   `--stream` follows the request once active and reports queued status while waiting.
4. `task view` reads conversation state without mutating the target.

Verification gate: the H-003/CLI-001 checklist (new/send/view, stdin text, explicit override, no
`AgentExecutor`, unchanged `ort` lifecycle).

### PR 3 - `CLI-006` CLI status visibility

Branch: `feature/CLI-006-cli-task-status`

1. Add compact list and single-session status commands reading `GET /session/status` and session list
   data; no separate status store.
2. Surface `idle | busy | queued (position/depth) | retry | error`; plain text by default, `--json`
   optional.
3. Keep status output separate from send/view semantics.

Verification gate: at least one busy session shown; completed sessions do not look active; queued state
reflects the `GATE-001` run status; `--json` parses.

## Ordering and Dependencies

- `CLI-008` (PR 1) must land first: it implements `GATE-001`, which blocks the other two.
- `CLI-001` (PR 2) depends on the queued prompt contract from PR 1.
- `CLI-006` (PR 3) depends on the queued run status from PR 1; it does not require PR 2.
- `GATE-001` itself is an acceptance artifact, not a code change; it is satisfied by PR 1.

## PR Workflow

- Create one branch and PR per phase, targeting `development`.
- Stage only files for the active board item; the worktree may contain another session's changes.
- Keep each item in `qa` after its PR is ready for local verification.
- Do not merge solely because tests pass; wait for explicit user merge direction.

## Documentation Updates

- Update `README.md` CLI overview when `task` commands land, and `docs/opencode-cli.md` for
  `task target/new/send/view` and status.
- Reference `GATE-001` and `invariants/message-queuing.md` from PR descriptions.

## Readiness Assessment

`GATE-001` and the three blocked stories are implementation-ready. Vanilla behavior is documented to
file:line at the pinned commit, the implementation-critical constraints (materialization ownership,
shared queue state, status shape, abort/cancel, restart, optimistic reconciliation) are explicit, and
the residual decisions (`--stream` while queued, status formats) are resolved. The one deliberate
partial is the CLI run-footer manage surface, which is conditional on a CLI run surface existing and is
called out rather than left ambiguous.

## Closeout (2026-09-21)

- `GATE-001` is implemented, user-QA confirmed, and marked `done`; it is now the primary and sole story
  for session prompt queuing and the queued-message display.
- `CLI-008`, `CLI-001`, and `CLI-006` were archived by user request and will not be implemented. PR 1
  was delivered by `GATE-001`; PR 2 and PR 3 are cancelled.
- This handoff is closed. No further work is planned from it.

## Reactivation Addendum (2026-09-22)

- The user reactivated `CLI-001` and `CLI-006` on 2026-09-22 as prerequisite gates for the remaining
  `CLI-*` stories. Only their cancellation is superseded; `CLI-008` remains archived (its queue behavior was
  delivered by `GATE-001`). This handoff otherwise stays closed.
- Canonical CLI/lifecycle behavior now lives in `wiki/cli-surface.md`.

## Folded Predecessor - H-003 (2026-09-22)

`H-003` (`handoffs/archive/2026-09-16-cli-task-targeting-handoff.md`) was folded into this handoff and moved to
`handoffs/archive/` because it covered the same board-item set (`CLI-001`, `CLI-006`, `CLI-008`) and was
already marked superseded by this document. Content merged in:

- `CLI-007` (merged, PR #37, branch `feature/CLI-007-task-target-selection`) shipped
  `opencode task target list|select|show|clear` with workspace-local `.opencode/task-target.json` storage
  and live validation of explicit target servers/sessions. It is the target input consumed by `CLI-001`
  and is the only part of `H-003` that was not superseded by this handoff.
- `H-003` verification for `CLI-007`: `cargo fmt`; `cargo check -p opencode-cli`; the `task target`
  subcommands including an unavailable-target check (`--server http://127.0.0.1:9`); and a live
  temporary-server smoke for list/select/show/clear.
- Intended CLI shape carried forward from `H-003`:

  ```sh
  ort task target list
  ort task target select
  ort task target show
  ort task target clear

  ort task new "Fix the failing provider test"
  ort task send "Now run the focused tests"
  ort task view
  ```

- Excluded then and still excluded here: `CLI-004` (detach) and `CLI-005` (same-workspace attach/reuse);
  first-class file/image attachments; interactive `task chat` (use `opencode attach <url>`).
- Ordering rationale retained: `CLI-007` first (target contract), queue (`GATE-001`) before `CLI-001` when
  `task send` must support TUI-open sessions, then `CLI-006` status visibility after.

## Archive Record - 2026-09-22

- Archived by user direction after audit.
- Handoff is explicitly closed; `GATE-001` is in `boards/done/` and was delivered by merged PR #61.
- Reactivated `CLI-001` and `CLI-006` work is owned by `H-006`
  (`handoffs/archive/2026-09-22-cli-task-surface-and-status-handoff.md`), so this handoff is historical only.
